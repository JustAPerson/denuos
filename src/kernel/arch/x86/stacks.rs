//! Statically allocated stacks
//!
//! These stacks are referenced in the TSS.
//!
//! It should be noted that `kstart` still utilizes the stack defined in
//! boot/boot32.s. Upon transitioning back from userspace to kernelspace, we
//! begin using the DEFAULT stack.

/// The default stack used by the kernel when transitioning from userspace to
/// kernelspace.
pub static mut DEFAULT: StaticStack = StaticStack::zero();
/// The emergency stack used when handling non-maskable interrupts which can
/// occur during any instruction. We separate this stack to avoid the very
/// slim chance of handling a NMI after loading the userspace stack
/// but just before calling `sysret`.
pub static mut NMI: StaticStack = StaticStack::zero();

/// A byte array which allocates space for a stack
pub struct StaticStack([u8; STACK_SIZE]);
/// The size in bytes of the various kernel stacks
pub const STACK_SIZE: usize = 16 * 1024;

impl StaticStack {
    /// Returns a zero initialized stack
    pub const fn zero() -> StaticStack {
        StaticStack([0; STACK_SIZE])
    }

    /// Returns the starting address of the stack (which traditionally grows down)
    pub fn top(&self) -> usize {
        self as *const _ as usize + STACK_SIZE
    }

    /// Loads the top of the stack into the `rsp` register
    #[inline(always)]
    pub unsafe fn load(&self) {
        asm!("mov rsp, $0" :: "r"(self.top()) :: "intel")
    }
}
