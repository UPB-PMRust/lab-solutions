#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use panic_probe as _;

/// Lights up the red LED and turns off the yellow and green LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
fn set_red(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_low();
    yellow.set_high();
    green.set_high();
}

/// Lights up the yellow LED and turns off the red and green LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
fn set_yellow(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_high();
    yellow.set_low();
    green.set_high();
}

/// Lights up the green LED and turns off the red and yellow LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
fn set_green(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_high();
    yellow.set_high();
    green.set_low();
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LEDs on the lab board are active LOW: they light up when the pin is LOW
    // and turn off when the pin is HIGH. We set the initial value of the pin to HIGH
    // so that the LED are turned off when the pins are setup.

    // The red LED is connected to D8 (PC7).
    let mut red = Output::new(peripherals.PC7, Level::High, Speed::Low);
    // The yellow LED is connected to D9 (PC6).
    let mut yellow = Output::new(peripherals.PC6, Level::High, Speed::Low);
    // The green LED is connected to D10 (PC9).
    let mut green = Output::new(peripherals.PC9, Level::High, Speed::Low);

    loop {
        // The `set_red` function takes mutable borrows (references)
        // to the LEDs.
        set_red(&mut red, &mut yellow, &mut green);
        Timer::after_secs(3).await;

        // The `set_green` function takes mutable borrows (references)
        // to the LEDs.
        set_green(&mut red, &mut yellow, &mut green);
        Timer::after_secs(3).await;

        // The `set_yellow` function takes mutable borrows (references)
        // to the LEDs.
        set_yellow(&mut red, &mut yellow, &mut green);
        Timer::after_secs(1).await;
    }
}
