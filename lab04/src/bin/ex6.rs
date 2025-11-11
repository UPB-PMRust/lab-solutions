#![no_std]
#![no_main]

use async_debounce::Debouncer;
use defmt::{debug, info};
use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Level, Output, OutputType, Pull, Speed},
    peripherals::{TIM2, TIM3},
    time::hz,
    timer::{
        Ch1, Ch2,
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    pubsub::{DynSubscriber, PubSubChannel},
};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use panic_probe as _;

// There are several exercises that use the same date types and functions for
// the Traffic Light, so these are grouped in a library. Take a look
// at `src/lib.rs`.
use lab04::traffic_light::{TrafficLightState, blink_yellow, set_green, set_red};

/// The channel used to publish the traffic light state from the main task.
///
/// The channel is publishing `TrafficLightState` values, has a capacity of 50,
/// allows 2 subscribers and 1 publisher.
///
/// When the capacity is full, publishers tasks will either fail to publish
/// a message or will be suspended (`.await`) until the channel has space.
static TRAFFIC_LIGHT_STATUS: PubSubChannel<ThreadModeRawMutex, TrafficLightState, 50, 2, 1> =
    PubSubChannel::new();

/// Computes the duty cycle per_mille (to the 1000) for
/// a given angle.
fn servo_duty_cycle_per_mille_for_angle(angle: u8) -> u16 {
    const MIN_PULSE_US: u32 = 500; // 0 degrees is 0.5 ms
    const MAX_PULSE_US: u32 = 2500; // 0 degrees is 2.5 ms
    const PERIOD_US: u32 = 20000; // 50 Hz period

    // 180 degrees ......... 2500 us
    // 0 degrees ........... 500 us
    // `angle` degrees ..... n us
    //
    // n = angle x 180 degree pulse length (ie 2500 - 500) / 180 + 500
    let pulse = angle as u32 * (MAX_PULSE_US - MIN_PULSE_US) / 180 + MIN_PULSE_US;

    // 100% duty cycle ...... 20000 us (20 ms, PWM 50 Hz)
    // n duty cycle ......... pulse us
    //
    // Due to accuracy, we compute the per mille (to the 1000)
    // We compute using `u32` due to accuracy and transform the
    // value to `u16`.
    let period_per_mille = ((pulse * 1000) / PERIOD_US) as u16;

    // Display the pulse length and the pulse per mille (to the 1000)
    debug!("Pulse length {} / {}‰", pulse, period_per_mille);

    // Return the `period_per_mille`
    period_per_mille
}

/// Task that handles the servo barrier
///
/// The `task` macro transforms the function into an embassy
/// task that can be spawned by a `Spawner`.
///
/// The task receives the traffic light state notification and
/// moves the barrier.
#[task]
async fn barrier(
    mut servo_pwm: SimplePwm<'static, TIM3>,
    mut subscriber: DynSubscriber<'static, TrafficLightState>,
) {
    // Get a mutable reference to channel 2 of TIM 2 to control it
    let mut servo = servo_pwm.ch1();

    // Start PWM on the channel
    servo.enable();

    // The PWM polarity configures what the duty cycle means:
    // - ActiveHigh (default) -> the amount of time the PWM signal is HIGH
    // - ActiveLow -> the amount of time the PWM signal is LOW
    //
    // The servo reads the amount of time the signal is HIGH.
    servo.set_polarity(OutputPolarity::ActiveHigh);

    loop {
        // Wait for the traffic light state update.
        //
        // The `next_message_pure` function returns the published latest message
        // that the channel still has. The subscriber might have missed some
        // messages.
        //
        // Using `next_message` will return either the latest message or
        // the number of missed messages.
        //
        // As this task does not care if it misses some messages, it uses
        // `next_message_pure` to get the next message.
        let traffic_light_state = subscriber.next_message_pure().await;

        match traffic_light_state {
            TrafficLightState::Yellow | TrafficLightState::Red => {
                info!("Barrier is closed");

                // Calculate the duty cycle per mille
                let servo_per_mille = servo_duty_cycle_per_mille_for_angle(0);

                // Set the duty cycle fraction per mille
                servo.set_duty_cycle_fraction(servo_per_mille, 1000);
            }
            TrafficLightState::Green => {
                info!("Barrier is open");

                // Calculate the duty cycle per mille
                let servo_per_mille = servo_duty_cycle_per_mille_for_angle(90);

                // Set the duty cycle fraction per mille
                servo.set_duty_cycle_fraction(servo_per_mille, 1000);
            }
        }
    }
}

