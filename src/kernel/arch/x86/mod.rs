pub mod frame_allocator;
#[macro_use]
pub mod interrupts;
pub mod intrinsics;
pub mod gdt;
pub mod multiboot;
pub mod paging;
pub mod pic;
pub mod syscall;
pub mod tss;

use self::multiboot::MultibootTags;
use self::frame_allocator::{MemRegion, frame_alloc, get_fallocator};

static mut PROTECTED_REGIONS: &'static mut [MemRegion; 2] = &mut [
    (0, 0), // kernel
    (0, 0), // multiboot info
];

#[no_mangle]
pub unsafe extern fn kstart(multiboot_tags: &MultibootTags) {
    let multiboot_info = multiboot_tags.parse();

    // protect some memory regions from frame allocator
    let elf_sections = multiboot_info.elf_sections.unwrap();
    PROTECTED_REGIONS[0] = (elf_sections.image_start(), elf_sections.image_end());
    PROTECTED_REGIONS[1] = (multiboot_tags.start(), multiboot_tags.end());

    println!("kernel region {:?}", PROTECTED_REGIONS[0]);
    println!("multiboot region {:?}", PROTECTED_REGIONS[1]);

    let mmap = multiboot_info.mem_map.unwrap();
    frame_allocator::initialize(mmap, PROTECTED_REGIONS);

    let free_pages = get_fallocator().free_pages();
    println!("first free page 0x{:x}", frame_alloc().addr());
    println!("free pages {} ({} MiB)", free_pages, free_pages / 256);

    let _ = paging::initialize();

    // set up interrupt handlers
    interrupts::initialize();
    pic::initialize();
    tss::initialize();
    syscall::initialize();
    enter_userspace();
}

pub fn enter_userspace() {
    // TODO map a stack at 0x8000_0000_0000
    syscall::sysret(userspace as usize, 0x200000);
}

pub fn userspace() {
    for i in 0..5 {
        unsafe { asm!("syscall") }
    }
    loop { }
}
