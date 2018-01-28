//! `syscall` / `sysret` instruction handling
//!
//! System calls are the primary mechanism for userspace to interact with the
//! kernel. It is important that system calls are performant. To this end, Intel
//! introduced the `sysenter` and `sysexit` instructions for the Pentium II.
//! These instructions avoid some of the security and correctness checks
//! inherent to other means of entering kernel space, such as interrupts. The
//! AMD64 architecture improved upon these with the `syscall` and `sysret`
//! instructions.
//!
//! The `syscall` instruction transitions to kernelspace (`cpl=0`) whereas the
//! `sysret` instruction transtitions to userspace (`cpl=3`). This is
//! facilitated by several model-specific registers, which record information
//! such as the value to load into kernelspace `rip` register. `syscall` stores
//! `rip` in `rcx` and `rflags` in `r11`. Thus, these registers are never
//! preserved. We must preserve the other registers.
//!
//! Some initialization must be done to enable these instructions. See the
//! `initialize()` function. See the `sysret()` instruction to manually
//! enter userspace.

use super::gdt::{SYS_CODE_OFFSET, USR_SYSC_OFFSET};
use super::intrinsics::{stmsr, wrmsr};
use super::Registers;

/// Syscall Target flags
pub const STAR: u64 = (SYS_CODE_OFFSET << 32 | USR_SYSC_OFFSET << 48) as u64;
/// The address loaded into the `rip` register by `syscall`
pub const LSTAR: unsafe fn() = syscall_enter;
/// The bits of `rflags` register that should be cleared by `syscall`
pub const SFMASK: u64 = 0;

/// Default value for the `rflags` register for the `sysret()` function
///
/// Only the interrupt flag is set, enabling preempting userspace with
/// interrupts such as IRQs.
pub const SYSRET_RFLAGS: usize = 0x200;

/// Enables the `syscall` and `sysret` instructions
pub fn initialize() {
    // set model specific registers
    wrmsr(0xC0000081, STAR);
    wrmsr(0xC0000082, LSTAR as u64);
    wrmsr(0xC0000084, SFMASK);
    // enable syscall instructions in EFER
    stmsr(0xC0000080, 0); // set the SCE bit
}

/// The function called in kernelspace by `syscall`
#[naked]
unsafe fn syscall_enter() {
    fn action(regs: &mut Registers) {
        println!("syscall'd");
    }
    asm!("
    pushq %rsp
    pushq %r11
    pushq %rcx
    sub $$16, %rsp  // skip the 4 bytes of padding
    movw %gs, 10(%rsp)
    movw %fs,  8(%rsp)
    movw %es,  6(%rsp)
    movw %ds,  4(%rsp)
    movw %ss,  2(%rsp)
    movw %cs,  0(%rsp)
    pushq %r15
    pushq %r14
    pushq %r13
    pushq %r12
    pushq %r11
    pushq %r10
    pushq %r9
    pushq %r8
    pushq %rbp
    pushq %rdi
    pushq %rsi
    pushq %rdx
    pushq %rcx
    pushq %rbx
    pushq %rax
    movq %rsp, %rdi // pass register state to function
    callq ${0:c}
    popq %rax
    popq %rbx
    popq %rcx
    popq %rdx
    popq %rsi
    popq %rdi
    popq %rbp
    popq %r8
    popq %r9
    popq %r10
    popq %r11
    popq %r12
    popq %r13
    popq %r14
    popq %r15
    // don't write cs/ss because sysret sets them
    movw  4(%rsp), %ds
    movw  6(%rsp), %es
    movw  8(%rsp), %fs
    movw 10(%rsp), %gs
    add $$16, %rsp  // skip the 4 bytes of padding
    popq %rcx
    popq %r11
    popq %rsp
    sysretq
    " :: "s"(action as u64))
}

pub fn sysret(registers: &Registers) -> ! {
    unsafe {
        asm! ("
        popq %rax
        popq %rbx
        popq %rcx
        popq %rdx
        popq %rsi
        popq %rdi
        popq %rbp
        popq %r8
        popq %r9
        popq %r10
        popq %r11
        popq %r12
        popq %r13
        popq %r14
        popq %r15
        // don't write cs/ss because sysret sets them
        movw  4(%rsp), %ds
        movw  6(%rsp), %es
        movw  8(%rsp), %fs
        movw 10(%rsp), %gs
        add $$16, %rsp  // skip the 4 bytes of padding
        popq %rcx
        popq %r11
        popq %rsp
        sysretq
        " :: "{rsp}"(registers)::"volatile")
    }
    loop { } // hint about diverging
}
