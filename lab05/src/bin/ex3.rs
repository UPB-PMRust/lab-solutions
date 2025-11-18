#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{self, Spi},
    time::Hertz,
};
use embassy_time::Timer;
use panic_probe as _;

/// WHO_AM_I Register Address
const WHO_AM_I: u8 = 0x75;
/// GYRO_CONFIG Register Address
const GYRO_CONFIG: u8 = 0x1b;
/// ACCEL_CONFIG Register Address
const ACCEL_CONFIG: u8 = 0x1c;

/// ACCEL_XOUT_H Register Address
///
/// This register stores the high value (most significant 8 bits
/// of the acceleration).
const ACCEL_XOUT_H: u8 = 0x3b;

/// GYRO_XOUT_H Register Address
///
/// This register stores the high value (most significant 8 bits
/// of the gyro).
const GYRO_XOUT_H: u8 = 0x43;

/// WHO_AM_I Register Value for the MPU6500 sensor
const WHO_AM_I_VALUE: u8 = 0x70;

/// The acceleration scale value for ±2g
const ACCEL_SCALE_2G: u8 = 0b00;

/// The gyro scale value for ±1000 deg / s
const GYRO_SCALE_1000: u8 = 0b10;

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
    // - R/W is the the first bit:
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

    // Verify that the MPU 6500 sensor is connected.

    // Start the SPI transmission by setting the CS line LOW.
    mpu6500_cs_pin.set_low();
    // Transfer the data:
    // - send the command buffer (command followed by one zero byte)
    // - receive in the rx buffer (random byte followed by the value of the WHO_AM_I register)
    //
    // Using the `unwrap` function will generate a panic if the `transfer` function fails.
    // This is a quick and dirty trick that is not recommended in production firmware,
    // but works for our example. If this happens in production, the firmware
    // should gracefully fail.
    spi.transfer(&mut rx, &command).await.unwrap();
    // End the SPI transmission by setting the CS line HIGH.
    mpu6500_cs_pin.set_high();
    // Panic if the WHO_AM_I register value if wrong
    // This is a quick and dirty trick that is not recommended in production firmware,
    // but works for our example. If this happens in production, the firmware
    // should gracefully fail.
    assert_eq!(rx[1], WHO_AM_I_VALUE);

    // Set the acceleration scale configuration

    // This is the buffer that is sent to the sensor. The format is:
    // | R/W REGISTER_ADDRESS | as many zeros as many data bytes we want to read |
    // - R/W is the the first bit:
    //  - 1 - read the register's value from the sensor
    //  - 0 - write a value to the sensor's register
    //
    // We shift 1 with 7 positions obtaining 0b1000_0000 and
    // negate it to obtain 0b0111_1111. We need to make sure
    // that bit 7 is 0 as we are performing a write. We
    // AND this value with the ACCEL_CONFIG register address 0x0001_1011
    // and obtain 0x0001_1011.
    //
    // This was actually not required as the ACCEL_CONFIG's
    // most significant bit was already 0.
    //
    // The second position of the command buffer is the value that
    // we want to write to the ACCEL_CONFIG register. In our case
    // we are only interested in the ACCEL_FS_SEL field which
    // is 2 bits long starting at bit position 3.
    //
    // As all the other fields of the ACCEL_CONFIG register are 0,
    // all that we have to do is to shift the
    // ACCEL_SCALE_2G value 3 positions to the left.
    let command = [!(1 << 7) & ACCEL_CONFIG, ACCEL_SCALE_2G << 3];
    // Even though we do not read any values form the sensor, we have to
    // supply an rx buffer with the same length as the command buffer.
    // The sensor will send us random data, but we use DMA and
    // DMA will want to transfer some data to us, regardless if it
    // is useful data or not.
    let mut rx = [0u8; 2];
    // Start the SPI transmission by setting the CS line LOW.
    mpu6500_cs_pin.set_low();
    // Transfer the data:
    // - send the command buffer (command followed by the register's new value)
    // - receive in the rx buffer random bytes
    //
    // Using the `unwrap` function will generate a panic if the `transfer` function fails.
    // This is a quick and dirty trick that is not recommended in production firmware,
    // but works for our example. If this happens in production, the firmware
    // should gracefully fail.
    spi.transfer(&mut rx, &command).await.unwrap();
    // End the SPI transmission by setting the CS line HIGH.
    mpu6500_cs_pin.set_high();

    // Set the gyro scale configuration

    // This is the buffer that is sent to the sensor. The format is:
    // | R/W REGISTER_ADDRESS | bytes that we want to write |
    // - R/W is the the first bit:
    //  - 1 - read the register's value from the sensor
    //  - 0 - write a value to the sensor's register
    //
    // We shift 1 with 7 positions obtaining 0b1000_0000 and
    // negate it to obtain 0b0111_1111. We need to make sure
    // that bit 7 is 0 as we are performing a write. We
    // AND this value with the GYRO_CONFIG register address 0x0001_1100
    // and obtain 0x0001_1100.
    //
    // This was actually not required as the GYRO_CONFIG's
    // most significant bit was already 0.
    //
    // The second position of the command buffer is the value that
    // we want to write to the GYRO_CONFIG register. In our case
    // we are only interested in the GYRO_FS_SEL field which
    // is 2 bits long starting at bit position 3.
    //
    // As all the other fields of the GYRO_CONFIG register are 0,
    // all that we have to do is to shift the
    // GYRO_SCALE_1000 value 3 positions to the left.
    let command = [GYRO_CONFIG, GYRO_SCALE_1000 << 3];
    // Even though we do not read any values form the sensor, we have to
    // supply an rx buffer with the same length as the command buffer.
    // The sensor will send us random data, but we use DMA and
    // DMA will want to transfer some data to us, regardless if it
    // is useful data or not.
    let mut rx = [0u8; 2];
    // Start the SPI transmission by setting the CS line LOW.
    mpu6500_cs_pin.set_low();
    // Transfer the data:
    // - send the command buffer (command followed by the register's new value)
    // - receive in the rx buffer random bytes
    //
    // Using the `unwrap` function will generate a panic if the `transfer` function fails.
    // This is a quick and dirty trick that is not recommended in production firmware,
    // but works for our example. If this happens in production, the firmware
    // should gracefully fail.
    spi.transfer(&mut rx, &command).await.unwrap();
    // End the SPI transmission by setting the CS line HIGH.
    mpu6500_cs_pin.set_high();

    loop {
        // Read the acceleration

        // This is the buffer that is sent to the sensor. The format is:
        // | R/W REGISTER_ADDRESS | as many zeros as many data bytes we want to read |
        // - R/W is the the first bit:
        //  - 1 - read the register's value from the sensor
        //  - 0 - write a value to the sensor's register
        //
        // We shift 1 with 7 positions obtaining 0b1000_0000 and
        // OR it with the ACCEL_XOUT_H register address 0x0011_1011
        // and obtain 0b1011_1011.
        //
        // We add six 0s that will be ignored by the sensor, but
        // are required as the sensor will send us the ACCEL_XOUT_H register's
        // value followed by the values of the next 5 registers:
        // - ACCEL_XOUT_L at 0x3c
        // - ACCEL_YOUT_H at 0x3d
        // - ACCEL_YOUT_L at 0x3e
        // - ACCEL_ZOUT_H at 0x3f
        // - ACCEL_ZOUT_L at 0x40
        //
        // Most sensors work like this. When reading or writing, the register
        // in the command is the first register. Every other value that is read
        // or written is to or from the following registers.
        let command = [(1 << 7) | ACCEL_XOUT_H, 0, 0, 0, 0, 0, 0];

        // This is the receive buffer. It is used to store bytes that the
        // sensor sends. We will ignore the first byte, as that byte is transmitted
        // by the sensor while we transmit the read command. The sensor sends random
        // data in the first byte.
        //
        // Bytes 1..6 store the values of the ACCEL_XOUT_H, ACCEL_YOUT_H, ACCEL_YOUT_L,
        // ACCEL_ZOUT_H and ACCEL_ZOUT_L registers.
        let mut rx = [0u8; 7];

        // Start the SPI transmission by setting the CS line LOW.
        mpu6500_cs_pin.set_low();
        // Transfer the data:
        // - send the command buffer (command followed by six zero bytes)
        // - receive in the rx buffer (random byte followed by the values of
        //   the ACCEL_XOUT_H, ACCEL_YOUT_H, ACCEL_YOUT_L,
        //   ACCEL_ZOUT_H and ACCEL_ZOUT_L registers).
        //
        // Using the `unwrap` function will generate a panic if the `transfer` function fails.
        // This is a quick and dirty trick that is not recommended in production firmware,
        // but works for our example. If this happens in production, the firmware
        // should gracefully fail.
        spi.transfer(&mut rx, &command).await.unwrap();

        // End the SPI transmission by setting the CS line HIGH.
        mpu6500_cs_pin.set_high();
        info!("Acceleration Values: {:?}", &rx[1..]);

        // Read the gyro

        // This is the buffer that is sent to the sensor. The format is:
        // | R/W REGISTER_ADDRESS | as many zeros as many data bytes we want to read |
        // - R/W is the the first bit:
        //  - 1 - read the register's value from the sensor
        //  - 0 - write a value to the sensor's register
        //
        // We shift 1 with 7 positions obtaining 0b1000_0000 and
        // OR it with the GYRO_XOUT_H register address 0x0100_0011
        // and obtain 0b1100_0011.
        //
        // We add six 0s that will be ignored by the sensor, but
        // are required as the sensor will send us the GYRO_XOUT_H register's
        // value followed by the values of the next 5 registers:
        // - GYROL_XOUT_L at 0x44
        // - GYROL_YOUT_H at 0x45
        // - GYROL_YOUT_L at 0x46
        // - GYROL_ZOUT_H at 0x47
        // - GYROL_ZOUT_L at 0x48
        //
        // Most sensors work like this. When reading or writing, the register
        // in the command is the first register. Every other value that is read
        // or written is to or from the following registers.
        let command = [(1 << 7) | GYRO_XOUT_H, 0, 0, 0, 0, 0, 0];

        // This is the receive buffer. It is used to store bytes that the
        // sensor sends. We will ignore the first byte, as that byte is transmitted
        // by the sensor while we transmit the read command. The sensor sends random
        // data in the first byte.
        //
        // Bytes 1..6 store the values of the GYRO_XOUT_H, GYRO_YOUT_H, GYRO_YOUT_L,
        // GYRO_ZOUT_H and GYRO_ZOUT_L registers.
        let mut rx = [0u8; 7];

        // Start the SPI transmission by setting the CS line LOW.
        mpu6500_cs_pin.set_low();

        // Transfer the data:
        // - send the command buffer (command followed by six zero bytes)
        // - receive in the rx buffer (random byte followed by the values of
        //   the GYRO_XOUT_H, GYRO_YOUT_H, GYRO_YOUT_L,
        //   GYRO_ZOUT_H and GYRO_ZOUT_L registers).
        //
        // Using the `unwrap` function will generate a panic if the `transfer` function fails.
        // This is a quick and dirty trick that is not recommended in production firmware,
        // but works for our example. If this happens in production, the firmware
        // should gracefully fail.
        spi.transfer(&mut rx, &command).await.unwrap();

        // End the SPI transmission by setting the CS line HIGH.
        mpu6500_cs_pin.set_high();
        info!("Gyro Values: {:?}", &rx[1..]);

        Timer::after_millis(100).await;
    }
}
