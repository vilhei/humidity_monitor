#![no_std]
#![no_main]

use core::cell::RefCell;

use embedded_hal_bus::i2c::CriticalSectionDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    entry,
    i2c::master::I2c,
    rtc_cntl::{
        sleep::{RtcSleepConfig, TimerWakeupSource},
        Rtc,
    },
    Blocking,
};
use esp_println::println;
use fugit::RateExtU32;
use humidity_monitor::{init_display, FONT2_NORMAL, TEXT_STYLE_BIG};
use static_cell::StaticCell;

use embedded_graphics::{
    geometry::AnchorPoint,
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};
use u8g2_fonts::types::{FontColor::Transparent, HorizontalAlignment, VerticalPosition};

use core::time::Duration;

static I2C0: StaticCell<critical_section::Mutex<RefCell<I2c<Blocking>>>> = StaticCell::new();

#[entry]
fn main() -> ! {
    let mut config = esp_hal::Config::default();
    config.cpu_clock = CpuClock::Clock160MHz;
    let peripherals = esp_hal::init(config);

    let i2c_dev = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config {
            frequency: 200u32.kHz(),
            ..Default::default()
        },
    )
    .with_sda(peripherals.GPIO3)
    .with_scl(peripherals.GPIO4);

    let i2c_cs: &'static critical_section::Mutex<RefCell<I2c<Blocking>>> =
        I2C0.init_with(|| critical_section::Mutex::new(RefCell::new(i2c_dev)));

    let mut aht = aht10_embedded::AHT10::new(CriticalSectionDevice::new(i2c_cs));
    aht.initialize().unwrap();

    let mut delay = Delay::new();
    match aht.read_data(&mut delay) {
        Ok(data) => {
            let tmp = data.temperature_celsius();
            let hum = data.humidity();
            println!("Temp {:.3} - Humidity {:.3}", tmp, hum);
        }

        Err(e) => {
            println!("Failed to read sensor : {e:?}");
        }
    }
    let wakeup_source = TimerWakeupSource::new(Duration::from_secs(5));
    let mut rtc = Rtc::new(peripherals.LPWR);
    let mut config = RtcSleepConfig::deep();
    // config.set_rtc_mem_inf_follow_cpu(false);
    rtc.sleep(&config, &[&wakeup_source]);
    unreachable!();
}

fn render_temperature<D>(display: &mut D, temperature: f32)
where
    D: DrawTarget<Color = BinaryColor, Error: core::fmt::Debug>,
{
    FONT2_NORMAL
        .render_aligned(
            format_args!("{temperature:.1} °C"),
            display.bounding_box().anchor_point(AnchorPoint::CenterLeft) + Point::new(0, -10),
            VerticalPosition::Center,
            HorizontalAlignment::Left,
            Transparent(BinaryColor::On),
            display,
        )
        .expect("Failed to render humidity text");
}
fn render_humidity<D>(display: &mut D, humidity: f32)
where
    D: DrawTarget<Color = BinaryColor, Error: core::fmt::Debug>,
{
    FONT2_NORMAL
        .render_aligned(
            format_args!("{humidity:.1} %"),
            display.bounding_box().anchor_point(AnchorPoint::CenterLeft) + Point::new(0, 20),
            VerticalPosition::Center,
            HorizontalAlignment::Left,
            Transparent(BinaryColor::On),
            display,
        )
        .expect("Failed to render humidity text");
}
