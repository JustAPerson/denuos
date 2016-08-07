#[cfg(target_arch = "x86_64")]
#[path = "./x86/mod.rs"]
mod arch;

// will error on unsupported target
pub use self::arch::*;
