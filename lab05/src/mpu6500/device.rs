//! MPU 6500 SPI async driver that uses an SPI Device of
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

// The `embedded_hal_async` crate exports standard async Hardware Abstraction
// Layer (HAL) traits that libraries like `embassy` implement. Drivers
// use these traits instead of the actual implementation of the HALs.

// This allows drivers to function with any type of bus implementation
// library that implements these traits. In our case, we use `embassy`s
// implementation of the SPI bus, but the driver could be used with
// any other library.
use embedded_hal_async::spi::SpiDevice;

use crate::mpu6500::{
    ACCEL_CONFIG, AccelScale, Acceleration, GYRO_CONFIG, Gyro, GyroScale, ValueRegister, WHO_AM_I,
    WHO_AM_I_VALUE,
};

pub struct Mpu6500<'a, S: SpiDevice> {
    spi: &'a mut S,
}

/// Public API
impl<'a, S: SpiDevice> Mpu6500<'a, S> {
    pub fn new(spi: &'a mut S) -> Mpu6500<'a, S> {
        Mpu6500 { spi }
    }

    pub async fn is_connected(&mut self) -> bool {
        let command = [(1 << 7) | WHO_AM_I, 0];
        let mut rx = [0u8; 2];

        let res = self.spi.transfer(&mut rx, &command).await;
        match res {
            Ok(()) => rx[1] == WHO_AM_I_VALUE,
            Err(_error) => false,
        }
    }

    pub async fn set_gyro_scale(&mut self, scale: GyroScale) -> Result<(), S::Error> {
        let command = [GYRO_CONFIG, (scale as u8) << 3];
        let mut rx = [0u8; 2];
        self.spi.transfer(&mut rx, &command).await
    }

    pub async fn set_accel_scale(&mut self, scale: AccelScale) -> Result<(), S::Error> {
        let command = [ACCEL_CONFIG, (scale as u8) << 3];
        let mut rx = [0u8; 2];
        self.spi.transfer(&mut rx, &command).await
    }

    pub async fn read_acceleration(&mut self) -> Result<Acceleration, S::Error> {
        let rx = self.read_value(ValueRegister::AccelXOutH).await?;
        Ok(Acceleration {
            x: i16::from_be_bytes((&rx[0..2]).try_into().unwrap()),
            y: i16::from_be_bytes((&rx[2..4]).try_into().unwrap()),
            z: i16::from_be_bytes((&rx[4..6]).try_into().unwrap()),
        })
    }

    pub async fn read_gyro(&mut self) -> Result<Gyro, S::Error> {
        let rx = self.read_value(ValueRegister::GyroXOutH).await?;
        Ok(Gyro {
            x: i16::from_be_bytes((&rx[0..2]).try_into().unwrap()),
            y: i16::from_be_bytes((&rx[2..4]).try_into().unwrap()),
            z: i16::from_be_bytes((&rx[4..6]).try_into().unwrap()),
        })
    }
}

/// Private API
impl<'a, S: SpiDevice> Mpu6500<'a, S> {
    async fn read_value(&mut self, value_register: ValueRegister) -> Result<[u8; 6], S::Error> {
        let command = [(1 << 7) | value_register as u8, 0, 0, 0, 0, 0, 0];
        let mut rx = [0u8; 7];
        let res = self.spi.transfer(&mut rx, &command).await;
        match res {
            Ok(()) => Ok((&rx[1..]).try_into().unwrap()),
            Err(error) => Err(error),
        }
    }
}
