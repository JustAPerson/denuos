use core;

#[lang = "eh_personality"] extern fn eh_personality() {}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn rust_begin_unwind(fmt: core::fmt::Arguments, file: &str, line: u32, column: u32) -> ! {
    use vga::print_error;
    // TODO SMP need to stop other cores
    print_error(format_args!("PANIC in {}:{}:{}\n    {}", file, line, column, fmt));
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
