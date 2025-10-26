#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LEDs on the lab board are active LOW, meaning the light up when the pin is LOW
    // and turn off when the pin is LOW. We set the initial value of the pin to HIGH
    // to turn off the LED.
    let mut led = Output::new(peripherals.PC7, Level::High, Speed::Low);
    led.set_low();

    // When the `main` function exists, `embassy` resets all the pins
    // and we will not see the LED on.
    // The loop prevents the `main` function from existing.
    loop {
        // It is a good idea to place a delay inside this loop,
        // otherwise the MCU will execute an empty loop and
        // might heat up.
        // Placing a delay makes the MCU wait for a few milliseconds
        // in between loops.
        Timer::after_millis(10).await;
    }
}
