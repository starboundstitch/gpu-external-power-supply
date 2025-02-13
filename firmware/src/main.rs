#![no_main]
#![no_std]

use cortex_m_rt::entry;
use nb::block;
use stm32c0xx_hal::prelude::*;
use stm32c0xx_hal::stm32;

use panic_halt as _;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();

    let mut timer = dp.TIM17.timer(&mut rcc);
    timer.start(500.millis());

    loop {
        led.toggle().unwrap();
        block!(timer.wait()).unwrap();
    }
}
