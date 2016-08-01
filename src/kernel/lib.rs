#![feature(lang_items, const_fn, unique)]
#![no_std]

extern crate rlibc;

mod vestige;

#[no_mangle]
pub extern fn kmain() {
    use core::slice;

    unsafe {
        let vga = slice::from_raw_parts_mut(0xb8000 as *mut u16, 80 * 25);
        vga[80] = (0x3f << 8) | 'H' as u16;
        vga[81] = (0x3f << 8) | 'i' as u16;
    }
}
