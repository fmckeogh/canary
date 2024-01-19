#![no_std]
#![no_main]

use {
    aht20_async::Aht20,
    defmt_rtt as _,
    embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice,
    embassy_executor::Spawner,
    embassy_nrf::{
        bind_interrupts,
        gpio::{Level, Output, OutputDrive},
        peripherals::{self, TWISPI0},
        twim::{self, Twim},
    },
    embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex},
    embassy_time::{Delay, Timer},
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, Twim<TWISPI0>>> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut led = Output::new(p.P0_14, Level::Low, OutputDrive::Standard);

    let i2c = Twim::new(p.TWISPI0, Irqs, p.P0_29, p.P0_28, Default::default());
    let i2c_bus = Mutex::new(i2c);
    let i2c_bus = I2C_BUS.init(i2c_bus);

    let mut aht20 = Aht20::new(I2cDevice::new(i2c_bus), Delay).await.unwrap();

    // let mut sgp30 = Sgp30::new(I2cDevice::new(i2c_bus), 0x58, Delay);
    // sgp30.init().unwrap();
    // let measurement = sgp30.measure().unwrap(); // called every second for calibration reasons
    // defmt::debug!("CO₂eq parts per million: {}", measurement.co2eq_ppm);
    // defmt::debug!("TVOC parts per billion: {}", measurement.tvoc_ppb);

    loop {
        led.toggle();
        Timer::after_millis(1000).await;
        let (humidity, temperature) = aht20.read().await.unwrap();
        defmt::debug!("{}°C, {}%", temperature.celsius(), humidity.rh());
    }
}

// epd1in54b
