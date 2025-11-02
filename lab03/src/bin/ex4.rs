#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    adc::{self, Adc, Averaging, Resolution, SampleTime},
    gpio::OutputType,
    peripherals::{TIM2, TIM3},
    time::khz,
    timer::{
        Ch1, Ch2, Ch3,
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

    // The RGB LED is connected to:
    // - RED on pin D3 (PB3)
    // - GREEN on pin D5 (PB4)
    // - BLUE on pin D6 (PB10)

    // PB3 can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let red_pwm_pin: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // PB4 can be connected for PWM to Channel 1 of TIM 3
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB4.
    let green_pwm_pin: PwmPin<'_, TIM3, Ch1> = PwmPin::new(peripherals.PB4, OutputType::PushPull);

    // PB10 can be connected for PWM to Channel 2 of TIM 3
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB10.
    let blue_pwm_pin: PwmPin<'_, TIM2, Ch3> = PwmPin::new(peripherals.PB10, OutputType::PushPull);

    // Enable PWM for TIM2
    // only Channels 2 and 3 will be used
    // and connected to pin PB3 and PB10
    let pwm1 = SimplePwm::new(
        peripherals.TIM2,   // Timer 2 peripheral
        None,               // Channel 1 not used
        Some(red_pwm_pin),  // Channel 2 output (PB3)
        Some(blue_pwm_pin), // Channel 3 output (PB10)
        None,               // Channel 4 not used
        khz(1),             // PWM frequency = 1 kHz
        Default::default(), // Default configuration
    );

    // Enable PWM for TIM3
    // only Channel 1 will be used and connected to pin PB4
    let mut pwm2 = SimplePwm::new(
        peripherals.TIM3,    // Timer 2 peripheral
        Some(green_pwm_pin), // Channel 1 output (PB4)
        None,                // Channel 2 not used
        None,                // Channel 3 not used
        None,                // Channel 4 not used
        khz(1),              // PWM frequency = 1 kHz
        Default::default(),  // Default configuration
    );

    // Split the PWM1 channels
    //
    // Usually we have `let mut pwm_channel = pwm.chX()` where X
    // represents one of the channel numbers.
    //
    // The `pwm_channel` variable has to be mutable, which means that
    // `pwm` is borrowed mutable (`&mut pwm`). The borrow (reference)
    // is valid until the `pwm_channel` variable goes out of
    // scope (after it is not used anymore). Rust forbids multiple
    // mutable borrows, meaning that writing
    // `let mut pwm_channel_2 = pwm.chX()` again will fail until
    // the `pwm_channel` variable is still used.
    //
    // To overcome this, the PWM peripheral provides a function called
    // `split` that allows us to receive all the 4 channels with one
    // single borrow.
    let pwm1_channels = pwm1.split();

    // Get a mutable reference to channel 2 of TIM 2 to control it
    let mut red_ch = pwm1_channels.ch2;
    // Get a reference to channel 2 of TIM 3 to control it
    let mut green_ch = pwm2.ch2();

    // Get a mutable reference to channel 3 of TIM 2 to control it
    let mut blue_ch = pwm1_channels.ch3;

    // Start PWM on the channels
    red_ch.enable();
    green_ch.enable();
    blue_ch.enable();

    // The PWM polarity configures what the duty cycle means:
    // - ActiveHigh (default) -> the amount of time the PWM signal is HIGH
    // - ActiveLow -> the amount of time the PWM signal is LOW
    //
    // The RGB LED pins on the lab board are active LOW: they light up when the PWM signal is LOW
    // and turn off when the PWM signal is HIGH. We set the polarity to LOW so that
    // the LED turns on during the PWM's duty cycle period.
    red_ch.set_polarity(OutputPolarity::ActiveLow);
    green_ch.set_polarity(OutputPolarity::ActiveLow);
    blue_ch.set_polarity(OutputPolarity::ActiveLow);

    // The light sensor is connected to A0 (PA0)
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
        let light_value = adc1.blocking_read(&mut peripherals.PA0);

        // Display the light's intensity
        info!("Light value {}", light_value);

        // Convert the sampled value in percentage:
        //
        // - 0 means 0%
        // - 16383 means 100%
        //
        // We convert the `potentiometer_value` to `u32` as multiplying it
        // with 100 will overflow a u16.
        // We convert the final value to a `u8` as it is a percentage and we
        // are sure that it fits.
        let percentage = (light_value as u32 * 100 / MAX_VALUE) as u8;

        // If the light intensity is low light up the LED with RED
        if percentage < 33 {
            // Set RED to 100%
            red_ch.set_duty_cycle_percent(100);
            // Set GREEN to 0%
            green_ch.set_duty_cycle_percent(0);
            // Set BLUE to 0%
            blue_ch.set_duty_cycle_percent(0);
        }
        // If the light intensity is medium light up the LED with RED
        else if percentage < 66 {
            // Set RED to 100%
            red_ch.set_duty_cycle_percent(0);
            // Set GREEN to 0%
            green_ch.set_duty_cycle_percent(100);
            // Set BLUE to 0%
            blue_ch.set_duty_cycle_percent(0);
        }
        // If the light intensity is high light up the LED with RED
        else {
            // Set RED to 100%
            red_ch.set_duty_cycle_percent(0);
            // Set GREEN to 0%
            green_ch.set_duty_cycle_percent(0);
            // Set BLUE to 0%
            blue_ch.set_duty_cycle_percent(100);
        }

        // It is a good idea to place a delay inside this loop,
        // otherwise the MCU will execute loop iterations
        // tightly and might heat up.
        //
        // Placing a delay makes the MCU wait for a few milliseconds
        // in between loops.
        Timer::after_millis(10).await;
    }
}
