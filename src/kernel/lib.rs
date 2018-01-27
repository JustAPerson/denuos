#![feature(alloc)]
#![feature(asm)]
#![feature(box_syntax)]
#![feature(const_fn)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(unique)]
#![feature(ptr_internals)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate bitflags;
extern crate kalloc;
extern crate rlibc;
extern crate spin;

// Import macros first
#[macro_use]
pub mod vga;

pub mod arch;
pub mod main;
pub mod vestige;
