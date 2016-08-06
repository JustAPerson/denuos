#![feature(lang_items, const_fn, unique)]
#![no_std]

extern crate rlibc;
extern crate spin;

mod vestige;
#[macro_use] mod vga;

#[no_mangle]
pub extern fn kmain() {
    vga::clear_screen();
    println!("Hello, world!");
}
