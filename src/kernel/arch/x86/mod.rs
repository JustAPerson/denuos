use main;

pub mod frame_allocator;
#[macro_use]
pub mod interrupts;
pub mod intrinsics;
pub mod gdt;
pub mod multiboot;
pub mod paging;
pub mod pic;
pub mod stacks;
pub mod syscall;
pub mod tss;

pub const KERNEL_BASE: usize = 0xffffffff80000000;

use self::multiboot::MultibootTags;
use self::frame_allocator::{frame_alloc, get_fallocator};

#[no_mangle]
pub unsafe extern fn kstart(multiboot_tags: &MultibootTags) {
    let multiboot_info = multiboot_tags.parse();

    // protect some memory regions from frame allocator
    let elf_sections = multiboot_info.elf_sections.unwrap();
    let (k_begin, k_end) = (elf_sections.image_start(), elf_sections.image_end() - KERNEL_BASE);
    let (m_begin, m_end) = (multiboot_tags.start(), multiboot_tags.end());
    let protected_regions = [
        (k_begin, k_end), // kernel image
        (m_begin, m_end), // multiboot data
    ];
    let mmap = multiboot_info.mem_map.unwrap();
    frame_allocator::initialize(mmap, protected_regions);

    println!("boot loader: {}", &multiboot_info.boot_loader_name.unwrap_or("none"));
    println!("cmd line: {}", &multiboot_info.cmd_line.unwrap_or("none"));
    println!("");
    println!("protected memory regions");
    println!("  kernel:    ({:#x}, {:#x}) size {} KiB", k_begin, k_end, (k_end - k_begin) / 1024);
    println!("  multiboot: ({:#x}, {:#x}) size {} KiB", m_begin, m_end, (m_end - m_begin) / 1024);
    println!("first free page 0x{:x}", frame_alloc().addr());
    let free_pages = get_fallocator().free_pages();
    println!("free pages {} ({} MiB)", free_pages, free_pages / 256);

    let _ = paging::initialize();
    // set up interrupt handlers
    interrupts::initialize();
    pic::initialize();
    gdt::initialize();
    tss::initialize();
    syscall::initialize();

    main::kmain();
}

#[repr(packed)]
pub struct Registers {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub cs: u16,
    pub ss: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    _pad:   u32, // 12 bytes of selectors would otherwise unalign the following
    pub rip:    u64,
    pub rflags: u64,
    pub rsp:    u64,
}

impl Registers {
    fn default_user(rip: usize, rsp: usize) -> Self {
        use self::gdt::{USR_CODE_OFFSET, USR_DATA_OFFSET};
        Registers {
            rip: rip as u64, cs: USR_CODE_OFFSET as u16,
            rsp: rsp as u64, ss: USR_DATA_OFFSET as u16,
            rflags: 0x200, // TODO standardize rflags
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
                    rbp: 0, rsi: 0, rdi: 0,
            r8:  0, r9:  0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            ds:  0, es:  0, fs:  0, gs:  0,
            _pad: 0,
        }
    }
}
