#![no_std]
#![no_main]

use defmt::{debug, info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{self, Spi},
    time::Hertz,
};
use panic_probe as _;

/// WHO_AM_I Register Address
const WHO_AM_I: u8 = 0x75;
/// WHO_AM_I Register Value for the MPU6500 sensor
const WHO_AM_I_VALUE: u8 = 0x70;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // Create the SPI bus configuration
    let mut config = spi::Config::default();
    // Set the SPI frequency to 1 MHz
    config.frequency = Hertz(1_000_000);

    // SPI1 is exposed by the Arduino header using pins:
    // - MISO - D12 (PA6)
    // - MOSI - D11 (PA7)
    // - CLK - D13 (PA5)
    //
    // We use the asynchronous API an we need two free
    // DMA channels. We use GPDMA1_CH0 and GPDMA1_CH1
    let mut spi = Spi::new(
        peripherals.SPI1,
        peripherals.PA5,
        peripherals.PA7,
        peripherals.PA6,
        peripherals.GPDMA1_CH0,
        peripherals.GPDMA1_CH1,
        config,
    );

    // We use the D7 (PA8) pin as CS
    let mut mpu6500_cs_pin = Output::new(peripherals.PA8, Level::High, Speed::Low);

    // This is the buffer that is sent to the sensor. The format is:
    // | R/W REGISTER_ADDRESS | as many zeros as many data bytes we want to read |
    // - R/W is the the most significant bit (first bit):
    //  - 1 - read the register's value from the sensor
    //  - 0 - write a value to the sensor's register
    //
    // We shift 1 with 7 positions obtaining 0b1000_0000 and
    // OR it with the WHO_AM_I register address 0x0111_0101
    // and obtain 0b1111_0101.
    //
    // We add another 0 that will be ignored by the sensor, but
    // is required as the sensor will send us the WHO_AM_I register's
    // value while we transmit this 0.
    let command = [(1 << 7) | WHO_AM_I, 0];

    // This is the receive buffer. It is used to store bytes that the
    // sensor sends. We will ignore the first byte, as that byte is transmitted
    // by the sensor while we transmit the read command. The sensor sends random
    // data in the first byte.
    //
    // The second byte is the value of the WHO_AM_I register.
    let mut rx = [0u8; 2];

    // Start the SPI transmission by setting the CS line LOW.
    mpu6500_cs_pin.set_low();
    // Transfer the data:
    // - send the command buffer (command followed by one zero byte)
    // - receive in the rx buffer (random byte followed by the value of the WHO_AM_I register)
    let res = spi.transfer(&mut rx, &command).await;
    // End the SPI transmission by setting the CS line HIGH.
    mpu6500_cs_pin.set_high();

    // SPI transmissions might return errors. We check if the `spi.transfer` function
    // has returned an error.
    if let Err(error) = res {
        // If we have an error, display it.
        warn!("Failed to connect to the sensor: {}", error);
    } else {
        // If there is no transmission error, we check the returned value.

        // The rx buffer received two bytes:
        // - the first byte is random data as it was received while we transmitted
        // the command
        // - the second byte is the register's value that was received while
        // we transmitted the 0 value
        let who_am_i_value = rx[1];
        debug!("Sensor's WHO_AM_I register is {:x}", who_am_i_value);

        // If the register's value is the one expected,
        // we confirm that the MPU 6500 is connected to
        // the SPI.
        if who_am_i_value == WHO_AM_I_VALUE {
            info!("Sensor is MPU6500");
        } else {
            // If the value is incorrect, the connection is faulty or
            // there might be another sensor connected.
            warn!("This is not an MPU6500 sensor");
        }
    }
}
