#![no_main]
#![no_std]

use cortex_m_rt::entry;
use nb::block;

use navigation::Navigation;
use stm32c0xx_hal::prelude::*;
use stm32c0xx_hal::{i2c::Config, rcc, stm32};

use embedded_graphics::{
    mono_font::{ascii::FONT_9X18, MonoTextStyle, MonoTextStyleBuilder},
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

mod navigation;
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
        .font(&FONT_9X18)
        .text_color(BinaryColor::On)
        .build();
    let text_style_inv = MonoTextStyleBuilder::new()
        .font(&FONT_9X18)
        .text_color(BinaryColor::Off)
        .build();

    // Fill
    let fill = PrimitiveStyle::with_fill(BinaryColor::Off);
    let fill_inv = PrimitiveStyle::with_fill(BinaryColor::On);
    let hollow = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    // GUI Text
    Text::with_baseline("   Vcore Vmem", Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();
    Text::with_baseline("V:", Point::new(0, 16), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();
    Text::with_baseline("A:", Point::new(0, 32), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();
    Text::with_baseline("T:", Point::new(0, 48), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    // Flush Display
    display.flush().unwrap();

    let mut led_time = dp.TIM14.timer(&mut rcc);
    let mut ui_time = dp.TIM17.timer(&mut rcc);
    led_time.start(1000.millis());
    ui_time.start(100.millis());

    // Current State of Devices
    let mut nav = Navigation::default();
    let mut dev = navigation::Device::default();
    let mut updated_val: f32 = 10.;

    loop {
        let mut update_display = true;

        if led_time.wait().is_ok() {
            led.toggle();
        }

        // Code that Runs Periodically
        if ui_time.wait().is_ok() {
            // Button Input
            match nav.get_mode() {
                navigation::Mode::Navigation => {
                    if up.is_low() {
                        nav.move_up();
                    } else if down.is_low() {
                        nav.move_down();
                    } else if right.is_low() {
                        nav.move_right();
                    } else if left.is_low() {
                        nav.move_left();
                    } else if enter.is_low() {
                        if nav.get_position().1 == 2 {
                            continue;
                        }
                        nav.change_mode();

                        // temporary value that is being updated
                        updated_val = match nav.get_position() {
                            (0, 0) => dev.core().voltage(),
                            (0, 1) => dev.core().current(),
                            (0, 2) => dev.core().temperature(),
                            (1, 0) => dev.mem().voltage(),
                            (1, 1) => dev.mem().current(),
                            (1, 2) => dev.mem().temperature(),
                            (_, _) => 0., // Default condition that sound never match
                        };
                    } else {
                        update_display = false;
                    }
                }
                navigation::Mode::Update => {
                    if up.is_low() {
                        updated_val += 0.005;
                    } else if down.is_low() {
                        updated_val -= 0.005;
                    } else if right.is_low() {
                        updated_val += 0.1;
                    } else if left.is_low() {
                        updated_val -= 0.1;
                    } else if enter.is_low() {
                        nav.change_mode();

                        // Write Data to I2C and update dev voltages here
                        dev.store_value(nav.get_position(), updated_val);
                    } else {
                        update_display = false;
                    }
                }
            }


            // Runs only if there is a value to update on the display to save on unnecessary write
            // cycles and full display clears
            // if update_display {
            // Updates the displays for all stored values
            // Vcore
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((0, 0)),
                dev.core().voltage(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((0, 1)),
                dev.core().current(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((0, 2)),
                dev.core().temperature(),
            );
            // Vmem
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((1, 0)),
                dev.mem().voltage(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((1, 1)),
                dev.mem().current(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((1, 2)),
                dev.mem().temperature(),
            );

            // Update Currently Hovered
            match nav.get_mode() {
                navigation::Mode::Navigation => {
                    Rectangle::new(nav.get_point(), Size::new(9 * 5, 16))
                        .into_styled(hollow)
                        .draw(&mut display)
                        .unwrap();
                }
                navigation::Mode::Update => {
                    display_data(
                        &mut display,
                        text_style_inv,
                        fill_inv,
                        nav.get_point(),
                        updated_val,
                    );
                }
            }

            display.flush().unwrap();
        }
    }
}

// Displays a floating point value in the 2x3 grid of values on the main display
fn display_data<I: embedded_hal::i2c::I2c, D: ssd1306::size::DisplaySize>(
    display: &mut Ssd1306<I2CInterface<I>, D, ssd1306::mode::BufferedGraphicsMode<D>>,
    text_style: MonoTextStyle<BinaryColor>,
    fill: PrimitiveStyle<BinaryColor>,
    point: Point,
    val: f32,
) {
    Rectangle::new(point, Size::new(9 * 5, 16))
        .into_styled(fill)
        .draw(display)
        .unwrap();

    // Parse Float Values
    let mut buf = [b'0'; BUFFER_SIZE];
    let num_chars = lexical_core::write(val, &mut buf);
    let mut num_chars: usize = num_chars.len().into();
    if num_chars > 5 {
        num_chars = 5;
    }

    // Write to Display
    Text::with_baseline(
        unsafe { core::str::from_utf8_unchecked(&(buf[..num_chars])) },
        point,
        text_style,
        Baseline::Top,
    )
    .draw(display)
    .unwrap();
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
