#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_time::Timer;
use panic_probe as _;

/// The LED's state
enum LedState {
    On,
    Off,
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LEDs on the lab board are active LOW: they light up when the pin is LOW
    // and turn off when the pin is HIGH. We set the initial value of the pin to HIGH
    // so that the LED are turned off when the pins are setup.
    let mut led = Output::new(peripherals.PC7, Level::High, Speed::Low);

    // The buttons on the lab board have an external pull up resistor (soldered
    // on the lab board), so the internal pull resistor is not needed.
    // Pull Up means that:
    //    - the pin's value is HIGH when the button is release
    //    - the pin's value is LOW when the button is pressed
    // We can either use `Pull::None` or `Pull::Up` (not recommended),
    // we cannot use `Pull::Down`.
    let button = Input::new(peripherals.PA8, Pull::None);

    // Define a mutable variable to store the LED's state.
    let mut led_state = LedState::Off;

    loop {
        // We do nothing (actually sleep very a few milliseconds) while
        // the button is not pressed.
        while !button.is_low() {
            Timer::after_millis(10).await;
        }

        // Modify the LED's state
        led_state = match led_state {
            // if the current led_state is LedState::Off, make it LedState::On
            LedState::Off => LedState::On,

            // if the current led_state is LedState::On, make it LedState::Off
            LedState::On => LedState::Off,
        };

        // light up or turn off the LED
        match led_state {
            LedState::On => led.set_low(),
            LedState::Off => led.set_high(),
        }

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
