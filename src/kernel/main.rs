use vga;

#[no_mangle]
pub extern fn kmain() {
    vga::get_vgabuffer().clear();
    panic!("No userspace to run!");
}
