//! Kernel Heap Allocator
//!
//! Currently implemented using a simplistic bump allocator. Freed memory is
//! just leaked.
#![feature(const_fn)]
#![feature(allocator_internals)]
#![feature(alloc)]
#![feature(allocator_api)]

#![no_std]

extern crate spin;
extern crate alloc;

use spin::Mutex;
use alloc::alloc::{Alloc, GlobalAlloc, Layout, AllocErr};
use core::ptr::NonNull;

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
}

unsafe impl Alloc for BumpAllocator {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        let size = layout.size();
        let align = layout.align();

        let alloc_start = align_up(self.next, align);
        let alloc_end = alloc_start + size;

        if alloc_end <= self.end {
            self.next = alloc_end;

            Ok(NonNull::new_unchecked(alloc_start as *mut u8))
        } else {
            Err(AllocErr)
        }
    }

    unsafe fn dealloc(&mut self, _ptr: NonNull<u8>, _layout: Layout) {
        // leak memory for time being
    }
}

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

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.allocator.lock();
        allocator.alloc(layout).map(|p| p.as_ptr()).unwrap_or(0 as *mut u8)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.allocator.lock();
        allocator.dealloc(NonNull::new(ptr).expect("Attempt to dealloc null ptr"), layout);
    }
}

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::new();
