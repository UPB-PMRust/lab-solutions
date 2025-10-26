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

    loop {
        led.set_low();
        Timer::after_millis(300).await;
        led.set_high();
        // Make sure you do not forget this delay. as otherwise the LED
        // will not blink, it will be turned off and immediately
        // turned on again in the next loop cycle.
        Timer::after_millis(300).await;
    }
}
