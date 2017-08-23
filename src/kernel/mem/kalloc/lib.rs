//! Kernel Heap Allocator
//!
//! Currently implemented using a simplistic bump allocator. Freed memory is
//! just leaked.
#![feature(const_fn)]
#![feature(allocator_internals)]
#![feature(global_allocator)]
#![feature(alloc)]
#![feature(allocator_api)]

#![default_lib_allocator]
#![no_std]

extern crate spin;
extern crate alloc;

use spin::Mutex;

pub const HEAP_SIZE:  usize = 1024 * 1024; // 1MiB
pub const HEAP_START: usize = 0xffff_e000_0000_0000;

fn align_up(start: usize, align: usize) -> usize {
    let mask = align - 1;
    (start + mask) & !mask
}

struct BumpAllocator {
    next: usize,
    end: usize,
}

impl BumpAllocator {
    const fn new(start: usize, size: usize) -> BumpAllocator {
        BumpAllocator {
            next: start,
            end: start + size,
        }
    }

    fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let alloc_start = align_up(self.next, align);
        let alloc_end = alloc_start + size;

        if alloc_end <= self.end {
            self.next = alloc_end;
            Some(alloc_start as *mut u8)
        } else {
            None
        }
    }
}

use alloc::allocator::{Alloc, Layout, AllocErr};
struct GlobalAllocator {
    allocator: Mutex<BumpAllocator>,
}
impl GlobalAllocator {
    const fn new() -> GlobalAllocator {
        GlobalAllocator {
            allocator: Mutex::new(BumpAllocator::new(HEAP_START, HEAP_SIZE)),
        }
    }
}

unsafe impl<'a> Alloc for &'a GlobalAllocator {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        let mut allocator = self.allocator.lock();
        let ptr = allocator.allocate(layout.size(), layout.align());
        Ok(ptr.expect("Out of heap memory"))
    }
    unsafe fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {
        // leak memory for time being
    }
}

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::new();
