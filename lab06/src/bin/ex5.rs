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
const AT24C256_ADDR: u8 = 0x50; // Note: A0, A1, A2 are all hooked to GND

const REGISTER_PWR_CTRL: u8 = 0x1B;
const PWR_MODE_ON: u8 = 0b0011_0000;
const PWR_TEMP_EN: u8 = 0b0000_0010;
const PWR_VAL: u8 = PWR_MODE_ON | PWR_TEMP_EN;

const REGISTER_OSR: u8 = 0x1C;
const OSR_TEMP_X2: u8 = 0b0000_1000;
const OSR_VAL: u8 = OSR_TEMP_X2;

const REGISTER_TEMP_XLSB: u8 = 0x07;

const REGISTER_NVM_PAR_T1: u8 = 0x31;

const TEMP_SAVE_ADDRESS: u16 = 0xACDC;

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

    // Read NVM calibration parameters (5 bytes in total)
    let mut nvm_data = [0u8; 5];
    i2c.write_read(BMP390_ADDR, &[REGISTER_NVM_PAR_T1], &mut nvm_data)
        .await
        .unwrap();

    // 0x31 (LSB) & 0x32 (MSB) -> u16
    let nvm_par_t1: u16 = ((nvm_data[1] as u16) << 8) | (nvm_data[0] as u16);
    // 0x33 (LSB) & 0x34 (MSB) -> u16
    let nvm_par_t2: u16 = ((nvm_data[3] as u16) << 8) | (nvm_data[2] as u16);
    // 0x35 -> i8 (Note: This is an 8-bit signed value!)
    let nvm_par_t3: i8 = nvm_data[4] as i8;

    let par_t1 = (nvm_par_t1 as f32) / 0.00390625; // 2^-8
    let par_t2 = (nvm_par_t2 as f32) / 1073741824.0; // 2^30
    let par_t3 = (nvm_par_t3 as f32) / 281474976710656.0; // 2^48

    // Read previous temperature value from EEPROM
    let mut prev_temp_data = [0u8; 4];
    i2c.write_read(
        AT24C256_ADDR,
        &TEMP_SAVE_ADDRESS.to_be_bytes(),
        &mut prev_temp_data,
    )
    .await
    .unwrap();
    let prev_temp_hundredths = i32::from_be_bytes(prev_temp_data);
    let prev_temp = (prev_temp_hundredths as f32) / 100.0;
    info!(
        "Previous temperature value read from EEPROM: {} °C",
        prev_temp
    );

    loop {
        let mut raw_temp_data = [0u8; 3];
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_XLSB], &mut raw_temp_data)
            .await
            .unwrap();

        let raw_temp: i32 = ((raw_temp_data[2] as i32) << 16)
            | ((raw_temp_data[1] as i32) << 8)
            | (raw_temp_data[0] as i32);
        info!("Raw temperature value: {}", raw_temp);

        // Based on Appendix 8.5: Temperature compensation
        // `raw_temp` is the u32 value read from registers 0x07..0x09
        let partial_data1 = (raw_temp as f32) - par_t1;
        let partial_data2 = partial_data1 * par_t2;

        // t_lin is the compensated temperature in degrees Celsius
        let t_lin = partial_data2 + (partial_data1 * partial_data1) * par_t3;

        info!("Compensated temperature value: {} °C", t_lin);

        let hundredths = (t_lin * 100.0) as i32;
        let mut tx_buf = [0u8; 2 + 4];
        tx_buf[..2].copy_from_slice(&TEMP_SAVE_ADDRESS.to_be_bytes());
        tx_buf[2..].copy_from_slice(&hundredths.to_be_bytes());

        i2c.write(AT24C256_ADDR, &tx_buf).await.unwrap();

        Timer::after_millis(400).await;
    }
}
