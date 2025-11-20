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
use panic_probe as _;

// We use the MPU6500 driver that requires the whole SPI bus
use lab05::mpu6500::{AccelScale, GyroScale, bus::Mpu6500};

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
    let mpu6500_cs_pin = Output::new(peripherals.PA8, Level::High, Speed::Low);

    // Create an instance of the MPU6500 driver
    let mut mpu6500 = Mpu6500::new(&mut spi, mpu6500_cs_pin);

    // Verify that the MPU 6500 sensor is connected.
    if mpu6500.is_connected().await {
        // Set the acceleration scale
        mpu6500
            .set_accel_scale(AccelScale::G2)
            .await
            .expect("Failed to set the acceleration scale");

        // Set the gyro scale
        mpu6500
            .set_gyro_scale(GyroScale::Gs1000)
            .await
            .expect("Failed to set the gyro scale");

        loop {
            // Read the acceleration
            //
            // Using the `unwrap` function will generate a panic if the `read_acceleration` function fails.
            // This is a quick and dirty trick that is not recommended in production firmware,
            // but works for our example. If this happens in production, the firmware
            // should gracefully fail.
            let acceleration = mpu6500.read_acceleration().await.unwrap();
            info!(
                "acceleration: X {}, Y {}, Z {}",
                acceleration.x, acceleration.y, acceleration.z
            );

            // Read the gyro
            //
            // Using the `unwrap` function will generate a panic if the `read_gyro` function fails.
            // This is a quick and dirty trick that is not recommended in production firmware,
            // but works for our example. If this happens in production, the firmware
            // should gracefully fail.
            let gyro = mpu6500.read_gyro().await.unwrap();
            info!("gyro: X {}, Y {}, Z {}", gyro.x, gyro.y, gyro.z);

            Timer::after_millis(100).await;
        }
    } else {
        warn!("MPU6500 sensor is not connected.");
    }
}
