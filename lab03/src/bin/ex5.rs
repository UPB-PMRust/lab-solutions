#![no_std]
#![no_main]

use defmt::{debug, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::OutputType,
    peripherals::TIM2,
    time::hz,
    timer::{
        Ch2,
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_time::Timer;
use panic_probe as _;

/// Computes the duty cycle per_mille (to the 1000) for
/// a given angle.
fn servo_duty_cycle_per_mille_for_angle(angle: u8) -> u16 {
    const MIN_PULSE_US: u32 = 500; // 0 degrees is 0.5 ms
    const MAX_PULSE_US: u32 = 2500; // 0 degrees is 2.5 ms
    const PERIOD_US: u32 = 20000; // 50 Hz period

    // 180 degrees ......... 2500 us
    // 0 degrees ........... 500 us
    // `angle` degrees ..... n us
    //
    // n = angle x 180 degree pulse length (ie 2500 - 500) / 180 + 500
    let pulse = angle as u32 * (MAX_PULSE_US - MIN_PULSE_US) / 180 + MIN_PULSE_US;

    // 100% duty cycle ...... 20000 us (20 ms, PWM 50 Hz)
    // n duty cycle ......... pulse us
    //
    // Due to accuracy, we compute the per mille (to the 1000)
    // We compute using `u32` due to accuracy and transform the
    // value to `u16`.
    let period_per_mille = ((pulse * 1000) / PERIOD_US) as u16;

    // Display the pulse length and the pulse per mille (to the 1000)
    debug!("Pulse length {} / {}‰", pulse, period_per_mille);

    // Return the `period_per_mille`
    period_per_mille
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The servo is connected on pin D3 (PB3)

    // PB3 can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let servo_pwm_pin: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // Enable PWM for TIM2
    // only Channel 2 will be used and connected to pin PB3
    let mut pwm = SimplePwm::new(
        peripherals.TIM2,    // Timer 2 peripheral
        None,                // Channel 1 not used
        Some(servo_pwm_pin), // Channel 2 output (PB3)
        None,                // Channel 3 not used
        None,                // Channel 4 not used
        hz(50),              // the servo needs a PWM frequency of 50 Hz
        Default::default(),  // Default configuration
    );

    // Get a mutable reference to channel 2 of TIM 2 to control it
    let mut servo = pwm.ch2();

    // Start PWM on the channel
    servo.enable();

    // The PWM polarity configures what the duty cycle means:
    // - ActiveHigh (default) -> the amount of time the PWM signal is HIGH
    // - ActiveLow -> the amount of time the PWM signal is LOW
    //
    // The servo reads the amount of time the signal is HIGH.
    servo.set_polarity(OutputPolarity::ActiveHigh);

    loop {
        // Take every angle from 0 to 180
        for angle in 0..=180 {
            // Display the angle
            debug!("Angle {}", angle);

            // Calculate the duty cycle per mille
            let servo_per_mille = servo_duty_cycle_per_mille_for_angle(angle);

            // Set the duty cycle fraction per mille
            servo.set_duty_cycle_fraction(servo_per_mille, 1000);

            // We have to allow the servo to move.
            // This may be adjusted accordingly.
            Timer::after_millis(10).await;
        }

        // Take every angle from 1 to 179 in reverse order
        for angle in (1..=179).rev() {
            // Display the angle
            debug!("Angle {}", angle);
            let servo_per_mille = servo_duty_cycle_per_mille_for_angle(angle);

            // Set the duty cycle fraction duty_cycle_period per mille
            servo.set_duty_cycle_fraction(servo_per_mille, 1000);

            // We have to allow the servo to move.
            // This may be adjusted accordingly.
            Timer::after_millis(10).await;
        }
    }
}
