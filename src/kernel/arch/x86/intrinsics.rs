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

/// Reads model-specific register
#[inline(always)]
pub fn rdmsr(register: u32) -> u64 {
    let (hi, lo): (u64, u64);
    unsafe { asm!("rdmsr" : "={eax}"(lo),"={edx}"(hi) : "{ecx}"(register) :: "intel" ) }
    (hi << 32) | lo
}

/// Writes model-specific register
#[inline(always)]
pub fn wrmsr(register: u32, value: u64) {
    let (hi, lo) = (value >> 32, value & 0xffff_ffff);
    unsafe { asm!("wrmsr" :: "{ecx}"(register),"{eax}"(lo),"{edx}"(hi) :: "intel" ) }
}

/// Sets bit in model-specific register
#[inline(always)]
pub fn stmsr(register: u32, offset: usize) {
    let value = rdmsr(register);
    wrmsr(register, value | (1 << offset));
}
