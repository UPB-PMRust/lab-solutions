#![no_std]
#![no_main]

use core::{cell::RefCell, fmt::Write};

use defmt::{debug, info, warn};
use defmt_rtt as _;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_stm32::{
    Config,
    gpio::{Level, Output, Speed},
    rcc::{Pll, PllDiv, PllMul, PllPreDiv, PllSource, Sysclk, VoltageScale, mux},
    spi::{self, Spi},
    time::Hertz,
};
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor},
    text::{Text, renderer::CharacterStyle},
};
use mipidsi::{
    interface::SpiInterface,
    models::ST7735s,
    options::{Orientation, Rotation},
};
use panic_probe as _;

// We use the MPU6500 driver that requires a blocking SPI device
use lab05::mpu6500::{AccelScale, GyroScale, device_blocking::Mpu6500};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Using displays means transferring a lot of data. While this
    // works with the default clock settings, it is very slow.
    // We use these lines of code to setup the external oscillator and
    // increase the frequency of the microcontroller to make the
    // display transfer faster.
    let mut config = Config::default();
    config.rcc.hsi = true;
    config.rcc.pll1 = Some(Pll {
        source: PllSource::HSI, // 16 MHz
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL10,
        divp: None,
        divq: None,
        divr: Some(PllDiv::DIV1), // 160 MHz
    });
    config.rcc.sys = Sysclk::PLL1_R;
    config.rcc.voltage_range = VoltageScale::RANGE1;
    config.rcc.mux.iclksel = mux::Iclksel::HSI48; // USB uses ICLK

    let peripherals = embassy_stm32::init(config);
    info!("Device started");

    // screen reset is D2 (PC8)
    let screen_rst = Output::new(peripherals.PC8, Level::Low, Speed::Low);
    // screen dc is D3 (PB3)
    let screen_dc = Output::new(peripherals.PB3, Level::Low, Speed::Low);

    // SPI1 is exposed by the Arduino header using pins:
    // - MISO - D12 (PA6)
    // - MOSI - D11 (PA7)
    // - CLK - D13 (PA5)
    //
    // We need a blocking SPI as the `mipidsi` display drivers require a blocking SPI device.
    let spi = Spi::new_blocking(
        peripherals.SPI1,
        peripherals.PA5,
        peripherals.PA7,
        peripherals.PA6,
        spi::Config::default(),
    );

    // Create a Mutex so that we can safely share the SPI bus between devices.
    //
    // Due to the Mutex, only one device will have access to the bus at a time.
    let spi_bus_mutex: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    // Set up the configuration for the display SPI device.
    let mut screen_spi_config = spi::Config::default();
    screen_spi_config.frequency = Hertz(3_000_000);

    // Use the D4(PB5) pin as the CS for the display.
    let screen_cs = Output::new(peripherals.PB5, Level::High, Speed::Low);

    // Create a SPI device for the mipidsi display driver.
    //
    // The display requires a higher frequency than the MPU6500 sensor.
    let display_spi = SpiDeviceWithConfig::new(&spi_bus_mutex, screen_cs, screen_spi_config);

    // Allocate a buffer for the display data transfer.
    let mut screen_buffer = [0; 4096];

    // Display work in several interfaces, like SPI or I2C. Display drivers are usually written in a way
    // in which they work on top of these interfaces. They accept a `DI` type which abstract the
    // display interface.
    let di = SpiInterface::new(display_spi, screen_dc, &mut screen_buffer);

    // Create an instance of the display driver.
    let mut screen = mipidsi::Builder::new(ST7735s, di)
        .reset_pin(screen_rst)
        .orientation(Orientation::new().rotate(Rotation::Deg180))
        .init(&mut Delay)
        .unwrap();

    // Create a SPI device for the MPU6500 sensor driver.
    //
    // The MPU6500 sensor requires a lower frequency than the display.
    let mut mpu6500_spi_config = spi::Config::default();
    mpu6500_spi_config.frequency = Hertz(1_000_000);

    // We use the D7 (PA8) pin as CS for the MPU6500 sensor.
    let mpu6500_cs_pin = Output::new(peripherals.PA8, Level::High, Speed::Low);

    // Create a SPI device for the MPU6500 driver.
    let mut mpu6500_spi_device =
        SpiDeviceWithConfig::new(&spi_bus_mutex, mpu6500_cs_pin, mpu6500_spi_config);

    // Create an instance of the MPU6500 driver
    let mut mpu6500 = Mpu6500::new(&mut mpu6500_spi_device);

    screen.clear(Rgb565::BLACK).unwrap();
    let mut style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    // We set the background color to print characters with a background, so that we can
    // fully overwrite the previous text. Otherwise, the new text will just be drawn
    // on top of the old text, making it unreadable.
    style.set_background_color(Some(Rgb565::BLACK));

    if mpu6500.is_connected() {
        mpu6500
            .set_accel_scale(AccelScale::G2)
            .expect("Failed to set the acceleration scale");
        mpu6500
            .set_gyro_scale(GyroScale::Gs1000)
            .expect("Failed to set the gyro scale");

        loop {
            let acceleration = mpu6500.read_acceleration().unwrap();
            let gyro = mpu6500.read_gyro().unwrap();

            let mut acceleration_buf = heapless::String::<100>::new();
            core::write!(
                &mut acceleration_buf,
                "Acceleration:\n X {}     \n Y {}     \n Z {}     ",
                acceleration.x,
                acceleration.y,
                acceleration.z
            )
            .unwrap();

            Text::new(&acceleration_buf, Point::new(0, 20), style)
                .draw(&mut screen)
                .unwrap();

            debug!(
                "Acceleration: X {}, Y {}, Z {}",
                acceleration.x, acceleration.y, acceleration.z
            );

            let mut gyro_buf = heapless::String::<100>::new();
            core::write!(
                &mut gyro_buf,
                "Gyro:\n X {}     \n Y {}     \n Z {}     ",
                gyro.x,
                gyro.y,
                gyro.z
            )
            .unwrap();

            Text::new(&gyro_buf, Point::new(0, 120), style)
                .draw(&mut screen)
                .unwrap();

            info!("Gyro: X {}, Y {}, Z {}", gyro.x, gyro.y, gyro.z);
            Timer::after_millis(100).await;
        }
    } else {
        warn!("MPU6500 sensor is not connected.");
    }
}
