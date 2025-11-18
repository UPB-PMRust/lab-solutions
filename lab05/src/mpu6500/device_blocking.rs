use embedded_hal::spi::SpiDevice;

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

    pub fn is_connected(&mut self) -> bool {
        let command = [(1 << 7) | WHO_AM_I, 0];
        let mut rx = [0u8; 2];

        let res = self.spi.transfer(&mut rx, &command);
        match res {
            Ok(()) => rx[1] == WHO_AM_I_VALUE,
            Err(_error) => false,
        }
    }

    pub fn set_gyro_scale(&mut self, scale: GyroScale) -> Result<(), S::Error> {
        let command = [GYRO_CONFIG, (scale as u8) << 3];
        let mut rx = [0u8; 2];
        self.spi.transfer(&mut rx, &command)
    }

    pub fn set_accel_scale(&mut self, scale: AccelScale) -> Result<(), S::Error> {
        let command = [ACCEL_CONFIG, (scale as u8) << 3];
        let mut rx = [0u8; 2];
        self.spi.transfer(&mut rx, &command)
    }

    pub fn read_acceleration(&mut self) -> Result<Acceleration, S::Error> {
        let rx = self.read_value(ValueRegister::AccelXOutH)?;
        Ok(Acceleration {
            x: i16::from_le_bytes((&rx[0..2]).try_into().unwrap()),
            y: i16::from_le_bytes((&rx[2..4]).try_into().unwrap()),
            z: i16::from_le_bytes((&rx[4..6]).try_into().unwrap()),
        })
    }

    pub fn read_gyro(&mut self) -> Result<Gyro, S::Error> {
        let rx = self.read_value(ValueRegister::GyroXOutH)?;
        Ok(Gyro {
            x: i16::from_le_bytes((&rx[0..2]).try_into().unwrap()),
            y: i16::from_le_bytes((&rx[2..4]).try_into().unwrap()),
            z: i16::from_le_bytes((&rx[4..6]).try_into().unwrap()),
        })
    }
}

/// Private API
impl<'a, S: SpiDevice> Mpu6500<'a, S> {
    fn read_value(&mut self, value_register: ValueRegister) -> Result<[u8; 6], S::Error> {
        let command = [(1 << 7) | value_register as u8, 0, 0, 0, 0, 0, 0];
        let mut rx = [0u8; 7];
        let res = self.spi.transfer(&mut rx, &command);
        match res {
            Ok(()) => Ok((&command[1..]).try_into().unwrap()),
            Err(error) => Err(error),
        }
    }
}
