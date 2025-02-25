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

use lexical_core::BUFFER_SIZE;

use defmt;
use defmt_rtt as _;

use panic_semihosting as _; // Sends Backtraces through Probe-rs

mod vrm_controller;

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
    let _avr_ready = gpioc.pc6.into_floating_input();
    let _bvr_ready = gpioc.pc7.into_floating_input();
    let _a_enable = gpioc.pc9.into_push_pull_output_in_state(PinState::High);
    let _b_enable = gpioc.pc10.into_push_pull_output_in_state(PinState::High);

    //** Enable Onboard Control Pins **//
    let up = gpiob.pb4.into_pull_up_input();
    let down = gpiob.pb5.into_pull_up_input();
    let left = gpiob.pb6.into_pull_up_input();
    let right = gpiob.pb7.into_pull_up_input();
    let enter = gpiob.pb8.into_pull_up_input();

    // Read Boot Pins
    defmt::info!("BtSl: {}", dp.FLASH.optr().read().n_boot_sel().bit_is_set());
    defmt::info!("nBoot1: {}", dp.FLASH.optr().read().n_boot1().bit_is_set());
    defmt::info!(
        "Boot_Lock: {}",
        dp.FLASH.secr().read().boot_lock().bit_is_set()
    );

    //** I2C Configuration **//

    // I2C Channel 1
    let sda1 = gpioa.pa10.into_open_drain_output_in_state(PinState::High);
    let scl1 = gpioa.pa9.into_open_drain_output_in_state(PinState::High);
    let mut i2c1 = dp.I2C1.i2c(sda1, scl1, Config::new(400.kHz()), &mut rcc);

    // I2C Channel 2
    let sda2 = gpioa.pa6.into_open_drain_output_in_state(PinState::High);
    let scl2 = gpioa.pa7.into_open_drain_output_in_state(PinState::High);
    let i2c2 = dp.I2C2.i2c(sda2, scl2, Config::new(400.kHz()), &mut rcc);

    //** VRM Controller Initialization **//
    let i2c_addr = 0x5F;
    // Create Controller
    let mut controller = vrm_controller::TPSC536C7::new(i2c1, i2c_addr);
    defmt::info!("Past VRM Controller Init");

    //** Display Configuration **//
    // I2C interface
    let interface = I2CDisplayInterface::new(i2c2);

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

    let mut led_time = dp.TIM14.timer(&mut rcc);
    let mut ui_time = dp.TIM17.timer(&mut rcc);
    led_time.start(1000.millis());
    ui_time.start(50.millis());

    let mut count: i32 = 10;

    loop {
        let mut update_display = false;

        if led_time.wait().is_ok() {
            led.toggle();
        }
        // Code that Runs Periodically
        if ui_time.wait().is_ok() {
            // Button Input
            if up.is_low() {
                count = count + 1;
                update_display = true;
            }
            if down.is_low() {
                count = count - 1;
                update_display = true;
            }

            // Runs only if there is a value to update on the display to save on unnecessary write
            // cycles and full display clears
            if update_display {
                let mut buffer = [b'0'; BUFFER_SIZE];
                let num_chars = lexical_core::write(count, &mut buffer);
                defmt::info!("Num Chars: {}", num_chars);
                let num_chars: usize = (num_chars.len()).into();

                clear_display(&mut display, fill);

                Text::with_baseline(
                    unsafe { core::str::from_utf8_unchecked(&(buffer[..num_chars])) },
                    Point::zero(),
                    text_style,
                    Baseline::Top,
                )
                .draw(&mut display)
                .unwrap();

                display.flush().unwrap();
            }
        }
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
