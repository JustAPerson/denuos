pub mod frame_allocator;
pub mod multiboot;

use self::multiboot::MultibootTags;
use self::frame_allocator::{MemRegion, FrameAllocator};

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
    let mut allocator = FrameAllocator::new(mmap, PROTECTED_REGIONS);

    println!("first free page 0x{:x}", allocator.alloc().addr());
    println!("free pages {} ({} MiB)", allocator.free_pages(), allocator.free_pages() / 256);
}
