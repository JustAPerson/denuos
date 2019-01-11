//! PCI Drivers

use crate::arch::x86;

pub trait HostBusBridge {
    fn pci_cs_read(&self, bus: u8, device: u8, func: u8, register: u8) -> u32;
    fn pci_cs_write(&self, bus: u8, device: u8, func: u8, register: u8, val: u32);
}

pub struct x86PIO;
pub fn x86_pio_calculate_addr(bus: u8, device: u8, func: u8, register: u8) -> u32 {
    assert!(device < 32);
    assert!(func < 8);
    assert!(register & 0b11 == 0);
    (1u32 << 31)
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((func as u32) << 8)
        | ((register as u32) & !0b11)
}

impl HostBusBridge for x86PIO {
    fn pci_cs_read(&self, bus: u8, device: u8, func: u8, register: u8) -> u32 {
        let addr = x86_pio_calculate_addr(bus, device, func, register);
        x86::intrinsics::outl(0xCF8, addr);
        x86::intrinsics::inl(0xCFC)
    }
    fn pci_cs_write(&self, bus: u8, device: u8, func: u8, register: u8, val: u32) {
        let addr = x86_pio_calculate_addr(bus, device, func, register);
        x86::intrinsics::outl(0xCF8, addr);
        x86::intrinsics::outl(0xCFC, val)
    }
}


