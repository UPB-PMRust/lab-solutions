#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use panic_probe as _;

/// Task that blinks the LED
///
/// The `task` macro transforms the function into an embassy
/// task that can be spawned by a `Spawner`.
///
/// The `pool_size` argument asks Embassy to allocate enough
/// memory for two identical tasks that run in parallel.
#[task(pool_size = 4)]
async fn led_blink(mut led_pin: Output<'static>, frequency: u64) {
    // The blink period is 1 / frequency [s] that is 1000 / frequency [ms]
    // To blink an LED, it has to be on for half of the period and off
    // for the other half, so we divide the period in 2.
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
    //
    // // The red LED is connected to D8 (PC7).
    let led_red = Output::new(peripherals.PC7, Level::High, Speed::Low);
    // The blue LED is connected to D9 (PC6).
    let led_blue = Output::new(peripherals.PC6, Level::High, Speed::Low);
    // The yellow LED is connected to D10 (PC9).
    let led_yellow = Output::new(peripherals.PC9, Level::High, Speed::Low);
    // The green LED is connected to D11 (PA7).
    let led_green = Output::new(peripherals.PA7, Level::High, Speed::Low);

    // Start a `led_blink` tasks that run in parallel with the `main` (this) task.
    // The tasks receive two parameters that represents the LED and
    // the blink frequency.
    //
    // Each task will start executing only when the main task and the
    // rest of the `led_blink` finish or use an `.await`.
    //
    // NOTE: The `led_blink` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner.spawn(led_blink(led_yellow, 3)).unwrap();
    spawner.spawn(led_blink(led_red, 4)).unwrap();
    spawner.spawn(led_blink(led_green, 5)).unwrap();
    spawner.spawn(led_blink(led_blue, 1)).unwrap();
}
