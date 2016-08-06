use core;

#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "panic_fmt"]
extern fn panic_fmt(fmt: core::fmt::Arguments, file: &str, line: u32) -> ! {
    use vga::{Color, ColorCode, WRITER};

    WRITER.lock().set_colorcode(ColorCode::new(Color::Red, Color::Black));
    println!("PANIC in {} at line {}:", file, line);
    println!("    {}", fmt);
    loop{}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
