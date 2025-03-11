#![no_main]
#![no_std]

use cortex_m_rt::entry;
use nb::block;

use stm32f4xx_hal::{self as hal, gpio::PinState, i2c::I2c, pac, prelude::*};

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

use navigation::Navigation;
mod navigation;
mod vrm_controller;

#[entry]
fn main() -> ! {
    //** Microcontroller Configuration **//
    let dp = pac::Peripherals::take().expect("Cannot Take Peripherals");
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.MHz()).freeze();

    //** Declare GPIOs **//
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();
    let gpiod = dp.GPIOD.split();
    let mut led = gpioa.pa5.into_push_pull_output();

    //** Enable VRM Controller Pins **//
    let _avr_ready = gpioc.pc6.into_floating_input();
    let _bvr_ready = gpioc.pc7.into_floating_input();
    let _b_enable = gpioc.pc10.into_push_pull_output_in_state(PinState::High);

    //** Enable Onboard Control Pins **//
    let up = gpioc.pc8.into_pull_up_input();
    let down = gpioc.pc5.into_pull_up_input();
    let left = gpiod.pd8.into_pull_up_input();
    let right = gpioa.pa12.into_pull_up_input();
    let enter = gpioa.pa11.into_pull_up_input();

    // Read Boot Pins

    //** I2C Configuration **//

    // I2C Channel 1
    let sda1 = gpiob.pb7;
    let scl1 = gpiob.pb6;
    let i2c1 = I2c::new(
        dp.I2C1,
        (scl1, sda1),
        hal::i2c::Mode::standard(100.kHz()),
        &clocks,
    );

    // I2C Channel 2
    let sda2 = gpioc.pc9;
    let scl2 = gpioa.pa8;
    let i2c2 = I2c::new(
        dp.I2C3,
        (scl2, sda2),
        hal::i2c::Mode::standard(100.kHz()),
        &clocks,
    );

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

    defmt::info!("Past Display Init");

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

    defmt::info!("First Display Flush");

    // Timer Configuration
    let mut led_time = dp.TIM1.counter_ms(&clocks);
    let mut ui_time = dp.TIM3.counter_ms(&clocks);
    led_time.start(1000.millis()).unwrap();
    ui_time.start(100.millis()).unwrap();

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
            if update_display {
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
