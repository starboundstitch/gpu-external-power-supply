#![no_main]
#![no_std]

use cortex_m_rt::entry;

use stm32f4xx_hal::{self as hal, gpio::PinState, i2c::I2c, pac, prelude::*};

use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use usb_device::prelude::*;

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
use usbd_serial::embedded_io::{ReadReady, WriteReady};
use vrm_controller::TPSC536C7;
mod navigation;
mod vrm_controller;

static mut EP_MEMORY: [u32; 1024] = [0; 1024];

#[entry]
fn main() -> ! {
    defmt::info!("System Starting");

    //** Microcontroller Configuration **//
    let dp = pac::Peripherals::take().expect("Cannot Take Peripherals");
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .require_pll48clk()
        .freeze();

    //** Declare GPIOs **//
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();
    let gpiod = dp.GPIOD.split();
    let mut led = gpioa.pa5.into_push_pull_output();

    //** Enable VRM Controller Pins **//
    let _avr_ready = gpioc.pc1.into_floating_input();
    let _bvr_ready = gpioc.pc2.into_floating_input();
    let _b_enable = gpioc.pc10.into_push_pull_output_in_state(PinState::High);

    //** Enable Onboard Control Pins **//
    let up = gpioc.pc6.into_pull_up_input();
    let down = gpiob.pb12.into_pull_up_input();
    let enter = gpiob.pb14.into_pull_up_input();
    let right = gpiob.pb13.into_pull_up_input();
    let left = gpiob.pb15.into_pull_up_input();

    // Read Boot Pins

    //** I2C Configuration **//

    // I2C Channel 1
    let sda1 = gpiob.pb7;
    let scl1 = gpiob.pb6;
    let i2c1 = I2c::new(
        dp.I2C1,
        (scl1, sda1),
        hal::i2c::Mode::standard(400.kHz()),
        &clocks,
    );

    // I2C Channel 2
    let sda2 = gpioc.pc9;
    let scl2 = gpioa.pa8;
    let i2c2 = I2c::new(
        dp.I2C3,
        (scl2, sda2),
        hal::i2c::Mode::standard(400.kHz()),
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

    let usb = USB::new(
        (dp.OTG_FS_GLOBAL, dp.OTG_FS_DEVICE, dp.OTG_FS_PWRCLK),
        (gpioa.pa11, gpioa.pa12),
        &clocks,
    );

    #[allow(static_mut_refs)] // Not My implementation
    let usb_bus = UsbBus::new(usb, unsafe { &mut EP_MEMORY });

    let mut serial = usbd_serial::SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .device_class(usbd_serial::USB_CLASS_CDC)
        .strings(&[StringDescriptors::default()
            .manufacturer("Overclocking Club")
            .product("gpu-external-power-supply")
            .serial_number("Prototype-1")])
        .unwrap()
        .build();

    defmt::info!("USB Initialized");

    // Timer Configuration
    let mut led_time = dp.TIM1.counter_ms(&clocks);
    let mut ui_time = dp.TIM3.counter_ms(&clocks);
    led_time.start(1000.millis()).unwrap();
    ui_time.start(100.millis()).unwrap();

    // Current State of Devices
    let mut nav = Navigation::default();
    let mut dev = navigation::Device::default();
    let mut updated_val: f32 = 10.;

    // Get Initial Values
    update_vrm_read(&mut dev, &mut controller);
    // Enable the device
    controller.ch_b().on_off_config(0x00);
    controller.vout_max().write(1.5);
    controller.ch_a().on_off_config(0x00);
    controller.vout_max().write(1.5);

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
                        defmt::info!("Up");
                        nav.move_up();
                    } else if down.is_low() {
                        defmt::info!("Down");
                        nav.move_down();
                    } else if right.is_low() {
                        defmt::info!("Right");
                        nav.move_right();
                    } else if left.is_low() {
                        defmt::info!("Left");
                        nav.move_left();
                    } else if enter.is_low() {
                        defmt::info!("Enter");
                        if nav.get_position().1 == 2 {
                            continue;
                        }
                        nav.change_mode();

                        // temporary value that is being updated
                        updated_val = match nav.get_position() {
                            (0, 0) => dev.core().get_voltage_setpoint(),
                            (0, 1) => dev.core().get_current_limit(),
                            (0, 2) => dev.core().get_temperature(),
                            (1, 0) => dev.mem().get_voltage_setpoint(),
                            (1, 1) => dev.mem().get_current_limit(),
                            (1, 2) => dev.mem().get_temperature(),
                            (_, _) => 0., // Default condition that sound never match
                        };
                    } else {
                        update_display = false;
                    }
                }
                navigation::Mode::Update => {
                    let (step_small, step_large) = match nav.get_position() {
                        (_, 0) => (0.005, 0.1),
                        (_, 1) => (1., 10.),
                        (_, _) => (0., 0.),
                    };
                    if up.is_low() {
                        updated_val += step_small;
                    } else if down.is_low() {
                        updated_val -= step_small;
                    } else if right.is_low() {
                        updated_val += step_large;
                    } else if left.is_low() {
                        updated_val -= step_large;
                    } else if enter.is_low() {
                        nav.change_mode();

                        // Write Data to I2C and update dev voltages here
                        dev.store_value(nav.get_position(), updated_val);

                        match nav.get_position() {
                            (0, _) => {
                                controller.ch_a();
                            }
                            (1, _) => {
                                controller.ch_b();
                            }
                            (_, _) => (),
                        };
                        match nav.get_position() {
                            (_, 0) => controller.vout_command().write(updated_val),
                            (_, 1) => controller.iout_oc_fault_limit().write(updated_val),
                            (_, _) => (), // Default condition that sound never match
                        };
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
                dev.core().get_voltage(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((0, 1)),
                dev.core().get_current(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((0, 2)),
                dev.core().get_temperature(),
            );
            // Vmem
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((1, 0)),
                dev.mem().get_voltage(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((1, 1)),
                dev.mem().get_current(),
            );
            display_data(
                &mut display,
                text_style,
                fill,
                navigation::translate_point((1, 2)),
                dev.mem().get_temperature(),
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

            // Read new I2C Values (at end so that it has the whole UI time for the values to
            // collect)
            update_vrm_read(&mut dev, &mut controller);

            // USB Handling
            if !usb_dev.poll(&mut [&mut serial]) {
                continue; // This means that the port cannot read or write currently
            }

            // USB to send values to computer
            if serial.write_ready().unwrap() {
                let mut slice = [0u8; 128];
                let length =
                    bincode::encode_into_slice(&dev, &mut slice, bincode::config::standard())
                        .unwrap();

                let slice = &slice[..length];

                let _ = serial.write(slice);
            }
        }

        // If no data to read, don't try read
        if !serial.read_ready().unwrap() {
            continue;
        }

        let mut buf = [0u8; 128];
        let count = match serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                defmt::debug!("USB: Read {} Bytes", count);
                Some(count)
            }
            Err(UsbError::WouldBlock) => {
                defmt::error!("USB: Read Buffer Full");
                None
            }
            Err(err) => {
                defmt::error!("USB: Other Error");
                None
            }
            _ => None,
        };

        if count.is_some() {
            defmt::println!("USB_Data: {}", buf[0..count.unwrap()]);
        }

        // Valid Read
        if count.is_some() {
            // Set Channel
            match buf[0] & 1u8 {
                0 => {
                    // Channel A
                    controller.ch_a();
                }
                1 => {
                    // Channel B
                    controller.ch_b();
                }
                _ => {}
            }
            // Send Command to I2C Device
            match buf[0] & 0x02 {
                0x0 => {
                    // Write
                    controller.command(&buf[1..count.unwrap()]);
                }
                0x2 => {
                    // Read
                }
                _ => {}
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

fn update_vrm_read<I: embedded_hal::i2c::I2c>(
    dev: &mut navigation::Device,
    controller: &mut TPSC536C7<I>,
) {
    // Get Values for Display
    // Voltage
    dev.core().set_voltage(controller.ch_a().read_vout());
    dev.mem().set_voltage(controller.ch_b().read_vout());
    // Temperature
    dev.core()
        .set_temperature(controller.ch_a().read_temperature_1());
    dev.mem()
        .set_temperature(controller.ch_b().read_temperature_1());
    // Current
    dev.core().set_current(controller.ch_a().read_iout());
    dev.mem().set_current(controller.ch_b().read_iout());
    // Voltage Setpoint
    dev.core()
        .set_voltage_setpoint(controller.ch_a().vout_command().read());
    dev.mem()
        .set_voltage_setpoint(controller.ch_b().vout_command().read());
    // Current Limit
    dev.core()
        .set_current_limit(controller.ch_a().iout_oc_fault_limit().read());
    dev.mem()
        .set_current_limit(controller.ch_b().iout_oc_fault_limit().read());
}
