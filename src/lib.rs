#![no_std]

use core::cell::RefCell;

use embedded_graphics::{
    mono_font::{
        ascii::FONT_6X10,
        iso_8859_1::{FONT_10X20, FONT_8X13},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder},
};
use embedded_hal_bus::i2c::CriticalSectionDevice;
use esp_hal::{i2c::master::I2c, Blocking};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::{Brightness, I2CInterface},
    size::DisplaySize128x64,
    I2CDisplayInterface, Ssd1306,
};
use u8g2_fonts::{
    fonts::{u8g2_font_helvR18_tf, u8g2_font_inr21_mf},
    FontRenderer,
};

pub const TEXT_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

pub const TEXT_STYLE_BOLD: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_8X13)
    .text_color(BinaryColor::On)
    .build();
pub const TEXT_STYLE_BIG: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_10X20)
    .text_color(BinaryColor::On)
    .build();

pub const OUTER_RECT_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .fill_color(BinaryColor::Off)
    .build();

pub const FILL_RECT_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .fill_color(BinaryColor::On)
    .build();

pub const FONT1_NORMAL: u8g2_fonts::FontRenderer = FontRenderer::new::<u8g2_font_helvR18_tf>();
pub const FONT1_BOLD: u8g2_fonts::FontRenderer =
    FontRenderer::new::<u8g2_fonts::fonts::u8g2_font_helvB18_tf>();

pub const FONT2_NORMAL: u8g2_fonts::FontRenderer = FontRenderer::new::<u8g2_font_inr21_mf>();

pub fn init_display(
    i2c_cs: &'static critical_section::Mutex<RefCell<I2c<Blocking>>>,
) -> Ssd1306<
    I2CInterface<CriticalSectionDevice<'static, I2c<'static, Blocking>>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
> {
    let interface = I2CDisplayInterface::new(CriticalSectionDevice::new(i2c_cs));
    let mut display = Ssd1306::new(
        interface,
        DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    display.init().expect("Failed to initialize screen");
    display
        .clear(BinaryColor::Off)
        .expect("Failed to clear screen");
    display
        .set_brightness(Brightness::NORMAL)
        .expect("Failed to set screen brightness");
    display.flush().unwrap();
    display
}

pub fn next_brightness(current_brightness: &Brightness) -> Brightness {
    match *current_brightness {
        Brightness::DIMMEST => Brightness::DIM,
        Brightness::DIM => Brightness::NORMAL,
        Brightness::NORMAL => Brightness::BRIGHT,
        Brightness::BRIGHT => Brightness::BRIGHTEST,
        Brightness::BRIGHTEST => Brightness::DIMMEST,
        x => x,
    }
}
