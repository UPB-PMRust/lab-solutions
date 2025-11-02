#![no_std]
#![no_main]

use defmt::{debug, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::OutputType,
    peripherals::TIM2,
    time::khz,
    timer::{
        Ch2,
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LED is connected on pin D3 (PB3)
    //
    // PB3 can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let led_pwm_pin: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // Enable PWM for TIM2
    // only Channel 2 will be used and connected to pin PB3
    let mut pwm = SimplePwm::new(
        peripherals.TIM2,   // Timer 2 peripheral
        None,               // Channel 1 not used
        Some(led_pwm_pin),  // Channel 2 output (PB3)
        None,               // Channel 3 not used
        None,               // Channel 4 not used
        khz(10),            // PWM frequency = 10 kHz
        Default::default(), // Default configuration
    );

    // Get a reference to channel 2 of TIM 2 to control it
    let mut ch2 = pwm.ch2();

    // Start PWM on the channel
    ch2.enable();

    // The PWM polarity configures what the duty cycle means:
    // - ActiveHigh (default) -> the amount of time the PWM signal is HIGH
    // - ActiveLow -> the amount of time the PWM signal is LOW
    //
    // The LEDs on the lab board are active LOW: they light up when the PWM signal is LOW
    // and turn off when the PWM signal is HIGH. We set the polarity to LOW so that
    // the LED turns on during the PWM's duty cycle period.
    ch2.set_polarity(OutputPolarity::ActiveLow);

    // Set the duty cycle of the channel
    ch2.set_duty_cycle_percent(25);

    // Iterate through numbers 0 to 10
    //
    // The LED turns off when the PWM signal is high, starting
    // with the LED off means generating a signal with
    // 100% duty cycle (always HIGH).
    for intensity in 0..=10 {
        // Display the LED's intensity
        debug!("LED intensity {}%", intensity * 10);

        // Set the duty cycle in percentage
        // - for intensity 0 => 0% (always on)
        // - for intensity 1 => 10% (10% on)
        // ...
        // - for intensity 9 => 90% (90% on)
        // - for intensity 10 => 100% (always off)
        ch2.set_duty_cycle_percent(intensity * 10);

        // Wait 1 second
        // Make sure you do not forget the `await`
        // as the timer will do nothing
        Timer::after_secs(1).await;
    }
}
