//! Kernel Heap Allocator
//!
//! Currently implemented using a simplistic bump allocator. Freed memory is
//! just leaked.
#![feature(allocator)]
#![feature(const_fn)]

#![allocator]
#![no_std]

extern crate spin;

use spin::Mutex;

pub const HEAP_SIZE:  usize = 1024 * 1024; // 1MiB
pub const HEAP_START: usize = 0xffff_e000_0000_0000;

static ALLOCATOR: Mutex<BumpAllocator> = Mutex::new(BumpAllocator::new(HEAP_START, HEAP_SIZE));

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

fn align_up(start: usize, align: usize) -> usize {
    let mask = align - 1;
    (start + mask) & !mask
}

#[no_mangle]
pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    ALLOCATOR.lock().allocate(size, align).expect("Out of heap memory")
}

#[no_mangle]
pub extern fn __rust_reallocate(ptr: *mut u8, size: usize, new_size: usize,
                                align: usize) -> *mut u8 {
    // taken from rust's liballoc_system
    use core::{ptr, cmp};
    let new_ptr = __rust_allocate(new_size, align);
    unsafe { ptr::copy(ptr, new_ptr, cmp::min(size, new_size)) };
    __rust_deallocate(ptr, size, align);
    new_ptr
}


#[allow(unused_variables)]
#[no_mangle]
pub extern fn __rust_deallocate(ptr: *mut u8, size: usize, align: usize) {
    // leaking mem is fine
}

#[allow(unused_variables)]
#[no_mangle]
pub extern fn __rust_usable_size(size: usize, align: usize) -> usize {
    // Don't allocate more than necessary
    size
}

#[allow(unused_variables)]
#[no_mangle]
pub extern fn __rust_reallocate_inplace(ptr: *mut u8, size: usize,
                                        new_size: usize, align: usize) -> usize {
    // Cannot realloc in place
    size
}
