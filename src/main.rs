#![no_std]
#![no_main]

#[rtic::app(device=esp32c3, dispatchers=[FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use core::cell::RefCell;

    use embedded_graphics::{
        geometry::AnchorPoint,
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Alignment, Text},
    };
    use embedded_hal_bus::i2c::CriticalSectionDevice;
    use esp_backtrace as _; // Panic handling
    use esp_hal::{
        clock::CpuClock,
        delay::Delay,
        gpio::{Input, Level, Output, Pull},
        i2c::master::{Config, I2c},
        timer::{
            timg::{TimerGroup, TimerX},
            PeriodicTimer,
        },
        Blocking,
    };
    use esp_println::println;
    use fugit::ExtU64;
    use fugit::RateExtU32;
    use humidity_monitor::{init_display, next_brightness, FONT2_NORMAL, TEXT_STYLE_BIG};
    use ssd1306::{
        mode::BufferedGraphicsMode,
        prelude::{Brightness, I2CInterface},
        size::DisplaySize128x64,
        Ssd1306,
    };
    use static_cell::StaticCell;

    use rtic_monotonics::{esp32c3::prelude::*, esp32c3_systimer_monotonic};
    use u8g2_fonts::types::{FontColor::Transparent, HorizontalAlignment, VerticalPosition};
    // esp32c3_systimer_monotonic!(Mono);

    #[shared]
    struct Shared {
        display: Ssd1306<
            I2CInterface<CriticalSectionDevice<'static, I2c<'static, Blocking>>>,
            DisplaySize128x64,
            BufferedGraphicsMode<DisplaySize128x64>,
        >,
    }
    #[local]
    struct Local {
        delay: Delay,
        timer: PeriodicTimer<
            'static,
            esp_hal::timer::timg::Timer<TimerX<esp_hal::peripherals::TIMG0>, Blocking>,
        >,
        aht: aht10_embedded::AHT10<CriticalSectionDevice<'static, I2c<'static, Blocking>>>,
        led: Output<'static>,
        // brightness: Brightness,
    }
    static I2C0: StaticCell<critical_section::Mutex<RefCell<I2c<Blocking>>>> = StaticCell::new();

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        println!("init");

        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::Clock80MHz;
        let peripherals = esp_hal::init(config);

        let delay = Delay::new();

        let i2c_dev = I2c::new(
            peripherals.I2C0,
            Config {
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

        let display = init_display(i2c_cs);

        let led = Output::new(peripherals.GPIO10, Level::Low);

        // let mut button = Input::new(peripherals.GPIO9, Pull::Up);
        // button.listen(esp_hal::gpio::Event::RisingEdge);

        // Mono::start(cx.device.SYSTIMER);
        // let brightness = Brightness::NORMAL;

        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let mut timer0 = PeriodicTimer::new(timg0.timer0);
        timer0.enable_interrupt(true);
        timer0.start(1.secs()).unwrap();

        (
            Shared { display },
            Local {
                delay,
                timer: timer0,
                aht,
                led,
                // brightness,
            },
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            esp_hal::riscv::asm::wfi();
        }
    }

    // #[task(binds=GPIO, shared=[button, display], local=[brightness])]
    // fn button_handler(mut cx: button_handler::Context) {
    //     cx.shared.button.lock(|button| {
    //         button.unlisten();
    //         button.clear_interrupt();
    //         println!("Button");
    //     });

    //     cx.shared.display.lock(|display| {
    //         let b = next_brightness(cx.local.brightness);
    //         println!("Setting display brightness to {:?}", b);
    //         display
    //             .set_brightness(b)
    //             .expect("Failed to set display brightness");
    //         *cx.local.brightness = b;
    //     });

    //     enable_button_interrupt::spawn().expect("Failed to spawn task to enable button interrupt");
    // }

    // #[task(priority = 1, shared=[button])]
    // async fn enable_button_interrupt(mut cx: enable_button_interrupt::Context) {
    //     println!("waiting to enable button interrupt");
    //     Mono::delay(200u64.millis()).await;
    //     cx.shared
    //         .button
    //         .lock(|button| button.listen(esp_hal::gpio::Event::RisingEdge));
    //     println!("Button interrupt enabled");
    // }

    #[task(binds=TG0_T0_LEVEL,shared=[display], local=[timer, aht, delay, led])]
    fn timer_handler(mut cx: timer_handler::Context) {
        cx.local.timer.clear_interrupt();
        // let display = cx.local.display;
        cx.shared.display.lock(|display| {
            display
                .clear(BinaryColor::Off)
                .expect("Failed to clear screen");

            match cx.local.aht.read_data(cx.local.delay) {
                Ok(data) => {
                    let tmp = data.temperature_celsius();
                    let hum = data.humidity();
                    println!("Temp {:.3} - Humidity {:.3}", tmp, hum);

                    render_temperature(display, tmp);
                    render_humidity(display, hum);

                    if hum > 30.0 {
                        cx.local.led.set_high();
                    } else {
                        cx.local.led.set_low();
                    }
                }

                Err(e) => {
                    println!("Failed to read sensor : {e:?}");
                    Text::with_alignment(
                        "Failed to\nread sensor",
                        display.bounding_box().anchor_point(AnchorPoint::Center),
                        TEXT_STYLE_BIG,
                        Alignment::Center,
                    )
                    .draw(display)
                    .expect("Failed to draw text on screen");
                    cx.local.led.set_high();
                }
            }
            display.flush().expect("Failed to flush display");
        })
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
}
