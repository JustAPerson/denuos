#[cfg(target_arch = "x86_64")]
pub mod x86;

/// Architecture independent intrinsics
#[cfg(target_arch = "x86_64")]
pub mod intrinsics {
    pub use super::x86::intrinsics::halt;
}
