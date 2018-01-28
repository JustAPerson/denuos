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
pub type Isr = unsafe fn();

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
        idt.register_isr(i, isr::ISR_UNKNOWN[i]);
    }

    idt.register_isr(0x0e, isr::isr_pf);

    // load rsp with ist1 from TSS. See boot/boot32.s
    // TODO handle MCE/NMI
    // idt.table[0x02].options |= 1;
    // idt.table[0x12].options |= 1;

    idt.load();
}

/// Enables interrupts
pub fn enable() {
    unsafe { asm!("sti") }
}

/// Disables interrupts
pub fn disable() {
    unsafe { asm!("cli") }
}

#[repr(packed)]
pub struct InterruptState {
    pub error:  u32,
    pub vector: u32,
    pub rip:    u64,
    pub cs:     u16,
    _pad1:      u16,
    _pad2:      u32,
    pub rflags: u64,
    pub rsp:    u64,
    pub ss:     u16,
    _pad3:      u16,
    _pad4:      u32,
}

#[inline(always)]
pub unsafe fn entry_error() {
}

#[inline(always)]
pub unsafe fn entry_plain() {
    asm!("pushq $$0" :::: "volatile");
}

/// A collection of interrupt service routines
#[macro_use]
pub mod isr {
    use super::*;

    macro_rules! isr_asm {
        ($vector:expr, $entry:path, $action:path) => {
            // should inline always
            $entry(); // push an error code if necessary

            // TODO reconsider pushing segments if we use %gs
            asm!("
            movl $0, 4(%rsp) // set vector
            movq %rsp, %rdi  // pass InterruptState to action
            callq ${1:c}
            addq $$8, %rsp   // remove error code
            iretq
            " :: "n"($vector), "s"($action as u64) :: "volatile");
        }
    }

    macro_rules! isr_expr {
        ( $name:ident, $vector:expr, $entry:path, $action:path) => {
            {
                #[naked]
                pub unsafe fn $name() {
                    isr_asm!($vector, $entry, $action);
                }
                $name
            }
        }
    }

    macro_rules! isr_action {
        ($entry:path, $name:ident, $vector:expr, $s:ident, $block:block) => {
            #[naked]
            pub unsafe fn $name() {
                fn action($s: &mut $crate::arch::x86::interrupts::InterruptState) {
                    $block
                }

                isr_asm!($vector, $entry, action);
            }
        }
    }

    macro_rules! isr_plain {
        ($($vector:expr => fn $name:ident ($s:ident) $block:block)*) => {$(
            isr_action!($crate::arch::x86::interrupts::entry_plain, $name, $vector, $s, $block);
        )*}
    }

    macro_rules! isr_error {
        ($($vector:expr => fn $name:ident ($s:ident) $block:block)*) => {$(
            isr_action!($crate::arch::x86::interrupts::entry_error, $name, $vector, $s, $block);
        )*}
    }

    isr_error! {
        0x0e => fn isr_pf(state) {
            unsafe {
                let cr2: u64;
                asm!("movq %cr2, %rax" :"={rax}"(cr2)::: );
                println!("int #PF(0x{:x}) cs={:x} rip={:x} ss={:x} rsp={:x} cr2={:x}",
                         state.error, state.cs, state.rip, state.ss, state.rsp, cr2);
            }
        }
    }

    fn isr_unknown(state: &mut InterruptState) {
        unsafe {
            panic!("Unexpected interrupt: {:x}", state.vector)
        }
    }

    pub static ISR_UNKNOWN: [unsafe fn(); 256] = [
        isr_expr!(isr_unknown_0x00, 0x00, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x01, 0x01, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x02, 0x02, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x03, 0x03, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x04, 0x04, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x05, 0x05, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x06, 0x06, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x07, 0x07, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x08, 0x08, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x09, 0x09, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x0a, 0x0a, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x0b, 0x0b, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x0c, 0x0c, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x0d, 0x0d, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x0e, 0x0e, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x0f, 0x0f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x10, 0x10, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x11, 0x11, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x12, 0x12, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x13, 0x13, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x14, 0x14, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x15, 0x15, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x16, 0x16, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x17, 0x17, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x18, 0x18, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x19, 0x19, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x1a, 0x1a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x1b, 0x1b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x1c, 0x1c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x1d, 0x1d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x1e, 0x1e, entry_error, isr_unknown), // error
        isr_expr!(isr_unknown_0x1f, 0x1f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x20, 0x20, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x21, 0x21, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x22, 0x22, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x23, 0x23, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x24, 0x24, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x25, 0x25, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x26, 0x26, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x27, 0x27, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x28, 0x28, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x29, 0x29, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x2a, 0x2a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x2b, 0x2b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x2c, 0x2c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x2d, 0x2d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x2e, 0x2e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x2f, 0x2f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x30, 0x30, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x31, 0x31, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x32, 0x32, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x33, 0x33, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x34, 0x34, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x35, 0x35, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x36, 0x36, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x37, 0x37, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x38, 0x38, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x39, 0x39, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x3a, 0x3a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x3b, 0x3b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x3c, 0x3c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x3d, 0x3d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x3e, 0x3e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x3f, 0x3f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x40, 0x40, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x41, 0x41, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x42, 0x42, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x43, 0x43, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x44, 0x44, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x45, 0x45, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x46, 0x46, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x47, 0x47, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x48, 0x48, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x49, 0x49, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x4a, 0x4a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x4b, 0x4b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x4c, 0x4c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x4d, 0x4d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x4e, 0x4e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x4f, 0x4f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x50, 0x50, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x51, 0x51, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x52, 0x52, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x53, 0x53, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x54, 0x54, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x55, 0x55, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x56, 0x56, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x57, 0x57, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x58, 0x58, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x59, 0x59, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x5a, 0x5a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x5b, 0x5b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x5c, 0x5c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x5d, 0x5d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x5e, 0x5e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x5f, 0x5f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x60, 0x60, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x61, 0x61, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x62, 0x62, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x63, 0x63, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x64, 0x64, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x65, 0x65, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x66, 0x66, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x67, 0x67, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x68, 0x68, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x69, 0x69, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x6a, 0x6a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x6b, 0x6b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x6c, 0x6c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x6d, 0x6d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x6e, 0x6e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x6f, 0x6f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x70, 0x70, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x71, 0x71, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x72, 0x72, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x73, 0x73, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x74, 0x74, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x75, 0x75, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x76, 0x76, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x77, 0x77, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x78, 0x78, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x79, 0x79, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x7a, 0x7a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x7b, 0x7b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x7c, 0x7c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x7d, 0x7d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x7e, 0x7e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x7f, 0x7f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x80, 0x80, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x81, 0x81, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x82, 0x82, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x83, 0x83, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x84, 0x84, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x85, 0x85, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x86, 0x86, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x87, 0x87, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x88, 0x88, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x89, 0x89, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x8a, 0x8a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x8b, 0x8b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x8c, 0x8c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x8d, 0x8d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x8e, 0x8e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x8f, 0x8f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x90, 0x90, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x91, 0x91, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x92, 0x92, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x93, 0x93, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x94, 0x94, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x95, 0x95, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x96, 0x96, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x97, 0x97, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x98, 0x98, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x99, 0x99, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x9a, 0x9a, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x9b, 0x9b, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x9c, 0x9c, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x9d, 0x9d, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x9e, 0x9e, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0x9f, 0x9f, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa0, 0xa0, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa1, 0xa1, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa2, 0xa2, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa3, 0xa3, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa4, 0xa4, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa5, 0xa5, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa6, 0xa6, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa7, 0xa7, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa8, 0xa8, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xa9, 0xa9, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xaa, 0xaa, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xab, 0xab, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xac, 0xac, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xad, 0xad, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xae, 0xae, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xaf, 0xaf, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb0, 0xb0, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb1, 0xb1, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb2, 0xb2, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb3, 0xb3, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb4, 0xb4, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb5, 0xb5, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb6, 0xb6, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb7, 0xb7, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb8, 0xb8, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xb9, 0xb9, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xba, 0xba, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xbb, 0xbb, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xbc, 0xbc, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xbd, 0xbd, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xbe, 0xbe, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xbf, 0xbf, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc0, 0xc0, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc1, 0xc1, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc2, 0xc2, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc3, 0xc3, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc4, 0xc4, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc5, 0xc5, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc6, 0xc6, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc7, 0xc7, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc8, 0xc8, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xc9, 0xc9, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xca, 0xca, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xcb, 0xcb, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xcc, 0xcc, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xcd, 0xcd, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xce, 0xce, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xcf, 0xcf, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd0, 0xd0, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd1, 0xd1, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd2, 0xd2, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd3, 0xd3, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd4, 0xd4, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd5, 0xd5, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd6, 0xd6, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd7, 0xd7, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd8, 0xd8, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xd9, 0xd9, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xda, 0xda, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xdb, 0xdb, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xdc, 0xdc, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xdd, 0xdd, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xde, 0xde, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xdf, 0xdf, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe0, 0xe0, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe1, 0xe1, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe2, 0xe2, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe3, 0xe3, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe4, 0xe4, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe5, 0xe5, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe6, 0xe6, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe7, 0xe7, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe8, 0xe8, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xe9, 0xe9, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xea, 0xea, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xeb, 0xeb, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xec, 0xec, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xed, 0xed, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xee, 0xee, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xef, 0xef, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf0, 0xf0, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf1, 0xf1, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf2, 0xf2, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf3, 0xf3, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf4, 0xf4, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf5, 0xf5, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf6, 0xf6, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf7, 0xf7, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf8, 0xf8, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xf9, 0xf9, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xfa, 0xfa, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xfb, 0xfb, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xfc, 0xfc, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xfd, 0xfd, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xfe, 0xfe, entry_plain, isr_unknown),
        isr_expr!(isr_unknown_0xff, 0xff, entry_plain, isr_unknown),
    ];
}
