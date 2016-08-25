/// Programmable Interrupt Controller
///
/// The x86 architecture is interrupt driven. Devices "interrupt" the CPU,
/// informing it to process the available information. To allow the masking and
/// prioritization of these interrupts, they are mediated by a Programmable
/// Interrupt Controller. Two 8259A PICs are utilized to allow up to 15 devices
/// to interrupt the CPU.
///
/// Each time the PIC interrupts the CPU, it issues an Interrupt Request (IRQ).
/// Every device is given a unique IRQ number which is translated to an
/// interrupt number for the processor. By default, the master PIC will map
/// IRQ0-7 to use interrupts 0x08-0x0f and the slave PIC will map IRQ8-15 to
/// use interrupts  0x70-0x77. Upon initialization we remap the PICs to use
/// interrupts 0x20-0x2f.
///
/// Currently we process the following IRQs:
///   - IRQ0 System Timer
///   - IRQ1 PS/2 Keyboard Input

use super::interrupts;
use super::intrinsics::{inb, outb};

/// Interrupt vector offset of the master PIC
pub const PIC1_OFFSET: u8 = 0x20;
/// Interrupt vector offset of the slave PIC
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

/// Wrapper for master PIC
static PIC1: Pic = Pic::new(0x20);
/// Wrapper for slave PIC
static PIC2: Pic = Pic::new(0xa0);

/// Wrapper around a PIC
struct Pic {
    port: u16,
}

impl Pic {
    /// Creates a wrapper around the PIC on the specified port
    const fn new(port: u16) -> Pic {
        Pic { port: port }
    }

    /// Writes byte to command port of PIC
    fn write_command(&self, b: u8) {
        outb(self.port, b)
    }

    /// Writes byte to data port of PIC
    fn write_data(&self, b: u8) {
        outb(self.port + 1, b)
    }

    /// Reads input from PIC
    fn read(&self) -> u8 {
        inb(self.port)
    }
}

/// Initializes both 8259A PICs
///
/// This remaps the PIC interrupt vectors to `PIC1_OFFSET` and `PIC2_OFFSET`
/// and modifies the IDT.
pub fn initialize() {
    // Constants for initialization command words
    const ICW1_INIT: u8 = 0x11; // start in cascade mode, requires ICW4

    const ICW3_PIC1: u8 = 0x04; // inform master of slave on IRQ2
    const ICW3_PIC2: u8 = 0x02; // inform slave to cascade through IRQ2

    const ICW4_8086: u8 = 0x01; // x86 compatibility mode

    // initialize master
    PIC1.write_command(ICW1_INIT);
    PIC1.write_data(PIC1_OFFSET);
    PIC1.write_data(ICW3_PIC1);
    PIC1.write_data(ICW4_8086);

    // initialize slave
    PIC2.write_command(ICW1_INIT);
    PIC2.write_data(PIC2_OFFSET);
    PIC2.write_data(ICW3_PIC2);
    PIC2.write_data(ICW4_8086);

    let mut idt = interrupts::Idt::current().unwrap();
    for i in PIC1_OFFSET..(PIC2_OFFSET + 8) {
        idt.register_isr(i as usize, general_irq);
    }
    idt.register_isr(0x20, system_timer);
    idt.register_isr(0x21, keyboard_input);
    idt.load();
    interrupts::enable();
}

/// Determines the IRQ number that was triggered
fn get_irq() -> Option<u8> {
    // read service registers
    PIC1.write_command(0x0b);
    PIC2.write_command(0x0b);
    let sr1 = PIC1.read() as u16;
    let sr2 = PIC2.read() as u16;
    let mut flags = (sr2 << 8) | sr1;

    // convert bitmask to IRQ number
    for i in 0..16 {
        if flags & 1 != 0 { // is lowest bit set?
            return Some(i);
        }
        flags = flags >> 1;
    }
    // "spurious" interrupt, doesn't correspond to a legitimate hardware event
    None
}

/// Informs the PIC that we have finished processing an interrupt
fn send_eoi(irq: u8) {
    const EOI: u8 = 0x20;
    if irq >= 8 {
        PIC2.write_command(EOI);
    }
    PIC1.write_command(EOI);
}

isr! {
    fn general_irq() {
        if let Some(irq) = get_irq() {
            panic!("Received unhadled IRQ{}", irq);
        }
    }

    fn system_timer() {
        send_eoi(0);
    }

    fn keyboard_input() {
        let sc = inb(0x60);
        println!("keyboard {:#x}", sc);
        send_eoi(1);
    }
}
