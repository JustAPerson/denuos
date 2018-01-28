#[cfg(target_arch = "x86_64")] pub mod x86;
#[cfg(target_arch = "x86_64")] pub mod generic {
    use super::x86;
    pub use self::x86::Registers;

    pub mod intrinsics {
        pub use super::x86::intrinsics::halt;
    }
}

