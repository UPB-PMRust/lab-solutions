#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Instant, Timer};
use panic_probe as _;

fn busy_wait(ms: u64) {
    let start_time = Instant::now();
    while start_time.elapsed().as_millis() < ms {}
}

#[task(pool_size = 2)]
async fn led_blink(mut led_pin: Output<'static>) {
    loop {
        led_pin.set_low();
        busy_wait(500);
        led_pin.set_high();
        busy_wait(500);
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

    spawner.spawn(led_blink(led_red)).unwrap();
    spawner.spawn(led_blink(led_blue)).unwrap();

    loop {
        Timer::after_secs(1).await;
    }
}
