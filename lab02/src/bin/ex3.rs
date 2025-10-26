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

    // The LEDs on the lab board are active LOW: they light up when the pin is LOW
    // and turn off when the pin is HIGH. We set the initial value of the pin to HIGH
    // so that the LED are turned off when the pins are setup.
    let mut led = Output::new(peripherals.PC7, Level::High, Speed::Low);

    loop {
        led.set_low();
        Timer::after_millis(300).await;
        led.set_high();
        // Make sure you do not forget this delay. Without it,
        // the LED won’t appear to blink: it would just turn off and
        // immediately back on at the beginning od the the next loop cycle.
        Timer::after_millis(300).await;
    }
}
