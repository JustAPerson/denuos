#![feature(lang_items, const_fn, unique)]
#![no_std]

extern crate rlibc;
extern crate spin;

// Import macros first
#[macro_use]
mod vga;

mod vestige;

#[no_mangle]
pub extern fn kmain() {
    vga::clear_screen();
    panic!("No userspace to run!");
}
