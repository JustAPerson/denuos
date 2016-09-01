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

use super::gdt::{SYS_CODE_OFFSET, USR_CODE_OFFSET};
use super::intrinsics::{stmsr, wrmsr};

/// Syscall Target flags
pub const STAR: u64 = (SYS_CODE_OFFSET << 32 | USR_CODE_OFFSET << 48) as u64;
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
// TODO document stack layout
// TODO FIXME preserve registers correctly
#[naked]
unsafe fn syscall_enter() {
    fn action() {
        println!("syscall'd");
    }
    asm! {"
        push rcx
        push r11
        call $0
        pop r11
        pop rcx
        sysret
        " :: "i"(action as usize) :: "intel"
    }
}

/// Performs the intial entry to userspace
///
/// # Initial State
/// | register                  | value              |
/// |---------------------------|--------------------|
/// | `rax`, `rbx`, `rdx`, `rdi`, `rsi`, `rbp`, `r8`, `r9`, `r10`, `r12`, `r13`, `r14`, `r15`| 0 |
/// | `rflags`, `r11`           | `SYSRET_RFLAGS`    |
/// | `rip`, `rcx`              | `target` parameter |
/// | `rsp`                     | `stack` parameter  |
#[naked]
pub fn sysret(target: usize, stack: usize) -> ! {
    unsafe {
        asm! {"
            xor rax, rax
            xor rbx, rbx
            xor rdx, rdx
            xor rdi, rdi
            xor rsi, rsi
            xor rbp, rbp
            xor r8, r8
            xor r9, r9
            xor r10, r10
            xor r12, r12
            xor r13, r13
            xor r14, r14
            xor r15, r15
            sysret
            " :: "{rcx}"(target),"{rsp}"(stack),"{r11}"(SYSRET_RFLAGS) :: "intel"
        }
    }
    loop { } // hint about diverging
}
