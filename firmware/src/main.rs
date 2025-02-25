#![no_main]
#![no_std]

use cortex_m_rt::entry;
use nb::block;

use stm32c0xx_hal::prelude::*;
use stm32c0xx_hal::{i2c::Config, rcc, stm32};

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use defmt;
use defmt_rtt as _;

use panic_semihosting as _; // Sends Backtraces through Probe-rs

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    //** Declare GPIOs **//
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();

    //** Enable VRM Controller Pins **//
    let _avr_ready = gpioc.pc6.into_push_pull_output_in_state(PinState::High);
    let _bvr_ready = gpioc.pc7.into_push_pull_output_in_state(PinState::High);
    let _a_enable = gpioc.pc9.into_push_pull_output_in_state(PinState::High);
    let _b_enable = gpioc.pc10.into_push_pull_output_in_state(PinState::High);

    // Read Boot Pins
    defmt::info!("BtSl: {}", dp.FLASH.optr().read().n_boot_sel().bit_is_set());
    defmt::info!("nBoot1: {}", dp.FLASH.optr().read().n_boot1().bit_is_set());
    defmt::info!(
        "Boot_Lock: {}",
        dp.FLASH.secr().read().boot_lock().bit_is_set()
    );

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
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    // Fill
    let fill = PrimitiveStyle::with_fill(BinaryColor::Off);

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

// Clears the Display
fn clear_display<I: embedded_hal::i2c::I2c, D: ssd1306::size::DisplaySize>(
    display: &mut Ssd1306<I2CInterface<I>, D, ssd1306::mode::BufferedGraphicsMode<D>>,
    fill: PrimitiveStyle<BinaryColor>,
) {
    Rectangle::new(Point::new(0, 0), Size::new(128, 64))
        .into_styled(fill)
        .draw(display)
        .unwrap();
}
