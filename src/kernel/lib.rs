#![feature(alloc)]
#![feature(asm)]
#![feature(associated_consts)]
#![feature(box_syntax)]
#![feature(collections)]
#![feature(const_fn)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(unique)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate collections;
extern crate kalloc;
extern crate rlibc;
extern crate spin;

// Import macros first
#[macro_use]
pub mod vga;

pub mod arch;
pub mod main;
pub mod vestige;
