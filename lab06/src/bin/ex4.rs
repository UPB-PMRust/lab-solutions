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

/// I2C device address for the BMP390. This is the default address when A0 is
/// connected to low. If A0 is connected to high, the address will be 0x77.
///
/// This will likely work even if A0 is not connected at all, in theory this
/// should be a "floating" pin, but in practice for our PM Lab board it will
/// function as if it were connected to low.
const BMP390_ADDR: u8 = 0x76;

/// Register addresses and values for the `PWR_CTRL` register. (Turns on and off
/// measurements and sets the power mode.)
///
/// | Bits | Description                                     |
/// |------|-------------------------------------------------|
/// | 0    | Pressure measurement on/off. 1 = on, 0 = off    |
/// | 1    | Temperature measurement on/off. 1 = on, 0 = off |
/// | 2-3  | Reserved, must be set to 0                      |
/// | 4-5  | Power mode. 00 = sleep, 01/10 = forced (one
///          measurement), 11 = normal                       |
/// | 6-7  | Reserved, must be set to 0                      |
const REGISTER_PWR_CTRL: u8 = 0x1B;
/// Bits to set in the `PWR_CTRL` register to set normal power mode.
const PWR_MODE_ON: u8 = 0b0011_0000;
/// Bits to set in the `PWR_CTRL` register to enable temperature measurement.
const PWR_TEMP_EN: u8 = 0b0000_0010;
/// Value to write to the `PWR_CTRL` register to enable temperature measurement
/// and set normal power mode.
const PWR_VAL: u8 = PWR_MODE_ON | PWR_TEMP_EN;

/// Register addresses and values for the `OSR` register. (Controls how many
/// samples are taken and averaged for each measurement)
///
/// | Bits | Description                                     |
/// |------|-------------------------------------------------|
/// | 0-2  | Pressure oversampling.
///         000 = no oversampling (1 sample),
///         001 = x2 (2 samples),
///         010 = x4 (4 samples),
///         011 = x8 (8 samples),
///         100 = x16 (16 samples),   
///         101 = x32 (32 samples),                          |
/// | 3-5  | Temperature oversampling.
///         000 = no oversampling (1 sample),
///         001 = x2 (2 samples),
///         010 = x4 (4 samples),
///         011 = x8 (8 samples),
///         100 = x16 (16 samples),
///         101 = x32 (32 samples),                          |
/// | 6-7  | Reserved, must be set to 0                      |
const REGISTER_OSR: u8 = 0x1C;
/// Bits to set in the `OSR` register to set temperature oversampling to x2.
const OSR_TEMP_X2: u8 = 0b0000_1000;
/// Value to write to the `OSR` register to set temperature oversampling to x2
/// and pressure oversampling to none.
const OSR_VAL: u8 = OSR_TEMP_X2;

/// Register addresses for the raw temperature data (Least significant bits).
const REGISTER_TEMP_XLSB: u8 = 0x07;
/// Register addresses for the calibration data. The calibration data is  5
/// registers in order. We can read them in one go by starting a sequential read
/// from the first register.
const REGISTER_NVM_PAR_T1: u8 = 0x31;

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

    // Before we can read any data from the sensor, we need to configure the power.
    i2c.write(BMP390_ADDR, &[REGISTER_PWR_CTRL, PWR_VAL])
        .await
        .unwrap();

    // And we also need to configure the oversampling settings.
    i2c.write(BMP390_ADDR, &[REGISTER_OSR, OSR_VAL])
        .await
        .unwrap();

    // Read NVM calibration parameters (5 bytes in total). See ex2 for more
    // details on the sequential read.
    let mut nvm_data = [0u8; 5];
    i2c.write_read(BMP390_ADDR, &[REGISTER_NVM_PAR_T1], &mut nvm_data)
        .await
        .unwrap();

    // Formulas for the partial calibration parameters taken from the datasheet.

    // 0x31 (LSB) & 0x32 (MSB) -> u16
    let nvm_par_t1: u16 = ((nvm_data[1] as u16) << 8) | (nvm_data[0] as u16);
    // 0x33 (LSB) & 0x34 (MSB) -> u16
    let nvm_par_t2: u16 = ((nvm_data[3] as u16) << 8) | (nvm_data[2] as u16);
    // 0x35 -> i8 (Note: This is an 8-bit signed value!)
    let nvm_par_t3: i8 = nvm_data[4] as i8;

    let par_t1 = (nvm_par_t1 as f32) / 0.00390625; // 2^-8
    let par_t2 = (nvm_par_t2 as f32) / 1073741824.0; // 2^30
    let par_t3 = (nvm_par_t3 as f32) / 281474976710656.0; // 2^48

    loop {
        // Read the raw temperature data in one go. (See ex2 for further
        // explanation)
        let mut raw_temp_data = [0u8; 3];
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_XLSB], &mut raw_temp_data)
            .await
            .unwrap();

        // Based on Appendix 8.5: Temperature compensation `raw_temp` is the u32
        // value read from registers 0x07..0x09
        let raw_temp: i32 = ((raw_temp_data[2] as i32) << 16)
            | ((raw_temp_data[1] as i32) << 8)
            | (raw_temp_data[0] as i32);
        info!("Raw temperature value: {}", raw_temp);

        // Formulas taken from datasheet
        let partial_data1 = (raw_temp as f32) - par_t1;
        let partial_data2 = partial_data1 * par_t2;

        // t_lin is the compensated temperature in degrees Celsius
        let t_lin = partial_data2 + (partial_data1 * partial_data1) * par_t3;

        info!("Compensated temperature value: {} °C", t_lin);
        Timer::after_millis(400).await;
    }
}
