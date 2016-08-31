//! Global Descriptor Table
//!
//! The Global Descriptor Table defines the several memory segments utilized by
//! the processor. Memory segmentation was the primary isolation mechanism
//! before paging. Its functionality has largely been deprecated in AMD64, which
//! requires a "flat", non-segmented memory model. However, it is still
//! used to define some security checks between user and system code.

extern {
    // The GDT is actually defined in `src/kernel/arch/x86/boot/boot32.s`.
    pub static mut GDT: [usize; 8];
}

pub const SYS_CODE_OFFSET: usize = 0x08;
pub const SYS_DATA_OFFSET: usize = 0x10;

pub const USR_CODE_OFFSET: usize = 0x18;
pub const USR_DATA_OFFSET: usize = 0x20;

pub const TSS_OFFSET:  usize = 0x30;
