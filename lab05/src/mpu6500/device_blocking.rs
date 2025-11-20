//! MPU 6500 SPI blocking driver that uses an SPI Device of
//! the SPI Bus.
//!
//! This driver requires an SPI device, meaning that the
//! SPI bus is available for other drivers while this
//! driver is not transferring.
//!
//! The driver receives the SPI device that includes the CS
//! pi and is not responsible for actuating the CS pin
//! to enable the SPI device. The [`SpiDevice::transfer`]
//! takes care of activation the CS pin of the device.
//!
//! ## Advantages:
//! - we can use the SPI bus for other devices when this driver
//!   does not transfer any data
//! - we cannot forget the CS pin low and we are sure that the
//!   transmission ends
//!
//! ## Disadvantages
//! - we have to use a Mutex
//! - we cannot perform back to back transmissions, as another device
//!   might get a hold of the SPI bus's Mutex in between
//!   the transactions

// The `embedded_hal` crate exports standard Hardware Abstraction
// Layer (HAL) traits that libraries like `embassy` implement. Drivers
// use these traits instead of the actual implementation of the HALs.

// This allows drivers to function with any type of bus implementation
// library that implements these traits. In our case, we use `embassy`s
// implementation of the SPI bus, but the driver could be used with
// any other library.
use embedded_hal::spi::SpiDevice;

use crate::mpu6500::{
    AccelScale, Acceleration, ConfigRegister, Gyro, GyroScale, ValueRegister, WHO_AM_I,
    WHO_AM_I_VALUE,
};

/// MPU 6500 SPI Device blocking driver
pub struct Mpu6500<'a, S: SpiDevice> {
    /// The SPI device
    spi: &'a mut S,

    /// The configured acceleration scale
    accel_scale: AccelScale,

    /// The configured gyro scale
    gyro_scale: GyroScale,
}

