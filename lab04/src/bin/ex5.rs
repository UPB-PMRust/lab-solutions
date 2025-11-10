#![no_std]
#![no_main]

use async_debounce::Debouncer;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::{
    join::join,
    select::{Either, select},
};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Level, Output, Pull, Speed},
};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use lab04::traffic_light::{Action, TrafficLightState, set_green, set_red, set_yellow, turn_off};
use panic_probe as _;

async fn blink_yellow(
    red: &mut Output<'_>,
    yellow: &mut Output<'_>,
    green: &mut Output<'_>,
    button1: &mut Debouncer<ExtiInput<'_>>,
    button2: &mut Debouncer<ExtiInput<'_>>,
) -> Action {
    for _ in 0..3 {
        set_yellow(red, yellow, green);
        if let Either::Second(_) = select(
            Timer::after_millis(125),
            join(
                button1.wait_for_falling_edge(),
                button2.wait_for_falling_edge(),
            ),
        )
        .await
        {
            return Action::ButtonPressed;
        }
        turn_off(red, yellow, green);
        if let Either::Second(_) = select(
            Timer::after_millis(125),
            join(
                button1.wait_for_falling_edge(),
                button2.wait_for_falling_edge(),
            ),
        )
        .await
        {
            return Action::ButtonPressed;
        }
    }
    Action::Timeout
}

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
    let mut button_s1 = Debouncer::new(
        ExtiInput::new(peripherals.PA8, peripherals.EXTI8, Pull::None),
        Duration::from_millis(100),
    );
    let mut button_s3 = Debouncer::new(
        ExtiInput::new(peripherals.PA7, peripherals.EXTI7, Pull::None),
        Duration::from_millis(100),
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
        let action = match traffic_light_state {
            TrafficLightState::Red => {
                // The `set_red` function takes mutable borrows (references)
                // to the LEDs.
                set_red(&mut red, &mut yellow, &mut green);
                match select(
                    Timer::after_secs(2),
                    join(
                        button_s1.wait_for_falling_edge(),
                        button_s3.wait_for_falling_edge(),
                    ),
                )
                .await
                {
                    Either::First(_) => Action::Timeout,
                    Either::Second(_) => Action::ButtonPressed,
                }
            }
            TrafficLightState::Yellow => {
                // The `set_yellow` function takes mutable borrows (references)
                // to the LEDs.
                blink_yellow(
                    &mut red,
                    &mut yellow,
                    &mut green,
                    &mut button_s1,
                    &mut button_s3,
                )
                .await
            }
            TrafficLightState::Green => {
                // The `set_green` function takes mutable borrows (references)
                // to the LEDs.
                set_green(&mut red, &mut yellow, &mut green);
                match select(Timer::after_secs(5), button_s1.wait_for_falling_edge()).await {
                    Either::First(_) => Action::Timeout,
                    Either::Second(_) => Action::ButtonPressed,
                }
            }
        };

        // Wait for the timer to expire or the button to be pressed

        match action {
            Action::ButtonPressed => match traffic_light_state {
                TrafficLightState::Yellow | TrafficLightState::Green => {
                    traffic_light_state = traffic_light_state.next();
                }
                TrafficLightState::Red => {}
            },
            Action::Timeout => traffic_light_state = traffic_light_state.next(),
        }
    }
}
