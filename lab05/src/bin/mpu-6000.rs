#![no_std]
#![no_main]

use defmt::{error, info, warn}; // Removed `unwrap`
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{Config, Mode, Phase, Polarity, Spi},
    time::Hertz,
};
use embassy_time::{Duration, Timer};
use panic_probe as _;

/// MPU6000 Registers & Constants
const MPU_WHO_AM_I: u8 = 0x75;
const MPU_WHO_AM_I_VALUE: u8 = 0x68; // Expected value for MPU-6000
const MPU_USER_CTRL: u8 = 0x6A;
const MPU_PWR_MGMT_1: u8 = 0x6B;
const MPU_ACCEL_XOUT_H: u8 = 0x3B;
const MPU_READ: u8 = 0x80;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_stm32::init(Default::default());
    info!("Device started");

    // Create the SPI bus configuration
    let mut config = Config::default();
    config.frequency = Hertz(1_000_000);

    // MPU6000 requires SPI Mode 3 (CPOL=1, CPHA=1)
    config.mode = Mode {
        polarity: Polarity::IdleHigh,
        phase: Phase::CaptureOnSecondTransition,
    };

    let mut spi = Spi::new(
        peripherals.SPI1,
        peripherals.PA5, // SCK
        peripherals.PA7, // MOSI
        peripherals.PA6, // MISO
        peripherals.GPDMA1_CH0,
        peripherals.GPDMA1_CH1,
        config,
    );

    // We use the PA8 pin as CS
    let mut cs = Output::new(peripherals.PA8, Level::High, Speed::VeryHigh);

    // Disable I2C interface
    let disable_i2c_buf: [u8; 2] = [MPU_USER_CTRL, 0x10];
    cs.set_low();
    if let Err(e) = spi.write(&disable_i2c_buf).await {
        error!("Failed to disable I2C interface: {:?}", e);
    }
    cs.set_high();

    // Wake up MPU6000
    let wake_buf: [u8; 2] = [MPU_PWR_MGMT_1, 0x01];
    cs.set_low();
    if let Err(e) = spi.write(&wake_buf).await {
        error!("Failed to wake up MPU-6000: {:?}", e);
    }
    cs.set_high();

    // Yield execution to the executor for 100ms to allow the sensor to stabilize
    Timer::after(Duration::from_millis(100)).await;

    // Verify WHO_AM_I
    let command = [MPU_WHO_AM_I | MPU_READ, 0x00];
    let mut rx = [0u8; 2];

    cs.set_low();
    let res = spi.transfer(&mut rx, &command).await;
    cs.set_high();

    match res {
        Err(e) => {
            error!("SPI transfer failed during WHO_AM_I check: {:?}", e);
        }
        Ok(_) => {
            let who_am_i_value = rx[1];
            info!("Sensor's WHO_AM_I register is 0x{:02X}", who_am_i_value);

            if who_am_i_value == MPU_WHO_AM_I_VALUE {
                info!("Sensor is MPU-6000");
            } else {
                warn!("This is not an MPU-6000 sensor. Expected 0x68.");
            }
        }
    }

    // Scaling factors matching the default configured ranges
    // ±2g -> 16384 LSB/g -> multiply by (9.81 / 16384.0) for m/s²
    // ±250°/s -> 131 LSB/°/s -> multiply by (250.0 / 32768.0) for °/s
    let accel_scale: f32 = 9.81 / 16384.0;
    let gyro_scale: f32 = 250.0 / 32768.0;

    // Continuous Data Reading Loop
    loop {
        let mut tx_buf = [0u8; 15];
        let mut rx_buf = [0u8; 15];
        tx_buf[0] = MPU_ACCEL_XOUT_H | MPU_READ;

        cs.set_low();
        let transfer_result = spi.transfer(&mut rx_buf, &tx_buf).await;
        cs.set_high();

        match transfer_result {
            Ok(_) => {
                // Parse raw 16-bit signed values (big-endian) skipping the first dummy byte
                let ax_raw = i16::from_be_bytes([rx_buf[1], rx_buf[2]]) as f32;
                let ay_raw = i16::from_be_bytes([rx_buf[3], rx_buf[4]]) as f32;
                let az_raw = i16::from_be_bytes([rx_buf[5], rx_buf[6]]) as f32;

                let gx_raw = i16::from_be_bytes([rx_buf[9], rx_buf[10]]) as f32;
                let gy_raw = i16::from_be_bytes([rx_buf[11], rx_buf[12]]) as f32;
                let gz_raw = i16::from_be_bytes([rx_buf[13], rx_buf[14]]) as f32;

                // Apply scaling
                let ax = ax_raw * accel_scale;
                let ay = ay_raw * accel_scale;
                let az = az_raw * accel_scale;
                let gx = gx_raw * gyro_scale;
                let gy = gy_raw * gyro_scale;
                let gz = gz_raw * gyro_scale;

                info!("Accel (m/s²): X={}, Y={}, Z={}", ax, ay, az);
                info!("Gyro  (°/s):  X={}, Y={}, Z={}", gx, gy, gz);
            }
            Err(e) => {
                // If a read fails, we just log it and the loop will try again after the delay
                warn!("Failed to read from sensor: {:?}", e);
            }
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}
