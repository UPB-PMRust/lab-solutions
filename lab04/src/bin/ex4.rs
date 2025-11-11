#![no_std]
#![no_main]

use async_debounce::Debouncer;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Level, Output, Pull, Speed},
};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use lab04::traffic_light::{TrafficLightState, blink_yellow, set_green, set_red};
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

    // The LEDs on the lab board are active LOW: they light up when the pin is LOW
    // and turn off when the pin is HIGH. We set the initial value of the pin to HIGH
    // so that the LED are turned off when the pins are setup.

    // The red LED is connected to D8 (PC7).
    let mut red = Output::new(peripherals.PC7, Level::High, Speed::Low);
    // The yellow LED is connected to D9 (PC6).
    let mut yellow = Output::new(peripherals.PC6, Level::High, Speed::Low);
    // The green LED is connected to D10 (PC9).
    let mut green = Output::new(peripherals.PC9, Level::High, Speed::Low);

    let mut traffic_light_state = TrafficLightState::Red;

    loop {
        let traffic_light_control = async {
            info!("Traffic Light {}", traffic_light_state);
            match traffic_light_state {
                TrafficLightState::Red => {
                    // The `set_red` function takes mutable borrows (references)
                    // to the LEDs.
                    set_red(&mut red, &mut yellow, &mut green);
                    Timer::after_secs(5).await;
                }
                TrafficLightState::Yellow => {
                    // The `set_yellow` function takes mutable borrows (references)
                    // to the LEDs.
                    blink_yellow(&mut red, &mut yellow, &mut green).await;
                }
                TrafficLightState::Green => {
                    // The `set_green` function takes mutable borrows (references)
                    // to the LEDs.
                    set_green(&mut red, &mut yellow, &mut green);
                    Timer::after_secs(10).await;
                }
            }
        };

        let action = select(traffic_light_control, button_s1.wait_for_falling_edge()).await;

        // Wait for the timer to expire or the button to be pressed

        match action {
            Either::First(_) => traffic_light_state = traffic_light_state.next(),
            Either::Second(_) => match traffic_light_state {
                TrafficLightState::Yellow | TrafficLightState::Green => {
                    traffic_light_state = traffic_light_state.next();
                }
                TrafficLightState::Red => {}
            },
        }
    }
}
