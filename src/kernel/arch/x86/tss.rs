//! Task State Segment
//!
//! When transitioning back to ring0, the processor must load a stack for the
//! kernel to use. This stack pointer is defined in the Task State Segment.
//!
//! Upon an interrupt or syscall the Task Register is read, which is used as an
//! offset into the GDT to find the TSS and use the rsp0 field as the kernel
//! stack.
//!
//! The TSS used to hold registers and other fields to facilitate hardware task
//! switching, but that's deprecated in AMD64.

use super::gdt::{GDT, TSS_OFFSET};
use super::stacks::{DEFAULT, NMI};

/// A wrapper around a Task State Segment
#[allow(dead_code)]
#[repr(packed)]
pub struct Tss {
    _reserved0: u32,
    rsp0:       usize,
    rsp1:       usize,
    rsp2:       usize,
    _reserved1: u32,
    _reserved2: u32,
    ist1:       usize,
    ist2:       usize,
    ist3:       usize,
    ist4:       usize,
    ist5:       usize,
    ist6:       usize,
    ist7:       usize,
    _reserved3: u32,
    _reserved4: u32,
    _reserved5: u16,
    io_map:     u16,
}

pub static mut TSS: Tss = Tss {
    _reserved0: 0,
    rsp0:       0,
    rsp1:       0,
    rsp2:       0,
    _reserved1: 0,
    _reserved2: 0,
    ist1:       0,
    ist2:       0,
    ist3:       0,
    ist4:       0,
    ist5:       0,
    ist6:       0,
    ist7:       0,
    _reserved3: 0,
    _reserved4: 0,
    _reserved5: 0,
    io_map:     0,
};

/// Initializes the TSS and TR
///
/// Necessary to re-enter ring0
pub fn initialize() {
    // GDT[6..8] contains the TSS segment.
    // It's already been initialized with the proper size and flags, but
    // we initialize the multi-part address fields here since we can't
    // manipulate the tss ptr before linking.
    unsafe {
        TSS.rsp0 = DEFAULT.top();
        TSS.ist1 = NMI.top();

        let tss_ptr = &TSS as *const _ as usize;
        GDT[6] |= (tss_ptr & 0x00ffffff) << 16; // 39:16
        GDT[6] |= (tss_ptr & 0xff000000) << 32; // 63:56
        GDT[7] = tss_ptr >> 32; // 95:64

        // load TR with byte-offset into GDT for TSS
        asm!("ltr ax" :: "{rax}"(TSS_OFFSET) :: "intel");
    }
}
