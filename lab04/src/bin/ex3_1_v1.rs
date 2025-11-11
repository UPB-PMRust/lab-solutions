#![no_std]
#![no_main]

use core::cmp::min;

use async_debounce::Debouncer;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
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
use embassy_time::Duration;
use embedded_hal_async::digital::Wait;
use panic_probe as _;

/// The possible commands that button tasks can send
enum Command {
    /// Increase the LED's intensity
    IncreaseIntensity,
    /// Decrease the LED's intensity
    DecreaseIntensity,
}

/// The channel used to send commands from the button tasks to the main task.
///
/// The channel is sending `Command` values and has a capacity of 50.
/// When the capacity is full, sending tasks will either fail to send
/// a message or will be suspended (`.await`) until the channel has space.
static COMMANDS_CHANNEL: Channel<ThreadModeRawMutex, Command, 50> = Channel::new();

/// The period in which a button's value has to stay stable
/// to be considered pressed or released.
///
/// Due to their mechanical construction, when pressed or released,
/// buttons generate voltage fluctuations that the GPIO pin might
/// register as several pressed and releases. To avoid this,
/// we have to debounce the signal. The general idea is:
/// - in a loop
///     - wait for the rising or falling edge
///     - wait for an amount of time
///     - if the value is still correct (HIGH or LOW) return
///     - if the value changed, it means it was a transitory
///       signal, go back and wait for another edge
/// ```
/// async fn debounce_wait_for_falling_edge (pin: ExtiInput<'static>, stable_for: Duration) {
///     loop {
///         pin.wait_for_falling_edge().await;
///         Timer::after(duration).await;
///         if pin.is_low() {
///             break;
///         }
///     }
/// }
/// ```
///
const DEBOUNCE_STABLE_PERIOD: Duration = Duration::from_millis(100);

/// Task that waits for a button press and send a command
/// to increase the LED's intensity.
///
/// The task receives the button that it waits for and the sending part
/// of the commands channel.
#[task]
async fn increase_intensity(
    mut button: Debouncer<ExtiInput<'static>>,
    sender: DynamicSender<'static, Command>,
) {
    loop {
        // Wait for a button press
        button.wait_for_falling_edge().await.ok();

        // Send the IncreaseIntensity command to the channel
        sender.send(Command::IncreaseIntensity).await;
    }
}

/// Task that waits for a button press and send a command
/// to decrease the LED's intensity.
///
/// The task receives the button that it waits for and the sending part
/// of the commands channel.
#[task]
async fn decrease_intensity(
    mut button: Debouncer<ExtiInput<'static>>,
    sender: DynamicSender<'static, Command>,
) {
    loop {
        // Wait for a button press
        button.wait_for_falling_edge().await.ok();

        // Send the DecreaseIntensity command to the channel
        sender.send(Command::DecreaseIntensity).await;
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
    //
    // Buttons have to be debounced to prevent the tasks from reading several
    // button presses due to electrical noise generated when the button is pressed.
    // `Debouncer` takes a GPIO Input and debounces the signal. It exposes similar
    // functions with `ExitInput`.
    let button_s1 = Debouncer::new(
        ExtiInput::new(peripherals.PA8, peripherals.EXTI8, Pull::None),
        DEBOUNCE_STABLE_PERIOD,
    );
    let button_s2 = Debouncer::new(
        ExtiInput::new(peripherals.PC7, peripherals.EXTI7, Pull::None),
        DEBOUNCE_STABLE_PERIOD,
    );

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

    // Get the sending end of the channel. This will be sent to all the tasks
    // that want to send commands to the channel.
    //
    // NOTE: The actual `Sender` type has a lot of parameters as it is a generic type.
    //       While using the `Sender` type is generally faster as the compiler can
    //       optimize the code, sending it to a function implies writing a long
    //       type name in the function's parameter. Using `DynamicSender` hides
    //       the long type name at a small speed penalty.
    let sender = COMMANDS_CHANNEL.dyn_sender();

    // Get the receiving end of the channel. This will be used by the
    // main task to receive commands.
    //
    // NOTE: The actual `Receiver` type is used here, as there is no function that
    //       receives it and the type does not have to be named, the compiler
    //       figures it out.
    let receiver = COMMANDS_CHANNEL.receiver();

    // Start a `increase_intensity` task that runs in parallel with the `main` (this) task.
    // The task receives two parameters that represent the button and the
    // sending end of the commands channel.
    //
    // The task will start executing only when the main task
    // finishes or uses an `.await`.
    //
    // NOTE: The `increase_intensity` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner
        .spawn(increase_intensity(button_s1, sender))
        .unwrap();

    // Start a `decrease_intensity` task that runs in parallel with the `main` (this) task.
    // The task receives two parameters that represent the button and the
    // sending end of the commands channel.
    //
    // The task will start executing only when the main task or the `increase_task``
    // finish or use an `.await`.
    //
    // NOTE: The `decrease_intensity` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner
        .spawn(decrease_intensity(button_s2, sender))
        .unwrap();

    // The initial LED's intensity percent
    let mut led_intensity = 0u8;

    loop {
        // Set the intensity by modifying the duty cycle
        led.set_duty_cycle_percent(led_intensity);

        // Wait for a command from one of the `increase_intensity` or `decrease_intensity` tasks
        let command = receiver.receive().await;

        match command {
            Command::IncreaseIntensity => {
                // The intensity cannot go higher than 100, we use the
                // minimum between 100 and the computed intensity.
                led_intensity = min(100, led_intensity + 10)
            }
            Command::DecreaseIntensity => {
                // The intensity cannot be lower then 0, so using `saturating_sub` will
                // perform the subtraction but will not go bellow the data type's minimum
                // value. The `led_intensity`'s data type is `u8`, with a minimum value
                // of 0.
                led_intensity = led_intensity.saturating_sub(10)
            }
        }
    }
}
