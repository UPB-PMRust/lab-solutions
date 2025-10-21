#![no_std]
#![no_main]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use defmt::{error, info};
use defmt_rtt as _;

// uncomment this to use panic_probe
// make sure you comment the panic_handler
// use panic_probe as _;

#[entry]
fn main() -> ! {
    info!("Device has started");
    panic!("panic here");
    // loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print a panic message using semihosting
    hprintln!("Panic occurred: {:?}", info);

    // Print an error using defmt
    error!("{:?}", info);

    // Enter an infinite loop to halt the system after a panic
    loop {}
}