/// Public API
///
/// The functions defined here are exported by the driver.
///
/// The type `S` used by the driver is defined as *any type that
/// implements the `SpiBus` trait*.
impl<'a, S: SpiDevice> Mpu6500<'a, S> {
    /// Create a new MPU6500 SPI device driver instance
    pub fn new(spi: &'a mut S) -> Mpu6500<'a, S> {
        Mpu6500 {
            spi,
            // The default value for the acceleration scale for
            // MPU6500 is 2G
            accel_scale: AccelScale::G2,
            // The default value for the gyro scale for
            // MPU6500 is 250 deg / 2
            gyro_scale: GyroScale::Gs250,
        }
    }

    /// Verifies if the MPU6500 sensor is connected to the bus
    pub fn is_connected(&mut self) -> bool {
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

        // Transfer the data:
        // - send the command buffer (command followed by one zero byte)
        // - receive in the rx buffer (random byte followed by the value of the WHO_AM_I register)
        //
        // We do store the result of the transmission (either OK(()) or Err(error)) and
        // use it to verify if the transmission was successful.
        let res = self.spi.transfer(&mut rx, &command);

        // Verify if the transmission was successful
        match res {
            // The transmission was successful, verify if the read WHO_AM_I value is correct
            Ok(()) => {
                // If the register's value is the one expected,
                // we confirm that the MPU 6500 is connected to
                // the SPI by returning `true` other wise we
                // return `false`.
                rx[1] == WHO_AM_I_VALUE
            }
            // The transmission was not successful, return the error
            Err(_error) => false,
        }
    }

    /// Set the gyro scale
    pub fn set_gyro_scale(&mut self, scale: GyroScale) -> Result<(), S::Error> {
        self.write_config(ConfigRegister::Gyro, (scale as u8) << 3)
    }

    /// Set the acceleration scale
    pub fn set_accel_scale(&mut self, scale: AccelScale) -> Result<(), S::Error> {
        let res = self.write_config(ConfigRegister::Accel, (scale as u8) << 3);
        match res {
            Ok(()) => {
                self.accel_scale = scale;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Read the acceleration
    ///
    /// The function returns either the acceleration value or an error
    pub fn read_acceleration(&mut self) -> Result<Acceleration, S::Error> {
        let rx = self.read_value(ValueRegister::AccelXOutH)?;
        Ok(Acceleration {
            x: self.convert_to_g(i16::from_be_bytes((&rx[0..2]).try_into().unwrap())),
            y: self.convert_to_g(i16::from_be_bytes((&rx[2..4]).try_into().unwrap())),
            z: self.convert_to_g(i16::from_be_bytes((&rx[4..6]).try_into().unwrap())),
        })
    }

    /// Read the gyro
    ///
    /// The function returns either the gyro value or an error
    pub fn read_gyro(&mut self) -> Result<Gyro, S::Error> {
        let rx = self.read_value(ValueRegister::GyroXOutH)?;
        Ok(Gyro {
            x: self.convert_to_deg_s(i16::from_be_bytes((&rx[0..2]).try_into().unwrap())),
            y: self.convert_to_deg_s(i16::from_be_bytes((&rx[2..4]).try_into().unwrap())),
            z: self.convert_to_deg_s(i16::from_be_bytes((&rx[4..6]).try_into().unwrap())),
        })
    }
}

/// Private API
///
/// The functions defined here are not exported by the driver and
/// are only used by the driver itself.
impl<'a, S: SpiDevice> Mpu6500<'a, S> {
    /// Internal function that sets the value of a config register.
    ///
    /// Writing the acceleration and gyro config use the same SPI transfer code,
    /// the only difference being the register address and the value.
    ///
    /// This function is used by `Mpu6500::set_accel_scale` and `Mpu6500::set_gyro_scale`.
    ///
    /// This function returns success or an error.
    fn write_config(&mut self, config_register: ConfigRegister, value: u8) -> Result<(), S::Error> {
        // This is the buffer that is sent to the sensor. The format is:
        // | R/W REGISTER_ADDRESS | as many zeros as many data bytes we want to read |
        // - R/W is the the most significant bit (first bit):
        //  - 1 - read the register's value from the sensor
        //  - 0 - write a value to the sensor's register
        //
        // We shift 1 with 7 positions obtaining 0b1000_0000 and
        // negate it to obtain 0b0111_1111. We need to make sure
        // that bit 7 is 0 as we are performing a write. We
        // AND this value with the register's address.
        //
        // The second position of the command buffer is the value that
        // we want to write to the config register.
        let command = [!(1 << 7) & config_register as u8, value];

        // Even though we do not read any values form the sensor, we have to
        // supply an rx buffer with the same length as the command buffer.
        // The sensor will send us random data, but we use DMA and
        // DMA will want to transfer some data to us, regardless if it
        // is useful data or not.
        let mut rx = [0u8; 2];

        // Transfer the data:
        // - send the command buffer (command followed by the register's new value)
        // - receive in the rx buffer random bytes
        self.spi.transfer(&mut rx, &command)
    }

    /// Internal function that reads six vales from the sensor starting from
    /// the address of the `value_register` provided.
    ///
    /// Reading the acceleration or the gyro uses the same SPI transfer code,
    /// the only difference being the register address where the read
    /// starts.
    ///
    /// This function is used by `Mpu6500::read_acceleration` and `Mpu6500::read_gyro`.
    ///
    /// This function returns the raw array of six value or an error.
    fn read_value(&mut self, value_register: ValueRegister) -> Result<[u8; 6], S::Error> {
        // This is the buffer that is sent to the sensor. The format is:
        // | R/W REGISTER_ADDRESS | as many zeros as many data bytes we want to read |
        // - R/W is the the most significant bit (first bit):
        //  - 1 - read the register's value from the sensor
        //  - 0 - write a value to the sensor's register
        //
        // We shift 1 with 7 positions obtaining 0b1000_0000 and
        // OR it with the register address.
        //
        // We add six 0s that will be ignored by the sensor, but
        // are required as the sensor will send us the ACCEL_XOUT_H register's
        // value followed by the values of the next 5 registers.
        //
        // Most sensors work like this. When reading or writing, the register
        // in the command is the first register. Every other value that is read
        // or written is to or from the following registers.
        let command = [(1 << 7) | value_register as u8, 0, 0, 0, 0, 0, 0];

        // This is the receive buffer. It is used to store bytes that the
        // sensor sends. We will ignore the first byte, as that byte is transmitted
        // by the sensor while we transmit the read command. The sensor sends random
        // data in the first byte.
        //
        // Bytes 1..6 store the values of the read registers.
        let mut rx = [0u8; 7];

        // Transfer the data:
        // - send the command buffer (command followed by six zero bytes)
        // - receive in the rx buffer (random byte followed by the values of
        //   the read registers).
        //
        // We do store the result of the transmission (either OK(()) or Err(error)) and
        // use it to verify if the transmission was successful.
        let res = self.spi.transfer(&mut rx, &command);

        // Verify if the transmission was successful
        match res {
            // The transmission was successful, extract and return the raw values
            Ok(()) => Ok((&rx[1..]).try_into().unwrap()),
            // The transmission was not successful, return the error
            Err(error) => Err(error),
        }
    }

    /// Converts the `u16` acceleration value to m/s^2 using
    /// the configured acceleration scale.
    fn convert_to_g(&self, value: i16) -> f32 {
        // i16::MAX ...... self.accel_scale.value() (2, 4, 8 or 16 x g)
        // value ......... acceleration
        //
        // acceleration = (value x self.accel_scale.value()) / i16::MAX
        (value as f32 * self.accel_scale.value()) / i16::MAX as f32
    }

    /// Converts the `u16` acceleration value to deg/s using
    /// the configured gyro scale.
    fn convert_to_deg_s(&self, value: i16) -> f32 {
        // i16::MAX ...... self.gyro_scale.value() (250, 500, 1000 or 2000 deg/s)
        // value ......... gyro
        //
        // acceleration = (value x self.gyro_scale.value()) / i16::MAX
        (value as f32 * self.gyro_scale.value()) / i16::MAX as f32
    }
}
