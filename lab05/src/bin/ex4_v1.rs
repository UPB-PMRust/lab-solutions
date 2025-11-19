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

const WHO_AM_I: u8 = 0x75;
const GYRO_CONFIG: u8 = 0x1b;
const ACCEL_CONFIG: u8 = 0x1c;

const ACCEL_XOUT_H: u8 = 0x3b;
const GYRO_XOUT_H: u8 = 0x43;

const WHO_AM_I_VALUE: u8 = 0x70;
const ACCEL_SCALE_2G: u8 = 0b00;
const GYRO_SCALE_1000: u8 = 0b10;

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

    mpu6500_cs_pin.set_low();
    let command = [ACCEL_CONFIG, ACCEL_SCALE_2G << 3];
    let mut rx = [0u8; 2];
    spi.transfer(&mut rx, &command).await.unwrap();
    mpu6500_cs_pin.set_high();

    mpu6500_cs_pin.set_low();
    let command = [GYRO_CONFIG, GYRO_SCALE_1000 << 3];
    let mut rx = [0u8; 2];
    spi.transfer(&mut rx, &command).await.unwrap();
    mpu6500_cs_pin.set_high();

    loop {
        mpu6500_cs_pin.set_low();
        let command = [(1 << 7) | ACCEL_XOUT_H, 0, 0, 0, 0, 0, 0];
        let mut rx = [0u8; 7];
        spi.transfer(&mut rx, &command).await.unwrap();
        mpu6500_cs_pin.set_high();
        info!(
            "Acceleration: X {}, Y {}, Z {}",
            i16::from_be_bytes([rx[1], rx[2]]),
            i16::from_be_bytes([rx[3], rx[4]]),
            i16::from_be_bytes([rx[5], rx[6]])
        );

        mpu6500_cs_pin.set_low();
        let command = [(1 << 7) | GYRO_XOUT_H, 0, 0, 0, 0, 0, 0];
        let mut rx = [0u8; 7];
        spi.transfer(&mut rx, &command).await.unwrap();
        mpu6500_cs_pin.set_high();
        info!(
            "Gyro: X {}, Y {}, Z {}",
            i16::from_be_bytes([rx[1], rx[2]]),
            i16::from_be_bytes([rx[3], rx[4]]),
            i16::from_be_bytes([rx[5], rx[6]])
        );

        Timer::after_millis(100).await;
    }
}
