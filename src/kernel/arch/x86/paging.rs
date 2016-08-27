use core;

use kalloc::{HEAP_SIZE, HEAP_START};

use super::frame_allocator::{frame_alloc, PAGE_SIZE};

pub const PTE_ADDR_MASK: usize = 0x000f_ffff_ffff_f000;

pub const PT1_INDEX: usize = 0x1ff << (0 * 9 + 12);
pub const PT2_INDEX: usize = 0x1ff << (1 * 9 + 12);
pub const PT3_INDEX: usize = 0x1ff << (2 * 9 + 12);
pub const PT4_INDEX: usize = 0x1ff << (3 * 9 + 12);

bitflags! {
    pub flags PageFlags: usize {
        const NONE          = 0,
        const PRESENT       = 1 << 0,
        const WRITE         = 1 << 1,
        const USER          = 1 << 2,
        const WRITE_THROUGH = 1 << 3,
        const NO_CACHE      = 1 << 4,
        const ACCESSED      = 1 << 5,
        const DIRTY         = 1 << 6,
        const HUGE          = 1 << 7,
        const GLOBAL        = 1 << 8,
        const NO_EXECUTE    = 1 << 63,
    }
}

struct PageEntry<L: PageLevel> {
    pub value: usize,
    level: core::marker::PhantomData<L>,
}

pub const NUM_ENTRIES: usize = 512;
struct PageTable<L: PageLevel> {
    entries: [PageEntry<L>; NUM_ENTRIES],
}

// Type safety magic
enum Level1 { }
enum Level2 { }
enum Level3 { }
enum Level4 { }

trait PageLevel {
    const LEVEL: usize;
    fn can_be_huge() -> bool {
        Self::LEVEL == 2 || Self::LEVEL == 3
    }
}
impl PageLevel for Level1 { const LEVEL: usize = 1; }
impl PageLevel for Level2 { const LEVEL: usize = 2; }
impl PageLevel for Level3 { const LEVEL: usize = 3; }
impl PageLevel for Level4 { const LEVEL: usize = 4; }

trait MappableLevel: PageLevel { }
impl MappableLevel for Level1  { }
impl MappableLevel for Level2  { }
impl MappableLevel for Level3  { }

trait NextPageLevel: PageLevel { type Next: MappableLevel; }
impl NextPageLevel for Level2  { type Next = Level1; }
impl NextPageLevel for Level3  { type Next = Level2; }
impl NextPageLevel for Level4  { type Next = Level3; }

impl<L: PageLevel> PageEntry<L> {
    fn set_addr(&mut self, addr: usize) {
        self.value = addr & PTE_ADDR_MASK;
    }

    fn get_addr(&self) -> usize {
        self.value & PTE_ADDR_MASK
    }

    fn flags(&self) -> PageFlags {
        PageFlags::from_bits_truncate(self.value)
    }

    fn terminal(&self) -> bool {
        if L::LEVEL == 1 {
            true
        } else if L::can_be_huge() {
            self.flags().contains(HUGE)
        } else {
            false
        }
    }

    fn present(&self) -> bool {
        self.flags().contains(PRESENT)
    }

    fn points_to_table(&self) -> bool {
        self.present() && !self.terminal()
    }
}

impl<L: PageLevel> PageTable<L> {
    fn new() ->  *mut PageTable<L> {
        let mut frame = frame_alloc();
        frame.clear();
        frame.addr() as *mut PageTable<L>
    }
}

impl<L: MappableLevel> PageTable<L> {
    fn map_mem(&mut self, index: usize, paddr: usize, flags: PageFlags) {
        self.entries[index].set_addr(paddr);
        self.entries[index].value |= flags.bits();
        self.entries[index].value |= PRESENT.bits();
        if L::can_be_huge() { // allow 2MB / 1GB pages
            self.entries[index].value |= HUGE.bits();
        }
    }
}

impl<L: NextPageLevel> PageTable<L> {
    fn map_table<'a>(&mut self, index: usize, table: *const PageTable<L::Next>) {
        self.entries[index].set_addr(table as usize);
        // if the entry in PT4 is not marked USER, then none of the pages mapped
        // in any lower tables (PT3-1) can be USER. Thus, mark all entries
        // pointing to tables as USER. Similar problem for WRITE.
        // Note: ring0 ignores WRITE flag unless CR0.WP is set
        self.entries[index].value |= (PRESENT | USER | WRITE).bits();
    }

    fn get_table_mut(&mut self, index: usize) -> Option<&mut PageTable<L::Next>> {
        let ref entry = self.entries[index];
        if !entry.points_to_table() { return None; }

        unsafe { Some(&mut *(entry.get_addr() as *mut PageTable<_>)) }
    }

    fn get_new_table(&mut self, index: usize) -> &mut PageTable<L::Next> {
        if self.entries[index].present() {
            self.get_table_mut(index).expect("Memory already mapped to")
        } else {
            let pt = PageTable::new();
            self.map_table(index, pt);
            self.get_table_mut(index).unwrap()
        }
    }
}

pub unsafe fn initialize() -> PT4 {
    let mut pt4 = PT4::new();
    pt4.map_to_1g(0, 0, NONE);

    // map heap
    for i in 0..HEAP_SIZE / PAGE_SIZE {
        let addr = i * PAGE_SIZE + HEAP_START;
        pt4.map_4k(addr, WRITE);
    }

    pt4.activate(); // flushes TLB
    pt4
}

pub struct PT4 {
    table: core::ptr::Unique<PageTable<Level4>>,
}

impl PT4 {
    pub fn new() -> PT4 {
        PT4 {
            table: unsafe { core::ptr::Unique::new(PageTable::new()) },
        }
    }

    fn get(&self) -> &PageTable<Level4> {
        unsafe { self.table.get() }
    }

    fn get_mut(&mut self) -> &mut PageTable<Level4> {
        unsafe { self.table.get_mut() }
    }

    pub fn map_4k(&mut self, vaddr: usize, flags: PageFlags) {
        self.map_to_4k(vaddr, frame_alloc().addr(), flags)
    }

    pub fn map_to_4k(&mut self, vaddr: usize, paddr: usize, flags: PageFlags) {
        self.get_mut()
            .get_new_table(get_pt4_index(vaddr))
            .get_new_table(get_pt3_index(vaddr))
            .get_new_table(get_pt2_index(vaddr))
            .map_mem(get_pt1_index(vaddr), paddr, flags);
    }

    pub fn map_to_2m(&mut self, vaddr: usize, paddr: usize, flags: PageFlags) {
        self.get_mut()
            .get_new_table(get_pt4_index(vaddr))
            .get_new_table(get_pt3_index(vaddr))
            .map_mem(get_pt2_index(vaddr), paddr, flags);
    }

    pub fn map_to_1g(&mut self, vaddr: usize, paddr: usize, flags: PageFlags) {
        self.get_mut()
            .get_new_table(get_pt4_index(vaddr))
            .map_mem(get_pt3_index(vaddr), paddr, flags);
    }

    pub fn activate(&self) {
        unsafe { asm!("mov cr3, $0" :: "r"(self.get()) :: "intel"); }
    }
}

pub fn get_pt1_index(val: usize) -> usize {
    (val & PT1_INDEX) >> 12
}
pub fn get_pt2_index(val: usize) -> usize {
    (val & PT2_INDEX) >> 21
}
pub fn get_pt3_index(val: usize) -> usize {
    (val & PT3_INDEX) >> 30
}
pub fn get_pt4_index(val: usize) -> usize {
    (val & PT4_INDEX) >> 39
}
