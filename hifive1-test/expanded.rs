#![feature(prelude_import)]
#![no_std]
#![no_main]
#[prelude_import]
use core::prelude::rust_2021::*;
#[macro_use]
extern crate core;
#[macro_use]
extern crate compiler_builtins;
extern crate panic_halt;
use hifive1::hal::prelude::*;
use hifive1::hal::DeviceResources;
use hifive1::{pin, sprintln};
use riscv_rt::entry;
pub mod slic {
    use riscv_slic::swi::InterruptNumber;
    /// Clears all interrupt flags to avoid interruptions of SLIC and HW controller.
    #[inline(always)]
    #[no_mangle]
    pub unsafe fn __slic_clear() {
        exti_clear();
        swi_clear();
    }
    /// Returns the current priority threshold of the SLIC.
    #[inline(always)]
    #[no_mangle]
    pub unsafe fn __slic_get_threshold() -> u8 {
        __SLIC.get_threshold()
    }
    /// Sets the priority threshold of the external interrupt controller and the SLIC.
    #[inline(always)]
    #[no_mangle]
    pub unsafe fn __slic_set_threshold(thresh: u8) {
        exti_set_threshold(thresh);
        __SLIC.set_threshold(thresh);
    }
    /// Returns the interrupt priority of a given software interrupt source.
    #[inline(always)]
    #[no_mangle]
    pub unsafe fn __slic_get_priority(interrupt: u16) -> u8 {
        let interrupt: Interrupt = InterruptNumber::try_from(interrupt).unwrap();
        __SLIC.get_priority(interrupt)
    }
    /// Sets the interrupt priority of a given software interrupt
    /// source in the external interrupt controller and the SLIC.
    #[inline(always)]
    #[no_mangle]
    pub unsafe fn __slic_set_priority(interrupt: u16, priority: u8) {
        let interrupt: Interrupt = InterruptNumber::try_from(interrupt).unwrap();
        __SLIC.set_priority(interrupt, priority);
        if let Ok(exti) = interrupt.try_into() {
            exti_set_priority(exti, priority);
        }
    }
    /// Marks a software interrupt as pending.
    #[inline(always)]
    #[no_mangle]
    pub unsafe fn __slic_pend(interrupt: u16) {
        let interrupt: Interrupt = InterruptNumber::try_from(interrupt).unwrap();
        __SLIC.pend(interrupt);
        if __SLIC.is_ready() {
            swi_set();
        }
    }
    use riscv_slic::exti::PriorityNumber;
    /// Converts an `u8` to the corresponding priority level.
    /// If conversion fails, it returns the highest available priority level.
    #[inline(always)]
    fn saturated_priority(mut priority: u8) -> e310x::Priority {
        if priority > e310x::Priority::MAX_PRIORITY_NUMBER {
            priority = e310x::Priority::MAX_PRIORITY_NUMBER;
        }
        e310x::Priority::try_from(priority).unwrap()
    }
    #[inline(always)]
    unsafe fn exti_clear() {
        let mut plic = e310x::Peripherals::steal().PLIC;
        plic.reset();
    }
    /// Returns the next pending external interrupt according to the PLIC.
    /// If no external interrupts are pending, it returns `None`.
    #[inline(always)]
    fn exti_claim() -> Option<e310x::Interrupt> {
        e310x::PLIC::claim()
    }
    /// Notifies the PLIC that a pending external interrupt as complete.
    /// If the interrupt was not pending, it silently ignores it.
    #[inline(always)]
    fn exti_complete(exti: e310x::Interrupt) {
        e310x::PLIC::complete(exti);
    }
    /// Sets the PLIC threshold to the desired value. If threshold is higher than
    /// the highest priority, it sets the threshold to the highest possible value.
    #[inline(always)]
    unsafe fn exti_set_threshold(threshold: u8) {
        let mut plic = e310x::Peripherals::steal().PLIC;
        plic.set_threshold(saturated_priority(threshold));
    }
    /// Enables the PLIC interrupt source and sets its priority to the desired value.
    /// If priority is higher than the highest priority, it sets it to the highest possible value.
    #[inline(always)]
    unsafe fn exti_set_priority(interrupt: e310x::Interrupt, priority: u8) {
        let mut plic = e310x::Peripherals::steal().PLIC;
        plic.enable_interrupt(interrupt);
        plic.set_priority(interrupt, saturated_priority(priority));
    }
    impl TryFrom<e310x::Interrupt> for Interrupt {
        type Error = e310x::Interrupt;
        fn try_from(value: e310x::Interrupt) -> Result<Self, Self::Error> {
            match value {
                e310x::Interrupt::RTC => Ok(Interrupt::RTC),
                _ => Err(value),
            }
        }
    }
    impl TryFrom<Interrupt> for e310x::Interrupt {
        type Error = Interrupt;
        fn try_from(value: Interrupt) -> Result<Self, Self::Error> {
            match value {
                Interrupt::RTC => Ok(e310x::Interrupt::RTC),
                _ => Err(value),
            }
        }
    }
    extern "C" {
        fn ClearRTC();
    }
    #[no_mangle]
    pub static __CLEAR_EXTERNAL_INTERRUPTS: [unsafe extern "C" fn(); 1usize] = [
        ClearRTC,
    ];
    #[no_mangle]
    #[allow(non_snake_case)]
    pub unsafe fn MachineExternal() {
        if let Some(exti) = unsafe { exti_claim() } {
            let swi: Result<Interrupt, e310x::Interrupt> = exti.try_into();
            match swi {
                Ok(swi) => {
                    __CLEAR_EXTERNAL_INTERRUPTS[swi as usize]();
                    __slic_pend(swi as u16);
                }
                _ => (e310x::__EXTERNAL_INTERRUPTS[exti as usize]._handler)(),
            }
            unsafe { exti_complete(exti) };
        }
    }
    /// Triggers a machine software interrupt via the CLINT peripheral
    #[inline(always)]
    pub unsafe fn swi_set() {
        let clint = e310x::Peripherals::steal().CLINT;
        clint.msip.write(|w| w.bits(0x01));
    }
    /// Clears the Machine Software Interrupt Pending bit via the CLINT peripheral
    #[inline(always)]
    pub unsafe fn swi_clear() {
        let clint = e310x::Peripherals::steal().CLINT;
        clint.msip.write(|w| w.bits(0x00));
    }
    /// Enumeration of software interrupts
    #[repr(u16)]
    pub enum Interrupt {
        RTC = 0,
        SoftLow = 1,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Interrupt {
        #[inline]
        fn clone(&self) -> Interrupt {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Interrupt {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Interrupt {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Interrupt::RTC => "RTC",
                    Interrupt::SoftLow => "SoftLow",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for Interrupt {}
    #[automatically_derived]
    impl ::core::cmp::Eq for Interrupt {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Interrupt {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Interrupt {
        #[inline]
        fn eq(&self, other: &Interrupt) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag == __arg1_tag
        }
    }
    unsafe impl riscv_slic::swi::InterruptNumber for Interrupt {
        const MAX_INTERRUPT_NUMBER: u16 = 2usize as u16 - 1;
        fn number(self) -> u16 {
            self as _
        }
        fn try_from(value: u16) -> Result<Self, u16> {
            match value {
                0 => Ok(Self::RTC),
                1 => Ok(Self::SoftLow),
                _ => Err(value),
            }
        }
    }
    extern "C" {
        fn RTC();
        fn SoftLow();
    }
    #[no_mangle]
    pub static __SOFTWARE_INTERRUPTS: [unsafe extern "C" fn(); 2usize] = [RTC, SoftLow];
    pub static mut __SLIC: riscv_slic::SLIC<2usize> = riscv_slic::SLIC::new();
    #[no_mangle]
    #[allow(non_snake_case)]
    pub unsafe fn MachineSoft() {
        swi_clear();
        while let Some((priority, interrupt)) = __SLIC.pop() {
            riscv_slic::run(priority, || __SOFTWARE_INTERRUPTS[interrupt as usize]());
        }
    }
}
use slic::Interrupt;
/// HW handler for clearing RTC.
/// We must define a ClearX handler for every bypassed HW interrupt
#[allow(non_snake_case)]
#[no_mangle]
unsafe fn ClearRTC() {
    let rtc = DeviceResources::steal().peripherals.RTC;
    let rtccmp = rtc.rtccmp.read().bits();
    ::hifive1::stdout::write_fmt(format_args!("clear RTC (rtccmp = {0})\n", rtccmp));
    rtc.rtccmp.write(|w| w.bits(rtccmp + 65536));
    riscv_slic::pend(Interrupt::SoftLow);
}
/// SW handler for RTC.
/// This task is automatically pended right after executing ClearRTC.
#[allow(non_snake_case)]
#[no_mangle]
unsafe fn RTC() {
    ::hifive1::stdout::write_str("software RTC\n");
}
/// SW handler for SoftLow (low priority task with no HW binding).
#[allow(non_snake_case)]
#[no_mangle]
unsafe fn SoftLow() {
    ::hifive1::stdout::write_str("software SoftLow\n");
}
#[export_name = "main"]
pub fn __risc_v_rt__main() -> ! {
    let dr = DeviceResources::take().unwrap();
    let p = dr.peripherals;
    let pins = dr.pins;
    let clocks = hifive1::clock::configure(p.PRCI, p.AONCLK, 64.mhz().into());
    unsafe {
        riscv_slic::disable();
        riscv_slic::clear_interrupts();
    };
    hifive1::stdout::configure(p.UART0, pins.pin17, pins.pin16, 115_200.bps(), clocks);
    let wdg = p.WDOG;
    wdg.wdogcfg.modify(|_, w| w.enalways().clear_bit());
    unsafe {
        riscv_slic::set_priority(Interrupt::SoftLow, 1);
        riscv_slic::set_priority(Interrupt::RTC, 2);
    }
    let mut rtc = p.RTC.constrain();
    rtc.disable();
    rtc.set_scale(0);
    rtc.set_rtc(0);
    rtc.set_rtccmp(10000);
    rtc.enable();
    unsafe {
        riscv_slic::set_interrupts();
        riscv_slic::enable();
    };
    loop {}
}
