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
use panic_probe as _;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

const LOWEST_I2C_ADDR: u8 = 0x08;
const HIGHEST_I2C_ADDR: u8 = 0x77;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // I2C pins
    let sda = peripherals.PB7;
    let scl = peripherals.PB6;

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

    for addr in LOWEST_I2C_ADDR..=HIGHEST_I2C_ADDR {
        let mut rx_buf = [0x00u8; 1];
        let res = i2c.write_read(addr, &[0x00], &mut rx_buf).await;
        info!(
            "Result for address 0x{:02x}: {:?} | Data: {:?}",
            addr, res, rx_buf
        );
    }

    #[allow(clippy::empty_loop)]
    loop {}
}
