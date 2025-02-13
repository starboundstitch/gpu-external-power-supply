#![no_main]
#![no_std]

use cortex_m_rt::entry;
use nb::block;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use stm32c0xx_hal::prelude::*;
use stm32c0xx_hal::{i2c::Config, stm32};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

use panic_halt as _;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();
    //** Display Configuration (I2C Channel 2) **//
    let sda = gpioa.pa6.into_open_drain_output_in_state(PinState::High);
    let scl = gpioa.pa7.into_open_drain_output_in_state(PinState::High);
    let i2c = dp.I2C2.i2c(sda, scl, Config::new(400.kHz()), &mut rcc);

    // I2C interface
    let interface = I2CDisplayInterface::new(i2c);

    // Configure the display
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    // Font and text color from the embedded_graphics library
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Write Text to the display
    Text::with_baseline("Nyan~Pasu", Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    // Flush Display
    display.flush().unwrap();

    let mut timer = dp.TIM17.timer(&mut rcc);
    timer.start(500.millis());

    loop {
        led.toggle();
        block!(timer.wait()).unwrap();
    }
}
