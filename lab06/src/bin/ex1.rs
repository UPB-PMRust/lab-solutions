#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self, I2c},
    peripherals,
};
use embassy_time::Timer;
use panic_probe as _;

// For I2C to work, we need to bind the interrupts to the correct handlers.
bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

/// The lowest I2C address a device can have. Addresses below this are reserved
/// for special purposes.
const LOWEST_I2C_ADDR: u8 = 0x08;
/// The highest I2C address a device can have. Addresses above this are reserved
/// for special purposes.
const HIGHEST_I2C_ADDR: u8 = 0x77;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize the device peripherals
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // Define the I2C pins. In this lab we will use the I2C1 peripheral, which
    // is connected to PB6 (SCL) and PB7 (SDA).
    let scl = peripherals.PB6;
    let sda = peripherals.PB7;

    // I2C definition
    let mut i2c = I2c::new(
        peripherals.I2C1,
        scl,
        sda,
        Irqs,
        peripherals.GPDMA1_CH0,
        peripherals.GPDMA1_CH1,
        Default::default(),
    );

    // Scan the I2C bus for devices. We do this by trying to read from each
    // possible address.

    for addr in LOWEST_I2C_ADDR..=HIGHEST_I2C_ADDR {
        // To execute a "scan", we can either try to execute a read directly, or
        // we can write a dummy register (0x00) and then wait for a response
        //
        // The first approach (read-only) may work with some devices, but it may
        // not work devices which decide to ignore read requests without a
        // preceding write. It will also take longer to get a timeout, which
        // will make the scan take longer to complete.
        //
        // The second approach (write-read) is more likely to work with a wider
        // range of devices, but it is not guaranteed that all devices will have
        // a `0x00` register.
        //
        // For this lab, the BMP390 has its chip ID register at `0x00`, and the
        // AT24C256 has its first memory address at `0x00`.
        let mut rx_buf = [0u8; 1];
        let res = i2c.write_read(addr, &[0x00], &mut rx_buf).await;
        info!(
            "Result for address 0x{:02x}: {:?} | Data: {:?}",
            addr, res, rx_buf
        );
    }

    loop {
        Timer::after_secs(1).await;
    }
}
