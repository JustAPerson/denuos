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
extern {
    // defined in boot/boot32.rs
    static tss: Tss;
}

/// A wrapper around a Task State Segment
// Unused for now, likely unnecessary to modify
// But here it is for future reference
#[repr(C, packed)]
struct Tss {
    _reserved0: u32,
    // The only relevant field of the TSS is rsp0, which is used to load
    // the kernel stack when transitioning from ring3 -> ring0. rsp0 reuses
    // the same stack (symbol stack_top in boot/boot32.s) as kstart.
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

/// Initializes the TSS and TR
///
/// Necessary to re-enter ring0
pub fn initialize() {
    // GDT[6..8] contains the TSS segment.
    // It's already been initialized with the proper size and flags, but
    // we initialize the multi-part address fields here since we can't
    // manipulate the tss ptr before linking.
    unsafe {
        let tss_ptr = &tss as *const _ as usize;
        GDT[6] |= (tss_ptr & 0x00ffffff) << 16; // 39:16
        GDT[6] |= (tss_ptr & 0xff000000) << 32; // 63:56
        GDT[7] = tss_ptr >> 32; // 95:64

        // load TR with byte-offset into GDT for TSS
        asm!("ltr ax" :: "{rax}"(TSS_OFFSET) :: "intel");
    }
}
