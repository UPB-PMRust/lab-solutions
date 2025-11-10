#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use panic_probe as _;

#[task(pool_size = 4)]
async fn led_blink(mut led_pin: Output<'static>, frequency: u64) {
    let millis = 1000 / frequency / 2;
    loop {
        led_pin.set_low();
        Timer::after_millis(millis).await;
        led_pin.set_high();
        Timer::after_millis(millis).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LEDs on the lab board are active LOW: they light up when the pin is LOW
    // and turn off when the pin is HIGH. We set the initial value of the pin to HIGH
    // so that the LED are turned off when the pins are setup.
    let led_red = Output::new(peripherals.PC7, Level::High, Speed::Low);
    let led_blue = Output::new(peripherals.PC6, Level::High, Speed::Low);
    let led_yellow = Output::new(peripherals.PC9, Level::High, Speed::Low);
    let led_green = Output::new(peripherals.PA7, Level::High, Speed::Low);

    spawner.spawn(led_blink(led_yellow, 3)).unwrap();
    spawner.spawn(led_blink(led_red, 4)).unwrap();
    spawner.spawn(led_blink(led_green, 5)).unwrap();
    spawner.spawn(led_blink(led_blue, 1)).unwrap();

    loop {
        Timer::after_secs(1).await;
    }
}
