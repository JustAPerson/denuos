use core;
use core::panic::PanicInfo;
use core::alloc::Layout;

#[lang = "eh_personality"] extern fn eh_personality() {}

#[panic_handler]
pub fn rust_panic_handler(panic: &PanicInfo) -> ! {
    use crate::vga::print_error;
    // TODO SMP need to stop other cores

    let unknown = format_args!("unknown");
    let msg = panic.message().unwrap_or(&unknown);
    if let Some(loc) = panic.location()  {
        print_error(format_args!("PANIC at {}:{}:{}\n    {}", loc.file(), loc.line(), loc.column(), msg));
    } else {
        print_error(format_args!("PANIC at unknown\n    {}", msg));
    }
}

#[alloc_error_handler]
pub fn rust_alloc_error_handler(layout: Layout) -> ! {
    panic!("OOM (request {:?})", layout);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
