#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    adc::{self, Adc, Averaging, Resolution, SampleTime},
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
    let mut peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LED is connected on pin D3 (PB3)

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
        khz(1),             // PWM frequency = 1 kHz
        Default::default(), // Default configuration
    );

    // Get a mutable reference to channel 2 of TIM 2 to control it
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

    // The potentiometer is connected to A0 (PA0)
    //
    // Pin PA0 can be connected ADC1's Channel 5
    let mut adc1 = Adc::new(peripherals.ADC1);

    // Set the resolution of ADC1 to 14 bits
    adc1.set_resolution(Resolution::BITS14);

    // Set ADC1 to read 1024 samples for every read request
    // and return the average of the sampled values
    adc1.set_averaging(Averaging::Samples1024);
    adc1.set_sample_time(SampleTime::CYCLES160_5);

    // Get the maximum value of a sample on 14 bits
    const MAX_VALUE: u32 = adc::resolution_to_max_count(Resolution::BITS14);

    loop {
        // Read channel 5 (pin PA0)
        let potentiometer_value = adc1.blocking_read(&mut peripherals.PA0);

        // Convert the sampled value in percentage:
        //
        // - 0 means 0%
        // - 16383 means 100%
        //
        // We convert the `potentiometer_value` to `u32` as multiplying it
        // with 100 will overflow a u16.
        // We convert the final value to a `u8` as it is a percentage and we
        // are sure that it fits.
        let percentage = (potentiometer_value as u32 * 100 / MAX_VALUE) as u8;

        // Set the duty cycle to control the LED's intensity
        ch2.set_duty_cycle_percent(percentage);

        // It is a good idea to place a delay inside this loop,
        // otherwise the MCU will execute loop iterations
        // tightly and might heat up.
        //
        // Placing a delay makes the MCU wait for a few milliseconds
        // in between loops.
        Timer::after_millis(10).await;
    }
}
