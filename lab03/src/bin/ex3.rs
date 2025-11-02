#![no_std]
#![no_main]

use defmt::{Format, debug, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{OutputType, Pull},
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

/// Stores the current LED color
///
/// #[derive(Format)] is required to be abel to
/// print this using the `defmt` formatter ({})
#[derive(Format)]
enum LedColor {
    Red,
    Yellow,
    Green,
}

/// Implements functions for the
/// LED color management
impl LedColor {
    /// Get the next LED color based on the
    /// current color (`&self`)
    fn next(&self) -> LedColor {
        match self {
            LedColor::Red => LedColor::Yellow,
            LedColor::Yellow => LedColor::Green,
            LedColor::Green => LedColor::Red,
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The RGB LED is connected to:
    // - RED on pin D3 (PB3)
    // - GREEN on pin D5 (PB4)
    // - BLUE on pin D6 (PB10)

    // D3 (PB3) can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let red_pwm_pin: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // D4 (PB4) can be connected for PWM to Channel 1 of TIM 3
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB4.
    let green_pwm_pin: PwmPin<'_, TIM3, Ch1> = PwmPin::new(peripherals.PB4, OutputType::PushPull);

    // D6 (PB10) can be connected for PWM to Channel 2 of TIM 3
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB10.
    let blue_pwm_pin: PwmPin<'_, TIM2, Ch3> = PwmPin::new(peripherals.PB10, OutputType::PushPull);

    // Enable PWM for TIM2
    // only Channels 2 and 3 will be used
    // and connected to pin PB3 and PB10
    let pwm2 = SimplePwm::new(
        peripherals.TIM2,   // Timer 2 peripheral
        None,               // Channel 1 not used
        Some(red_pwm_pin),  // Channel 2 output (PB3)
        Some(blue_pwm_pin), // Channel 3 output (PB10)
        None,               // Channel 4 not used
        khz(10),            // PWM frequency = 1 kHz
        Default::default(), // Default configuration
    );

    // Enable PWM for TIM3
    // only Channel 1 will be used and connected to pin PB4
    let mut pwm3 = SimplePwm::new(
        peripherals.TIM3,    // Timer 3 peripheral
        Some(green_pwm_pin), // Channel 1 output (PB4)
        None,                // Channel 2 not used
        None,                // Channel 3 not used
        None,                // Channel 4 not used
        khz(10),             // PWM frequency = 10 kHz
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
    let pwm2_channels = pwm2.split();

    // Get a mutable reference to channel 2 of TIM 2 to control it
    let mut red_ch = pwm2_channels.ch2;
    // Get a reference to channel 1 of TIM 3 to control it
    let mut green_ch = pwm3.ch1();

    // Get a mutable reference to channel 3 of TIM 2 to control it
    let mut blue_ch = pwm2_channels.ch3;

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

    // The button is connected to D7 (PA8)
    //
    // The buttons on the lab board have an external pull up resistor (soldered
    // on the lab board), so the internal pull resistor is not needed.
    // Pull Up means that:
    //    - the pin's value is HIGH when the button is release
    //    - the pin's value is LOW when the button is pressed
    // We can either use `Pull::None` or `Pull::Up` (not recommended),
    // we cannot use `Pull::Down`.
    let mut button = ExtiInput::new(peripherals.PA8, peripherals.EXTI8, Pull::None);

    // Store the current LED color
    let mut color = LedColor::Red;

    loop {
        // Display the LED color
        debug!("LED color is {}", color);

        // Light up the RGB LED based on the current color
        match color {
            LedColor::Red => {
                // Set RED to 100%
                red_ch.set_duty_cycle_percent(100);
                // Set GREEN to 0%
                green_ch.set_duty_cycle_percent(0);
                // Set BLUE to 0%
                blue_ch.set_duty_cycle_percent(0);
            }
            LedColor::Yellow => {
                // Set RED to 100%
                red_ch.set_duty_cycle_percent(100);
                // Set GREEN to 30%
                //
                // Ideally it should be 100%, but the LED on the lab board
                // shows yellow-ish color when using 100% of RED and 30%
                // of green.
                green_ch.set_duty_cycle_percent(30);
                // Set BLUE to 0%
                blue_ch.set_duty_cycle_percent(0);
            }
            LedColor::Green => {
                // Set RED to 0%
                red_ch.set_duty_cycle_percent(0);
                // Set GREEN to 100%
                green_ch.set_duty_cycle_percent(100);
                // Set BLUE to 0%
                blue_ch.set_duty_cycle_percent(0);
            }
        }

        // NOTE: As the blue PWM channel is always 0% in the exercise,
        //       instead of using PWM for BLUE, we could connect
        //       the BLUE pin of the LED to VCC

        // We do nothing (actually sleep very a few milliseconds) while
        // the button is not pressed.
        button.wait_for_falling_edge().await;

        // Compute the next LED color
        color = color.next();

        // Debouncing
        //
        // It is a good idea to sleep a few milliseconds to debounce the
        // button.
        //
        // Due to the mechanical construction of the button, when a button is
        // pressed, the value of the pin oscillates between HIGH and LOW. Waiting
        // for a few milliseconds allows the signal to stabilize. Unless we do
        // this, the button will appear to be pressed several times.
        //
        // The number of milliseconds can be adjusted.
        Timer::after_millis(500).await;
    }
}
