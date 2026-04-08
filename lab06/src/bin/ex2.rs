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

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

// Note: A0 must be hooked to low, otherwise the address is 0x77
const BMP390_ADDR: u8 = 0x76;

const REGISTER_PWR_CTRL: u8 = 0x1B;
const PWR_MODE_ON: u8 = 0b0011_0000;
const PWR_TEMP_EN: u8 = 0b0000_0010;
const PWR_VAL: u8 = PWR_MODE_ON | PWR_TEMP_EN;

const REGISTER_OSR: u8 = 0x1C;
const OSR_TEMP_X2: u8 = 0b0000_1000;
const OSR_VAL: u8 = OSR_TEMP_X2;

const REGISTER_TEMP_XLSB: u8 = 0x07;
const REGISTER_TEMP_LSB: u8 = 0x08;
const REGISTER_TEMP_MSB: u8 = 0x09;

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

    i2c.write(BMP390_ADDR, &[REGISTER_PWR_CTRL, PWR_VAL])
        .await
        .unwrap();

    i2c.write(BMP390_ADDR, &[REGISTER_OSR, OSR_VAL])
        .await
        .unwrap();

    loop {
        let mut raw_temp_data = [0u8; 3];
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_XLSB], &mut raw_temp_data)
            .await
            .unwrap();

        let mut raw_xlsb = [0u8; 1];
        let mut raw_lsb = [0u8; 1];
        let mut raw_msb = [0u8; 1];
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_XLSB], &mut raw_xlsb)
            .await
            .unwrap();
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_LSB], &mut raw_lsb)
            .await
            .unwrap();
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_MSB], &mut raw_msb)
            .await
            .unwrap();

        info!("Raw data: {:?}", raw_temp_data);
        info!(
            "Raw byte by byte: {:?}",
            [raw_xlsb[0], raw_lsb[0], raw_msb[0]]
        );

        let raw_temp: i32 = ((raw_temp_data[2] as i32) << 16)
            | ((raw_temp_data[1] as i32) << 8)
            | (raw_temp_data[0] as i32);
        info!("Raw temperature value: {}", raw_temp);

        Timer::after_millis(400).await;
    }
}
