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
/// Register addresses for the raw temperature data (Middle significant bits).
const REGISTER_TEMP_LSB: u8 = 0x08;
/// Register addresses for the raw temperature data (Most significant bits).
const REGISTER_TEMP_MSB: u8 = 0x09;

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

    // Read the raw temperature data. There are 2 ways we can do this.
    loop {
        // The first way is to read all 3 bytes in one go. Since the registers
        // are consecutige, the BMP automatically increments the register
        // address after each byte, so we can just read from the first register,
        // and so long as we have more bytes to read, the main (us, the MCU)
        // will send ACK to continue reading. After we read the last byte (as
        // determined by the size of the array), we will send a NACK indicating
        // to the BMP that we are done reading. This will ensure that the
        // temperature data is consistent.
        let mut raw_temp_data = [0u8; 3];
        i2c.write_read(BMP390_ADDR, &[REGISTER_TEMP_XLSB], &mut raw_temp_data)
            .await
            .unwrap();

        // Alternatively, we can read each byte one by one. This is more likely
        // to cause inconsistent data, since the BMP may update the temperature
        // data in between our reads, but it is still possible to get the
        // correct data if we are lucky.
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

        // The raw temperature data is a 24-bit signed integer
        let raw_temp: i32 = ((raw_temp_data[2] as i32) << 16)
            | ((raw_temp_data[1] as i32) << 8)
            | (raw_temp_data[0] as i32);
        info!("Raw temperature value: {}", raw_temp);

        Timer::after_millis(400).await;
    }
}
