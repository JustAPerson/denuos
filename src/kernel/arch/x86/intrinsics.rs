//! Special Instruction Intrinsics
//!
//! The x86 architecture includes a wide variety of specialized instructions
//! which may be rather inconvenient to use directly. Here wrappers are provide
//! for instructions that are useful to many areas of the kernel.  Instructions
//! specific to a single subsystem are better left safely wrapped in the
//! relevant modules.

/// Transmits byte to port
#[inline(always)]
pub fn outb(port: u16, data: u8) {
    unsafe { asm!("out dx, al" :: "{dx}"(port),"{al}"(data) :: "volatile","intel") }
}

/// Receives byte from port
#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let data;
    unsafe { asm!("in al, dx" : "={al}"(data) : "{dx}"(port) :: "volatile","intel") }
    data
}
