#![no_std]
#![no_main]

use core::cmp::min;

use async_debounce::Debouncer;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{OutputType, Pull},
    peripherals::TIM2,
    time::khz,
    timer::{
        Ch2,
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Channel, DynamicSender},
};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use panic_probe as _;

static COMMANDS_CHANNEL: Channel<ThreadModeRawMutex, u8, 50> = Channel::new();

#[task]
async fn adjust_intensity(
    sender: DynamicSender<'static, u8>,
    button_increase: ExtiInput<'static>,
    button_decrease: ExtiInput<'static>,
) {
    let mut led_intensity = 0u8;
    let mut debounced_button_increase = Debouncer::new(button_increase, Duration::from_millis(100));
    let mut debounced_button_decrease = Debouncer::new(button_decrease, Duration::from_millis(100));
    loop {
        sender.send(led_intensity).await;
        Timer::after_millis(100).await;
        let button = select(
            debounced_button_increase.wait_for_falling_edge(),
            debounced_button_decrease.wait_for_falling_edge(),
        )
        .await;
        match button {
            Either::First(_) => led_intensity = min(100, led_intensity + 10),
            Either::Second(_) => led_intensity = led_intensity.saturating_sub(10),
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LED is connected on pin D3 (PB3)
    //
    // PB3 can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let led_red: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // The buttons on the lab board have an external pull up resistor (soldered
    // on the lab board), so the internal pull resistor is not needed.
    // Pull Up means that:
    //    - the pin's value is HIGH when the button is release
    //    - the pin's value is LOW when the button is pressed
    // We can either use `Pull::None` or `Pull::Up` (not recommended),
    // we cannot use `Pull::Down`.
    let button_s1 = ExtiInput::new(peripherals.PA8, peripherals.EXTI8, Pull::None);
    let button_s2 = ExtiInput::new(peripherals.PC7, peripherals.EXTI7, Pull::None);

    // Enable PWM for TIM2
    // only Channel 2 will be used and connected to pin PB3
    let mut pwm = SimplePwm::new(
        peripherals.TIM2,   // Timer 2 peripheral
        None,               // Channel 1 not used
        Some(led_red),      // Channel 2 output (PB3)
        None,               // Channel 3 not used
        None,               // Channel 4 not used
        khz(1),             // PWM frequency = 1 kHz
        Default::default(), // Default configuration
    );

    // Get a reference to channel 2 of TIM 2 to control it
    let mut led = pwm.ch2();

    // Start PWM on the channel
    led.enable();

    // The PWM polarity configures what the duty cycle means:
    // - ActiveHigh (default) -> the amount of time the PWM signal is HIGH
    // - ActiveLow -> the amount of time the PWM signal is LOW
    //
    // The LEDs on the lab board are active LOW: they light up when the PWM signal is LOW
    // and turn off when the PWM signal is HIGH. We set the polarity to LOW so that
    // the LED turns on during the PWM's duty cycle period.
    led.set_polarity(OutputPolarity::ActiveLow);

    let sender = COMMANDS_CHANNEL.dyn_sender();
    let receiver = COMMANDS_CHANNEL.receiver();

    spawner
        .spawn(adjust_intensity(sender, button_s1, button_s2))
        .unwrap();

    loop {
        let led_intensity = receiver.receive().await;
        // Set the duty cycle of the channel
        led.set_duty_cycle_percent(led_intensity);
    }
}
