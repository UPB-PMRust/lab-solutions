use embassy_stm32::gpio::Output;
use embassy_time::Timer;

pub enum TrafficLightState {
    Red,
    Yellow,
    Green,
}

impl TrafficLightState {
    pub fn next(&self) -> TrafficLightState {
        match self {
            TrafficLightState::Red => TrafficLightState::Yellow,
            TrafficLightState::Yellow => TrafficLightState::Green,
            TrafficLightState::Green => TrafficLightState::Red,
        }
    }
}

/// Lights up the red LED and turns off the yellow and green LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
pub fn set_red(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_low();
    yellow.set_high();
    green.set_high();
}

/// Lights up the yellow LED and turns off the red and green LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
pub fn set_yellow(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_high();
    yellow.set_low();
    green.set_high();
}

/// Lights up the green LED and turns off the red and yellow LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
pub fn set_green(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_high();
    yellow.set_high();
    green.set_low();
}

pub async fn blink_yellow(red: &mut Output<'_>, yellow: &mut Output<'_>, green: &mut Output<'_>) {
    for _ in 0..3 {
        set_yellow(red, yellow, green);
        Timer::after_millis(500).await;
        turn_off(red, yellow, green);
        Timer::after_millis(500).await;
    }
}

pub fn turn_off(red: &mut Output, yellow: &mut Output, green: &mut Output) {
    // The LEDs on the lab board are active LOW: they light up when the
    // pin is LOW and turn off when the pin is HIGH.
    red.set_high();
    yellow.set_high();
    green.set_high();
}
