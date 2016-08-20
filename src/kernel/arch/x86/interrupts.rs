//! Interrupt Handling
//!
//! This module handles the initialisation of the Interrupt Descriptor Table.
//! Several events can cause a CPU Exception. When an exception occurs, the
//! CPU uses the IDT to locate the appropriate interrupt service routine. The
//! ISR is expected to either solve the problem (e.g. load a missing page) or
//! kill the exceptional process.
//!
//! ISRs do not utilize a standard calling convention. They must return using
//! the `iret` instruction. To enforce this, an ISR must be diverging. Thus,
//! the ISR can either `panic!()` or call `isr::iret()`. See the `Isr` type
//! alias.

/// Number of entries to allocate space for in the IDT
pub const IDT_ENTRIES: usize = 256;
/// Number of bytes occupied by the IDT minus 1
pub const IDT_SIZE: u16      = IDT_ENTRIES as u16 * 16 - 1;

/// The correct function prototype of an interrupt service routine
pub type Isr = fn() -> !;

/// Wrapper type of binary representation of an IDT entry
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct IdtEntry {
    ptr_low:  u16,
    selector: u16,
    options:  u16,
    ptr_med:  u16,
    ptr_high: u32,
    reserved: u32,
}

/// A `(size, pointer)` pair pointing at an array of `IdtEntries`
#[repr(packed)]
pub struct Idt{
    size: u16,
    table: &'static mut [IdtEntry; IDT_ENTRIES]
}

impl IdtEntry {
    /// Constructs an entry from a given interrupt service routine
    fn from(isr: Isr) -> IdtEntry {
        let ptr = isr as usize;
        IdtEntry {
            ptr_low:  (ptr & 0xffff) as u16,
            ptr_med:  ((ptr >> 16) & 0xffff) as u16,
            ptr_high: ((ptr >> 32) & 0xffff_ffff) as u32,
            selector: 0x08, // kernel code segment
            options:  0x8e00,
            reserved: 0,
        }
    }
}

impl Idt {
    /// Creates an empty table
    ///
    /// Allocate array on heap, then leak box so it's never freed.
    pub fn new() -> Idt {
        use alloc::boxed::Box;
        let table = box [Default::default(); IDT_ENTRIES];
        Idt {
            size: IDT_SIZE,
            table: unsafe { &mut *Box::into_raw(table) }
        }
    }

    /// Returns the current table
    pub fn current() -> Option<Idt> {
        use core::mem;
        unsafe {
            let mut idt: Idt = mem::uninitialized();
            asm!("sidt [$0]" :: "r"(&mut idt) :: "intel");
            if idt.size != IDT_SIZE {
                // uninitialized IDT
                return None;
            }
            Some(idt)
        }
    }

    /// Registers an interrupt service routine in this table
    pub fn register_isr(&mut self, index: usize, isr: Isr) {
        self.table[index] = IdtEntry::from(isr);
    }

    /// Loads the table into the IDT register
    pub fn load(&self) {
        unsafe { asm!("lidt [$0]" :: "r"(self) :: "intel"); }
    }
}

/// Creates and loads a minimal interrupt descriptor table
pub fn initialize() {
    let mut idt = Idt::new();
    for i in 0..256 {
        idt.register_isr(i, isr::unknown);
    }
    idt.register_isr(0x00, isr::division_by_zero);
    idt.register_isr(0x06, isr::invalid_opcode);
    idt.register_isr(0x08, isr::double_fault);
    idt.register_isr(0x0d, isr::general_protection_fault);
    idt.register_isr(0x0e, isr::page_fault);
    idt.load();
}

/// A collection of interrupt service routines
#[macro_use]
pub mod isr {
    /// Correctly returns from an ISR
    #[inline(always)]
    pub fn iret() -> ! {
        unsafe { asm!("iretq" :::: "volatile"); }
        unreachable!();
    }

    /// Correctly wraps non-destructive interrupt service routines
    ///
    /// # Examples
    /// ```
    /// isr! {
    ///     fn system_timer() {
    ///         println!("system timer");
    ///     }
    ///     fn keyboard_input() {
    ///         println!("keyboard input");
    ///     }
    /// }
    ///
    /// let mut idt = Idt::new();
    /// idt.register_isr(0x20, system_timer);
    /// idt.register_isr(0x21, keyboard_input);
    /// ```
    #[macro_export]
    macro_rules! isr {
        ( $(fn $name:ident () $code:block)*) => (
            $(
                #[naked]
                fn $name() -> ! {
                    fn action() {
                        $code
                    }
                    action();
                    $crate::arch::x86::interrupts::isr::iret();
                }
            )*
        )
    }

    /// Wraps a simple panic message interrupt
    // Doesn't require the `isr!` because we don't need to preserve any state
    // to return to since we panic.
    macro_rules! panic_isr {
        ($name:ident, $msg:expr) => (
            #[naked]
            pub fn $name() -> ! {
                panic!($msg);
            }
        );
    }

    // TODO FIXME panic!()/println!() in an ISR can deadlock

    // generic ISR
    // unfortunately it seems impossible to infer the specific interrupt number
    // for better reporting without creating a routine for each interrupt.
    panic_isr!(unknown, "unknown interrupt");

    // interrupts we'll probably run into soon
    panic_isr!(division_by_zero, "division by zero");
    panic_isr!(invalid_opcode, "invalid opcode");
    panic_isr!(double_fault, "double fault");
    panic_isr!(general_protection_fault, "general protection fault");
    panic_isr!(page_fault, "page fault");
}
