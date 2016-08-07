use vga;

#[no_mangle]
pub extern fn kmain() {
    vga::clear_screen();
    panic!("No userspace to run!");
}
