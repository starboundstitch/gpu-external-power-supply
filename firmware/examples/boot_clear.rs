#![no_main]
#![no_std]

use cortex_m_rt::entry;
use panic_semihosting as _;
use stm32c0xx_hal::prelude::*;
use stm32c0xx_hal::stm32;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let _rcc = dp.RCC.constrain();

    //** Unlock Sequence for Flash Control Register **//
    // Unlock Flash Memory
    dp.FLASH.keyr().write(|w| unsafe { w.bits(0x4567_0123) });
    dp.FLASH.keyr().write(|w| unsafe { w.bits(0xCDEF_89AB) });
    // Unlock Option Bytes
    dp.FLASH.optkeyr().write(|w| unsafe { w.bits(0x0819_2A3B) });
    dp.FLASH.optkeyr().write(|w| unsafe { w.bits(0x4C5D_6E7F) });

    // Modify boot_select
    dp.FLASH.optr().modify(|_, w| w.n_boot_sel().clear_bit());
    while dp.FLASH.sr().read().bsy1().bit_is_set() {}

    // Save Options
    dp.FLASH.cr().modify(|_, w| w.optstrt().set_bit());
    while dp.FLASH.sr().read().bsy1().bit_is_set() {}
    dp.FLASH.cr().modify(|_, w| w.obl_launch().set_bit());

    // Should Reset First
    loop {}
}
