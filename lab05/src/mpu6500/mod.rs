pub mod bus;
pub mod device;
pub mod device_blocking;

/// WHO_AM_I Register Address
const WHO_AM_I: u8 = 0x75;
/// GYRO_CONFIG Register Address
const GYRO_CONFIG: u8 = 0x1b;
/// ACCEL_CONFIG Register Address
const ACCEL_CONFIG: u8 = 0x1c;

/// WHO_AM_I Register Value for the MPU6500 sensor
const WHO_AM_I_VALUE: u8 = 0x70;

/// The register address that the [`Mpu6500::read_value`]
/// function should read
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
/// This is represented as a `u8` so that it can be cast
/// to a `u8` using the `as` keyword
#[repr(u8)]
pub enum GyroScale {
    Gs250 = 0b00,
    Gs500 = 0b01,
    Gs1000 = 0b10,
    Gs2000 = 0b11,
}

/// The possible values for the `ACCEL_FS_SEL` field of
/// the `ACCEL_CONFIG` register.
///
/// This is represented as a `u8` so that it can be cast
/// to a `u8` using the `as` keyword
#[repr(u8)]
pub enum AccelScale {
    G2 = 0b00,
    G4 = 0b01,
    G8 = 0b10,
    G16 = 0b11,
}

/// Stores the acceleration on all the three axes.
pub struct Acceleration {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

/// Stores the gyro values on all the three axes.
pub struct Gyro {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}
