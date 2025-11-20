//! MPU 6500 SPI driver that uses the SPI Bus.
//!
//! This module exports three drivers
//! - MPU6500 async SPI Bus driver
//! - MPU6500 async SPI Device driver
//! - MPU6500 blocking SPI Device driver
//!
//! It defines several data structures used by all the drivers.

pub mod bus;
pub mod device;
pub mod device_blocking;

/// WHO_AM_I Register Address
const WHO_AM_I: u8 = 0x75;

/// WHO_AM_I Register Value for the MPU6500 sensor
const WHO_AM_I_VALUE: u8 = 0x70;

/// The gravitational acceleration
const G: f32 = 9.80665;

/// The register address that the [`bus::Mpu6500::write_config`]
/// function should read
///
/// Instead of using numbers we defined this as an enum to
/// make sure that users cannot use any other values
/// than these.
///
/// This is represented as a `u8` so that it can be cast
/// to a `u8` using the `as` keyword
///
/// [`Copy`] and [`Clone`] are derived so that the value
/// can be copied when sent as a parameter to a function.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ConfigRegister {
    Gyro = 0x1b,
    Accel = 0x1c,
}

/// The register address that the [`bus::Mpu6500::read_value`]
/// function should read
///
/// Instead of using numbers we defined this as an enum to
/// make sure that users cannot use any other values
/// than these.
///
/// This is represented as a `u8` so that it can be cast
/// to a `u8` using the `as` keyword
///
/// [`Copy`] and [`Clone`] are derived so that the value
/// can be copied when sent as a parameter to a function.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ValueRegister {
    AccelXOutH = 0x3b,
    GyroXOutH = 0x43,
}

/// The possible values for the `GYRO_FS_SEL` field of
/// the `GYRO_CONFIG` register.
///
/// Instead of using numbers we defined this as an enum to
/// make sure that users cannot use any other values
/// than these.
///
/// This is represented as a `u8` so that it can be cast
/// to a `u8` using the `as` keyword
///
/// [`Copy`] and [`Clone`] are derived so that the value
/// can be copied when sent as a parameter to a function.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum GyroScale {
    Gs250 = 0b00,
    Gs500 = 0b01,
    Gs1000 = 0b10,
    Gs2000 = 0b11,
}

impl GyroScale {
    /// Returns the absolute maximum value in deg/s for
    /// each scale value.
    pub fn value(&self) -> f32 {
        match self {
            GyroScale::Gs250 => 250f32,
            GyroScale::Gs500 => 500f32,
            GyroScale::Gs1000 => 1000f32,
            GyroScale::Gs2000 => 2000f32,
        }
    }
}

/// The possible values for the `ACCEL_FS_SEL` field of
/// the `ACCEL_CONFIG` register.
///
/// Instead of using numbers we defined this as an enum to
/// make sure that users cannot use any other values
/// than these.
///
/// This is represented as a `u8` so that it can be cast
/// to a `u8` using the `as` keyword
///
/// [`Copy`] and [`Clone`] are derived so that the value
/// can be copied when sent as a parameter to a function.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum AccelScale {
    G2 = 0b00,
    G4 = 0b01,
    G8 = 0b10,
    G16 = 0b11,
}

impl AccelScale {
    /// Returns the absolute maximum value in m/s^2 for
    /// each scale value.
    pub fn value(&self) -> f32 {
        match self {
            AccelScale::G2 => 2f32 * G,
            AccelScale::G4 => 4f32 * G,
            AccelScale::G8 => 8f32 * G,
            AccelScale::G16 => 16f32 * G,
        }
    }
}

/// Stores the acceleration on all the three axes.
pub struct Acceleration {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Stores the gyro values on all the three axes.
pub struct Gyro {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
