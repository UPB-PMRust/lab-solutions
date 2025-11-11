#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Instant;
use panic_probe as _;

/// Implements a wait function using busy waiting.
///
/// Busy waiting means holding the CPU busy while the
/// wait time elapses. This is very inefficient, as nothing
/// else can run while this function executes.
///
/// NOTE: Using this within embassy will block all the tasks
/// until this function returns.
fn busy_wait(ms: u64) {
    let start_time = Instant::now();
    while start_time.elapsed().as_millis() < ms {}
}

/// Task the blinks the LED
///
/// The `task` macro transforms the function into an embassy
/// task that can be spawned by a `Spanner`.
///
/// The `pool_size` argument asks Embassy to allocate enough
/// memory for two identical tasks that run in parallel.
#[task(pool_size = 2)]
async fn led_blink(mut led_pin: Output<'static>) {
    // This loop blocks the embassy executor as it loops
    // forever without executing any `.await`.
    //
    // NOTE: as soon as this task starts executing, no other task
    // will be able to execute as this task never uses `.await`
    // and doesn't finish.
    //
    // The loop should use an asynchronous `wait` function like
    // `Timer::after_millis(500).await. This would interrupt
    // the `loop` from while ti waits and allow other
    // tasks to run.
    loop {
        led_pin.set_low();
        busy_wait(500);
        // CORRECT: Timer::after_millis(500).await;

        led_pin.set_high();
        busy_wait(500);
        // CORRECT: Timer::after_millis(500).await;
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
    // The red LED is connected to D8 (PC7)
    let led_red = Output::new(peripherals.PC7, Level::High, Speed::Low);
    // The blue LED is connected to D9 (PC6)
    let led_blue = Output::new(peripherals.PC6, Level::High, Speed::Low);

    // Start a `led_blink` task that runs in parallel with the `main` (this) task.
    // The task receives as parameter that represents the red LED.
    //
    // The task will start executing only when the main task
    // finishes or uses an `.await`.
    //
    // NOTE: The `led_blink` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner.spawn(led_blink(led_red)).unwrap();

    // Start a `led_blink` task that runs in parallel with the `main` (this) task.
    // The task receives as parameter that represents the blue LED.
    //
    // The task will start executing only when the main task or the first `led_blink`
    // task finish or use an `.await`.
    //
    // NOTE: The `led_blink` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner.spawn(led_blink(led_blue)).unwrap();
}
