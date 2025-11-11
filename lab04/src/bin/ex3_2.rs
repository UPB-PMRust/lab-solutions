#![no_std]
#![no_main]

use core::cmp::min;

use async_debounce::Debouncer;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{OutputType, Pull},
    peripherals::TIM2,
    time::khz,
    timer::{
        Ch2,
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use panic_probe as _;

/// The period in which a button's value has to stay stable
/// to be considered pressed or released.
///
/// Due to their mechanical construction, when pressed or released,
/// buttons generate voltage fluctuations that the GPIO pin might
/// register as several pressed and releases. To avoid this,
/// we have to debounce the signal. The general idea is:
/// - in a loop
///     - wait for the rising or falling edge
///     - wait for an amount of time
///     - if the value is still correct (HIGH or LOW) return
///     - if the value changed, it means it was a transitory
///       signal, go back and wait for another edge
/// ```
/// async fn debounce_wait_for_falling_edge (pin: ExtiInput<'static>, stable_for: Duration) {
///     loop {
///         pin.wait_for_falling_edge().await;
///         Timer::after(duration).await;
///         if pin.is_low() {
///             break;
///         }
///     }
/// }
/// ```
///
const DEBOUNCE_STABLE_PERIOD: Duration = Duration::from_millis(100);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LED is connected on pin D3 (PB3)
    //
    // PB3 can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let led_red: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // The buttons on the lab board have an external pull up resistor (soldered
    // on the lab board), so the internal pull resistor is not needed.
    // Pull Up means that:
    //    - the pin's value is HIGH when the button is release
    //    - the pin's value is LOW when the button is pressed
    // We can either use `Pull::None` or `Pull::Up` (not recommended),
    // we cannot use `Pull::Down`.
    //
    // Buttons have to be debounced to prevent the tasks from reading several
    // button presses due to electrical noise generated when the button is pressed.
    // `Debouncer` takes a GPIO Input and debounces the signal. It exposes similar
    // functions with `ExitInput`.
    let mut button_s1 = Debouncer::new(
        ExtiInput::new(peripherals.PA8, peripherals.EXTI8, Pull::None),
        DEBOUNCE_STABLE_PERIOD,
    );
    let mut button_s2 = Debouncer::new(
        ExtiInput::new(peripherals.PC7, peripherals.EXTI7, Pull::None),
        DEBOUNCE_STABLE_PERIOD,
    );

    // Enable PWM for TIM2
    // only Channel 2 will be used and connected to pin PB3
    let mut pwm = SimplePwm::new(
        peripherals.TIM2,   // Timer 2 peripheral
        None,               // Channel 1 not used
        Some(led_red),      // Channel 2 output (PB3)
        None,               // Channel 3 not used
        None,               // Channel 4 not used
        khz(1),             // PWM frequency = 1 kHz
        Default::default(), // Default configuration
    );

    // Get a reference to channel 2 of TIM 2 to control it
    let mut led = pwm.ch2();

    // Start PWM on the channel
    led.enable();

    // The PWM polarity configures what the duty cycle means:
    // - ActiveHigh (default) -> the amount of time the PWM signal is HIGH
    // - ActiveLow -> the amount of time the PWM signal is LOW
    //
    // The LEDs on the lab board are active LOW: they light up when the PWM signal is LOW
    // and turn off when the PWM signal is HIGH. We set the polarity to LOW so that
    // the LED turns on during the PWM's duty cycle period.
    led.set_polarity(OutputPolarity::ActiveLow);

    // The initial LED's intensity percent
    let mut led_intensity = 0u8;

    loop {
        // Set the intensity by modifying the duty cycle
        led.set_duty_cycle_percent(led_intensity);

        // Wait for one of the two buttons to be pressed.
        //
        // `select` receives two Futures as parameters and waits
        // for one of them to finish. When a Future finishes, the
        // other Future is dropped and `select` returns.
        //
        // NOTE: The `wait_for_falling_edge` functions are called
        //       without an `.await` as `select` requires the
        //       Futures, not the Futures' result.
        //       The `.await` is used for the `select` function.
        let button = select(
            button_s1.wait_for_falling_edge(),
            button_s2.wait_for_falling_edge(),
        )
        .await;

        match button {
            // If the first Future returns, it means that the button was pressed
            //
            // The actual return value of the Future is not important so
            // a `_` is used to ask the compiler to discard the value.
            Either::First(_) => {
                // The intensity cannot go higher than 100, we use the
                // minimum between 100 and the computed intensity.
                led_intensity = min(100, led_intensity + 10)
            }

            // If the second Future returns, it means that the second button was pressed
            //
            // The actual return value of the Future is not important so
            // a `_` is used to ask the compiler to discard the value.
            Either::Second(_) => {
                // The intensity cannot be lower then 0, so using `saturating_sub` will
                // perform the subtraction but will not go bellow the data type's minimum
                // value. The `led_intensity`'s data type is `u8`, with a minimum value
                // of 0.
                led_intensity = led_intensity.saturating_sub(10)
            }
        }
    }
}
