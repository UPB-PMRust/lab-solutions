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
use lab04::traffic_light::{TrafficLightState, blink_yellow, set_green, set_red};
use panic_probe as _;

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
        let traffic_light_state = subscriber.next_message_pure().await;

        // next_message
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

#[task]
async fn sound(
    mut buzzer_pwm: SimplePwm<'static, TIM2>,
    mut subscriber: DynSubscriber<'static, TrafficLightState>,
) {
    let mut beep = false;
    loop {
        let traffic_light_state = select(subscriber.next_message_pure(), async {
            // Get a mutable reference to channel 2 of TIM 2 to control it
            let mut buzzer = buzzer_pwm.ch2();

            buzzer.enable();
            buzzer.set_duty_cycle_percent(50);
            loop {
                if beep {
                    Timer::after_millis(500).await;
                    buzzer.disable();
                    Timer::after_millis(500).await;
                    buzzer.enable();
                } else {
                    Timer::after_secs(1).await;
                }
            }
        })
        .await;

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

    let mut traffic_light_state = TrafficLightState::Red;

    let publisher = TRAFFIC_LIGHT_STATUS.publisher().unwrap();
    let subscriber_barrier = TRAFFIC_LIGHT_STATUS.dyn_subscriber().unwrap();
    let subscriber_sound = TRAFFIC_LIGHT_STATUS.dyn_subscriber().unwrap();

    spawner
        .spawn(barrier(servo_pwm, subscriber_barrier))
        .unwrap();
    spawner.spawn(sound(buzzer_pwm, subscriber_sound)).unwrap();

    loop {
        let traffic_light_control = async {
            info!("Traffic Light {}", traffic_light_state);
            publisher.publish_immediate(traffic_light_state);
            // publisher.publish(traffic_light_state).await;
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

        let action = select(traffic_light_control, button_s1.wait_for_falling_edge()).await;

        // Wait for the timer to expire or the button to be pressed

        match action {
            Either::First(_) => {
                info!("Timeout");
                traffic_light_state = traffic_light_state.next();
            }
            Either::Second(_) => {
                info!("Buttons pressed");
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
