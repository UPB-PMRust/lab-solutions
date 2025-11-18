#![no_std]
#![no_main]

use defmt::{info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{self, Spi},
    time::Hertz,
};
use embassy_time::Timer;
use lab05::mpu6500::{AccelScale, GyroScale, bus::Mpu6500};
use panic_probe as _;

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

    let mpu6500_cs_pin = Output::new(peripherals.PA8, Level::High, Speed::Low);

    let mut mpu6500 = Mpu6500::new(&mut spi, mpu6500_cs_pin);

    if mpu6500.is_connected().await {
        mpu6500
            .set_accel_scale(AccelScale::G2)
            .await
            .expect("Failed to set the acceleration scale");
        mpu6500
            .set_gyro_scale(GyroScale::Gs1000)
            .await
            .expect("Failed to set the gyro scale");

        loop {
            let acceleration = mpu6500.read_acceleration().await.unwrap();
            info!(
                "acceleration: X {}, Y {}, Z {}",
                acceleration.x, acceleration.y, acceleration.z
            );

            let gyro = mpu6500.read_gyro().await.unwrap();
            info!("gyro: X {}, Y {}, Z {}", gyro.x, gyro.y, gyro.z);
            Timer::after_millis(100).await;
        }
    } else {
        warn!("MPU6500 sensor is not connected.");
    }
}