/// Task that handles the buzzer
///
/// The `task` macro transforms the function into an embassy
/// task that can be spawned by a `Spawner`.
///
/// The task receives the traffic light state notification and
/// controls the buzzer.
#[task]
async fn sound(
    mut buzzer_pwm: SimplePwm<'static, TIM2>,
    mut subscriber: DynSubscriber<'static, TrafficLightState>,
) {
    let mut beep = false;
    loop {
        // Execute a task while waiting for another task:
        // - wait for a message to arrive
        // - repeat the traffic light sound pattern until
        //   a message arrives
        //
        // `select` receives two Futures as parameters and waits
        // for one of them to finish. When a Future finishes, the
        // other Future is dropped and `select` returns.
        //
        // NOTE: The `next_message_pure` function and the
        //       async block that controls the sound are called
        //       without an `.await` as `select` requires the
        //       Futures, not the Futures' result.
        //       The `.await` is used for the `select` function.
        let traffic_light_state = select(
            // Wait for the traffic light state update.
            //
            // The `next_message_pure` function returns the published latest message
            // that the channel still has. The subscriber might have missed some
            // messages.
            //
            // Using `next_message` will return either the latest message or
            // the number of missed messages.
            //
            // As this task does not care if it misses some messages, it uses
            // `next_message_pure` to get the next message.
            subscriber.next_message_pure(),
            // Control the PWM to generate the buzzer sound
            async {
                // Get a mutable reference to channel 2 of TIM 2 to control it
                let mut buzzer = buzzer_pwm.ch2();

                // Start generating the buzzer sound
                buzzer.enable();

                // Set a duty cycle of 50%.
                //
                // The duty cycle is not very relevant as this generates sound.
                buzzer.set_duty_cycle_percent(50);
                loop {
                    if beep {
                        Timer::after_millis(500).await;
                        // Stop the sound
                        buzzer.disable();
                        Timer::after_millis(500).await;
                        // Start the sound
                        buzzer.enable();
                    } else {
                        // This should be "wait forever", but as `embassy-rs` does
                        // not provide such a function, wait 1 second
                        // make use of the loop to wait again.
                        Timer::after_secs(1).await;
                    }
                }
            },
        )
        .await;

        // Use pattern matching to extract the traffic light state.
        //
        // This should have used a `match` statement, but the second Future of the `select`
        // function never returns (it is an infinite `loop`) and the compiler figures out
        // that `Either::Second` cannot be returned.
        let Either::First(traffic_light_state) = traffic_light_state;

        // next_message
        match traffic_light_state {
            TrafficLightState::Red => {
                buzzer_pwm.set_frequency(hz(400));
                beep = true;
            }
            TrafficLightState::Yellow | TrafficLightState::Green => {
                buzzer_pwm.set_frequency(hz(200));
                beep = false;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

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
    // functions with `ExtiInput`.
    //
    // The S1 button is connected on pin D7 (PA8).
    let mut button_s1 = Debouncer::new(
        ExtiInput::new(peripherals.PA8, peripherals.EXTI8, Pull::None),
        Duration::from_millis(100),
    );

    // The LEDs on the lab board are active LOW: they light up when the pin is LOW
    // and turn off when the pin is HIGH. We set the initial value of the pin to HIGH
    // so that the LED are turned off when the pins are setup.

    // The red LED is connected to D8 (PC7).
    let mut red = Output::new(peripherals.PC7, Level::High, Speed::Low);
    // The yellow LED is connected to D9 (PC6).
    let mut yellow = Output::new(peripherals.PC6, Level::High, Speed::Low);
    // The green LED is connected to D10 (PC9).
    let mut green = Output::new(peripherals.PC9, Level::High, Speed::Low);

    // PB4 can be connected for PWM to Channel 3 of TIM 1
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB4.
    let servo_pwm_pin: PwmPin<'_, TIM3, Ch1> = PwmPin::new(peripherals.PB4, OutputType::PushPull);

    // Enable PWM for TIM3
    // only Channel 1 will be used and connected to pin PB4
    let servo_pwm = SimplePwm::new(
        peripherals.TIM3,    // Timer 3 peripheral
        Some(servo_pwm_pin), // Channel 1 output (PB4)
        None,                // Channel 2 not used
        None,                // Channel 3 not used
        None,                // Channel 4 not used
        hz(50),              // the servo needs a PWM frequency of 50 Hz
        Default::default(),  // Default configuration
    );

    // The LED is connected on pin D3 (PB3)
    //
    // PB3 can be connected for PWM to Channel 2 of TIM 2
    // The `PwmPin` sets the correct configuration of the MODER and
    // the Alternate Function of the pin PB3.
    let buzzer_pin: PwmPin<'_, TIM2, Ch2> = PwmPin::new(peripherals.PB3, OutputType::PushPull);

    // Enable PWM for TIM2
    // only Channel 2 will be used and connected to pin PB3
    let buzzer_pwm = SimplePwm::new(
        peripherals.TIM2,   // Timer 2 peripheral
        None,               // Channel 1 not used
        Some(buzzer_pin),   // Channel 2 output (PB3)
        None,               // Channel 3 not used
        None,               // Channel 4 not used
        hz(1),              // PWM frequency = 1 kHz
        Default::default(), // Default configuration
    );

    // The initial traffic light state
    let mut traffic_light_state = TrafficLightState::Red;

    // Get the publishing end of the channel. This will be used by the
    // main task to publish the traffic light status.
    //
    // NOTE: The actual `Publisher` type is used here, as there is no function that
    //       receives it and the type does not have to be named, the compiler
    //       figures it out.
    let publisher = TRAFFIC_LIGHT_STATUS.publisher().unwrap();

    // Get the a subscribing end of the channel for the `barrier` and `sound` tasks. These
    // tasks will receive each one a subscribing end that that they will use to receive
    // traffic light state updates.
    //
    // NOTE: The actual `Subscriber` type has a lot of parameters as it is a generic type.
    //       While using the `Subscriber` type is generally faster as the compiler can
    //       optimize the code, sending it to a function implies writing a long
    //       type name in the function's parameter. Using `DynSubscriber` hides
    //       the long type name at a small speed penalty.
    let subscriber_barrier = TRAFFIC_LIGHT_STATUS.dyn_subscriber().unwrap();
    let subscriber_sound = TRAFFIC_LIGHT_STATUS.dyn_subscriber().unwrap();

    // Start the `barrier` task that runs in parallel with the `main` (this) task.
    // The task receives three parameters that represent the PWM device and a
    // subscriber end of the traffic light state channel.
    //
    // The task will start executing only when the main task
    // finishes or uses an `.await`.
    //
    // NOTE: The `barrier` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner
        .spawn(barrier(servo_pwm, subscriber_barrier))
        .unwrap();

    // Start the `sound` task that runs in parallel with the `main` (this) task.
    // The task receives three parameters that represent the PWM device and a
    // subscriber end of the traffic light state channel.
    //
    // The task will start executing only when the main task
    // finishes or uses an `.await`.
    //
    // NOTE: The `sound` function is called without an `.await` as the
    //       spawner requires the task's Future, not the Future's result.
    spawner.spawn(sound(buzzer_pwm, subscriber_sound)).unwrap();

    loop {
        // `async` blocks are used to group several asynchronous actions together
        // that will be executed at some point later in the firmware.
        //
        // The `traffic_light_control` variable stores the Future returned
        // by the `async` block. Instructions in the `async` block are not
        // executed until the `traffic_light_control` is awaited.
        //
        // To memorize the Future returned by a block, use
        // `let future = async { ... };`
        //
        // To execute a block immediately and memorize the value it returns, use
        // `let value = async {...}.await`;
        let traffic_light_control = async {
            info!("Traffic Light {}", traffic_light_state);

            // Publish the traffic light state
            //
            // This function will do its best to publish the traffic light state
            // to the channel. If the channel is at capacity, it will just drop
            // the message.
            //
            // if not dropping messages is important, the task can use the
            // `publish` function. When the channel is at capacity, this
            // function suspends the task until it can publish the message.
            //
            // ```
            // publisher.publish(traffic_light_state).await;
            // ```
            publisher.publish_immediate(traffic_light_state);

            match traffic_light_state {
                TrafficLightState::Red => {
                    // The `set_red` function takes mutable borrows (references)
                    // to the LEDs.
                    set_red(&mut red, &mut yellow, &mut green);
                    Timer::after_secs(5).await;
                }
                TrafficLightState::Yellow => {
                    // The `set_yellow` function takes mutable borrows (references)
                    // to the LEDs.
                    blink_yellow(&mut red, &mut yellow, &mut green).await
                }
                TrafficLightState::Green => {
                    // The `set_green` function takes mutable borrows (references)
                    // to the LEDs.
                    set_green(&mut red, &mut yellow, &mut green);
                    Timer::after_secs(10).await;
                }
            }
        };

        // Wait for one of the actions to happen:
        // - the traffic light control has reached the end of a state
        //   and wants a new state
        // - the button was pressed
        //
        // `select` receives two Futures as parameters and waits
        // for one of them to finish. When a Future finishes, the
        // other Future is dropped and `select` returns.
        //
        // NOTE: The `traffic_light_control` block and the
        //       `wait_for_falling_edge` function are called
        //       without an `.await` as `select` requires the
        //       Futures, not the Futures' result.
        //       The `.await` is used for the `select` function.
        let action = select(traffic_light_control, button_s1.wait_for_falling_edge()).await;

        match action {
            // If the first Future returns, it means that the `traffic_light_control` has
            // finished.
            //
            // The actual return value of the Future is not important so
            // a `_` is used to ask the compiler to discard the value.
            Either::First(_) => {
                info!("Timeout");
                traffic_light_state = traffic_light_state.next();
            }

            // If the second Future returns, it means that the button was pressed
            //
            // The actual return value of the Future is not important so
            // a `_` is used to ask the compiler to discard the value.
            Either::Second(_) => {
                info!("Button pressed");
                match traffic_light_state {
                    TrafficLightState::Yellow | TrafficLightState::Green => {
                        traffic_light_state = traffic_light_state.next();
                    }
                    TrafficLightState::Red => {}
                }
            }
        }
    }
}
