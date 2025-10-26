#![no_std]
#![no_main]

use defmt::{debug, info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use panic_probe as _;

/// More Codes for the uppercase English letters
const MORSE: [&str; 26] = [
    ".-",   // A
    "-...", // B
    "-.-.", // C
    "-..",  // D
    ".",    // E
    "..-.", // F
    "--.",  // G
    "....", // H
    "..",   // I
    ".---", // J
    "-.-",  // K
    ".-..", // L
    "--",   // M
    "-.",   // N
    "---",  // O
    ".--.", // P
    "--.-", // Q
    ".-.",  // R
    "...",  // S
    "-",    // T
    "..-",  // U
    "...-", // V
    ".--",  // W
    "-..-", // X
    "-.--", // Y
    "--..", // Z
];

/// Lights up the red LED and turns off the yellow and green LED
///
/// The function uses mutable references to the LEDs, as `set_high` and
/// `set_low` required mutable borrows (references).
///
/// As this is an async function, the `Output` type requires its lifetime
/// to be stated `Output<`_`>`. As it is not relevant for the function,
/// the unknown lifetime `'_` is used here.
async fn display_symbol(leds: &mut [Output<'_>; 3], morse_symbol: char) {
    // The LEDs on the lab board ar active LOW, meaning they turn on when
    // the pin is set to LOW and turn off when the pin is set to HIGH.
    match morse_symbol {
        '.' => {
            leds[0].set_high();
            leds[1].set_low();
            leds[2].set_high();
        }
        '-' => {
            // We want to iterate (take all the elements one at a time)
            // through the array of LEDs, so we use `iter_mut` to
            // get a mutable iterator (`set_low` requires mutable references)
            for led in leds.iter_mut() {
                led.set_low();
            }
        }
        // If the program is correct, this should never execute
        // and it will panic if it tries to execute it.
        _ => panic!("Unknown mores code symbol {}", morse_symbol),
    }
    Timer::after_secs(500).await;

    // Turn off all the LEDs
    //
    // We want to iterate (take all the elements one at a time)
    // through the array of LEDs, so we use `iter_mut` to
    // get a mutable iterator (`set_low` requires mutable references)
    for led in leds.iter_mut() {
        led.set_high();
    }
    Timer::after_millis(500).await;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // The LEDs on the lab board are active LOW, meaning the light up when the pin is LOW
    // and turn off when the pin is LOW. We set the initial value of the pin to HIGH
    // to turn off the LED.

    // The three LEDs are connected to D8 (PC7), D9 (PC6) and D10 (PC9).
    let mut leds = [
        Output::new(peripherals.PC7, Level::High, Speed::Low),
        Output::new(peripherals.PC6, Level::High, Speed::Low),
        Output::new(peripherals.PC9, Level::High, Speed::Low),
    ];

    // The text to display in more code.
    //
    // It contains a space that is not displayable and
    // will generate a `warn` message.
    //
    // This is intentional, so we can test if the messages
    // is displayed.
    let text = "Hello DM";

    // take every letter from the text
    for letter in text.chars() {
        // make it uppercase
        let letter = letter.to_ascii_uppercase();
        // verify if the character is in between A and Z
        if letter > 'A' && letter < 'Z' {
            // We have to compute the position of the morse code in the
            // MORSE array. Position 0 is A, position 1 is B and so on.
            //
            // Characters (char) cannot be subtracted, as they are not
            // simple value, they use UTF-8. ASCII characters are always
            // represented on 7 bits and fit in an `u8` which we can
            // subtract.
            //
            // We convert the letter, that we know is an ASCII character, to
            // `u8` and subtract the `u8` value of A (b`A` means the ASCII
            // representation of A in `u8`).
            //
            // Rust requires arrays to use `usize` indices, so we convert
            // the `u8` position that we have computed to a `usize`.
            let letter_position = (letter as u8 - b'A') as usize;

            // Display every symbol of the MORSE code
            for symbol in MORSE[letter_position].chars() {
                debug!("Displaying {}", letter);
                display_symbol(&mut leds, symbol).await;
            }
        } else {
            // Write a warning if we have a character that we are unable to display
            warn!("Unable to display letter {}", letter);
        }
    }
}
