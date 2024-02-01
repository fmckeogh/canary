#![no_std]
#![no_main]

use {
    aht20_async::Aht20,
    core::fmt::Write,
    defmt::trace,
    defmt_rtt as _,
    embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice,
    embassy_executor::Spawner,
    embassy_nrf::{
        bind_interrupts,
        gpio::{Input, Level, Output, OutputDrive, Pull},
        peripherals::{self, TWISPI0},
        spim::{self, Frequency, Spim, MODE_0},
        twim::{self, Twim},
    },
    embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex},
    embassy_time::{Delay, Duration, Ticker},
    embedded_graphics::{
        mono_font::MonoTextStyle, pixelcolor::BinaryColor::On as Black, prelude::*, text::Text,
    },
    epd_waveshare::{
        epd1in54b::{Epd1in54b, *},
        prelude::{WaveshareDisplay, *},
    },
    panic_probe as _,
    profont::PROFONT_24_POINT,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
   SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
   SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1 => spim::InterruptHandler<peripherals::TWISPI1>;
});

static I2C_BUS: StaticCell<Mutex<ThreadModeRawMutex, Twim<TWISPI0>>> = StaticCell::new();
// static CHANNEL: Channel<ThreadModeRawMutex, (Humidity, Temperature), 32> =
// Channel::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    trace!("starting...");

    let p = embassy_nrf::init(Default::default());

    let i2c = Twim::new(p.TWISPI0, Irqs, p.P0_29, p.P0_28, Default::default());
    let i2c_bus = Mutex::new(i2c);
    let i2c_bus = I2C_BUS.init(i2c_bus);

    let mut aht20 = Aht20::new(I2cDevice::new(i2c_bus), Delay).await.unwrap();
    trace!("initialized aht20");

    {
        // let mut sgp30 = Sgp30::new(I2cDevice::new(i2c_bus), 0x58, Delay);
        // sgp30.init().unwrap();
        // let measurement = sgp30.measure().unwrap(); // called every second
        // for calibration reasons defmt::debug!("CO₂eq parts per
        // million: {}", measurement.co2eq_ppm); defmt::debug!("TVOC
        // parts per billion: {}", measurement.tvoc_ppb);
    }

    let mut config = spim::Config::default();
    config.frequency = Frequency::M4;
    config.mode = MODE_0;
    let mut spi = Spim::new_txonly(p.TWISPI1, Irqs, p.P0_23, p.P0_13, config);
    let cs = Output::new(p.P0_27, Level::Low, OutputDrive::Standard);
    let busy = Input::new(p.P0_11, Pull::None);
    let dc = Output::new(p.P0_12, Level::Low, OutputDrive::Standard);
    let rst = Output::new(p.P0_05, Level::Low, OutputDrive::Standard);
    let mut epd = Epd1in54b::new(&mut spi, cs, busy, dc, rst, &mut Delay).unwrap();
    trace!("initialized epd");

    const INTERVAL: Duration = Duration::from_secs(30);
    let mut ticker = Ticker::every(INTERVAL);

    loop {
        let (humidity, temperature) = aht20.read().await.unwrap();

        let mut buf = ConstBuf::new();
        write!(
            &mut buf,
            "{:.2}°C\n{:.2}%",
            temperature.celsius(),
            humidity.rh()
        )
        .unwrap();

        let mut display = Display1in54b::default();
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Black);

        Text::new(buf.as_str(), Point::new(10, 50), style)
            .draw(&mut display)
            .unwrap();

        epd.wake_up(&mut spi, &mut Delay).unwrap();

        epd.update_and_display_frame(&mut spi, &display.buffer(), &mut Delay)
            .unwrap();

        epd.sleep(&mut spi, &mut Delay).unwrap();

        ticker.next().await;
    }
}

struct ConstBuf {
    buf: [u8; 128],
    pos: usize,
}

impl ConstBuf {
    fn new() -> Self {
        Self {
            buf: [0u8; 128],
            pos: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.pos]).unwrap()
    }
}

impl core::fmt::Write for ConstBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let str_len = s.as_bytes().len();
        self.buf[self.pos..self.pos + str_len].copy_from_slice(s.as_bytes());
        self.pos += str_len;
        Ok(())
    }
}

// #[embassy_executor::task]
// async fn aht20_read(
//     mut aht20: Aht20<I2cDevice<'static, ThreadModeRawMutex, Twim<'static,
// TWISPI0>>, Delay>,     sender: Sender<'static, ThreadModeRawMutex, (Humidity,
// Temperature), 32>, ) {
//     const INTERVAL: Duration = Duration::from_secs(1);

//     let mut ticker = Ticker::every(INTERVAL);
//     loop {
//         let (humidity, temperature) = aht20.read().await.unwrap();
//         trace!("{}°C, {}%", temperature.celsius(), humidity.rh());
//         sender.send((humidity, temperature)).await;
//         ticker.next().await;
//     }
// }

// async fn sgp30_read() {
// }
