//! Global Descriptor Table
//!
//! The Global Descriptor Table defines the several memory segments utilized by
//! the processor. Memory segmentation was the primary isolation mechanism
//! before paging. Its functionality has largely been deprecated in AMD64, which
//! requires a "flat", non-segmented memory model. However, it is still
//! used to define some security checks between user and system code.

use self::flags::*;

pub const SYS_CODE_OFFSET: usize = 0x08;
pub const SYS_DATA_OFFSET: usize = 0x10;

pub const USR_CODE_OFFSET: usize = 0x18;
pub const USR_DATA_OFFSET: usize = 0x20;

pub const TSS_OFFSET:  usize = 0x30;

pub mod flags {
    pub const CODE: usize    = 3 << 43;
    pub const DATA: usize    = 2 << 43;
    pub const TSS: usize     = 9 << 40;
    pub const SYS: usize     = 0 << 45;
    pub const USR: usize     = 3 << 45;
    pub const LONG: usize    = 1 << 53;
    pub const PRESENT: usize = 1 << 47;
    pub const WRITE: usize   = 1 << 41;
}

pub type Gdt = [usize; 8];
pub static mut GDT: Gdt = [
    0,
    SYS | CODE | PRESENT | LONG,
    SYS | DATA | PRESENT | WRITE,

    USR | CODE | PRESENT | LONG,
    USR | DATA | PRESENT | WRITE,
    USR | CODE | PRESENT,

    TSS | PRESENT | 104,
    0,
];

/// Initialize new GDT with long mode segments
pub fn initialize() {
    use core::mem::size_of;

    #[allow(dead_code)]
    #[repr(packed)]
    struct GdtPointer {
        size: u16,
        ptr: &'static Gdt,
    }

    unsafe {
        let gdtp = GdtPointer {
            size: size_of::<Gdt>() as u16 - 1,
            ptr: &GDT,
        };
        asm!("lgdt [$0]" :: "r"(&gdtp) :: "intel");
    }
}
