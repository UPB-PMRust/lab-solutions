#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    mode::Async,
    spi::{self, Error, Spi},
    time::Hertz,
};
use embassy_time::Timer;
use panic_probe as _;

const WHO_AM_I: u8 = 0x75;
const GYRO_CONFIG: u8 = 0x1b;
const ACCEL_CONFIG: u8 = 0x1c;

const ACCEL_XOUT_H: u8 = 0x3b;
const GYRO_XOUT_H: u8 = 0x43;

const WHO_AM_I_VALUE: u8 = 0x70;

#[repr(u8)]
#[allow(dead_code)]
enum GyroScale {
    Gs250 = 0b00,
    Gs500 = 0b01,
    Gs1000 = 0b10,
    Gs2000 = 0b11,
}

#[repr(u8)]
#[allow(dead_code)]
enum AccelScale {
    G2 = 0b00,
    G4 = 0b01,
    G8 = 0b10,
    G16 = 0b11,
}

struct Acceleration {
    x: i16,
    y: i16,
    z: i16,
}

struct Gyro {
    x: i16,
    y: i16,
    z: i16,
}

async fn set_gyro_scale(
    spi: &mut Spi<'_, Async>,
    cs: &mut Output<'_>,
    scale: GyroScale,
) -> Result<(), Error> {
    cs.set_low();
    let command = [(1 << 7) | GYRO_CONFIG, (scale as u8) << 3];
    let mut rx = [0u8; 2];
    let res = spi.transfer(&mut rx, &command).await;
    cs.set_high();
    res
}

async fn set_accel_scale(
    spi: &mut Spi<'_, Async>,
    cs: &mut Output<'_>,
    scale: AccelScale,
) -> Result<(), Error> {
    cs.set_low();
    let command = [(1 << 7) | ACCEL_CONFIG, (scale as u8) << 3];
    let mut rx = [0u8; 2];
    let res = spi.transfer(&mut rx, &command).await;
    cs.set_high();
    res
}

async fn read_acceleration(
    spi: &mut Spi<'_, Async>,
    cs: &mut Output<'_>,
) -> Result<Acceleration, Error> {
    cs.set_low();
    let command = [(1 << 7) | ACCEL_XOUT_H, 0, 0, 0, 0, 0, 0];
    let mut rx = [0u8; 7];
    let res = spi.transfer(&mut rx, &command).await;
    cs.set_high();
    match res {
        Ok(()) => Ok(Acceleration {
            x: i16::from_le_bytes([rx[1], rx[2]]),
            y: i16::from_le_bytes([rx[3], rx[4]]),
            z: i16::from_le_bytes([rx[5], rx[6]]),
        }),
        Err(error) => Err(error),
    }
}

async fn read_gyro(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>) -> Result<Gyro, Error> {
    cs.set_low();
    let command = [(1 << 7) | GYRO_XOUT_H, 0, 0, 0, 0, 0, 0];
    let mut rx = [0u8; 7];
    let res = spi.transfer(&mut rx, &command).await;
    cs.set_high();
    match res {
        Ok(()) => Ok(Gyro {
            x: i16::from_le_bytes([rx[1], rx[2]]),
            y: i16::from_le_bytes([rx[3], rx[4]]),
            z: i16::from_le_bytes([rx[5], rx[6]]),
        }),
        Err(error) => Err(error),
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    let mut config = spi::Config::default();
    config.frequency = Hertz(1_000_000);

    let mut spi = Spi::new(
        peripherals.SPI1,
        peripherals.PA5,
        peripherals.PA7,
        peripherals.PA6,
        peripherals.GPDMA1_CH0,
        peripherals.GPDMA1_CH1,
        config,
    );

    let mut mpu6500_cs_pin = Output::new(peripherals.PA8, Level::High, Speed::Low);

    let command = [(1 << 7) | WHO_AM_I, 0];
    let mut rx = [0u8; 2];
    mpu6500_cs_pin.set_low();
    spi.transfer(&mut rx, &command).await.unwrap();
    mpu6500_cs_pin.set_high();
    assert_eq!(rx[1], WHO_AM_I_VALUE);

    set_accel_scale(&mut spi, &mut mpu6500_cs_pin, AccelScale::G2)
        .await
        .expect("Failed to set the acceleration scale");
    set_gyro_scale(&mut spi, &mut mpu6500_cs_pin, GyroScale::Gs1000)
        .await
        .expect("Failed to set the gyro scale");

    loop {
        let acceleration = read_acceleration(&mut spi, &mut mpu6500_cs_pin)
            .await
            .unwrap();
        info!(
            "Acceleration: X {}, Y {}, Z {}",
            acceleration.x, acceleration.y, acceleration.z
        );

        let gyro = read_gyro(&mut spi, &mut mpu6500_cs_pin).await.unwrap();
        info!("Gyro: X {}, Y {}, Z {}", gyro.x, gyro.y, gyro.z);

        Timer::after_millis(100).await;
    }
}
