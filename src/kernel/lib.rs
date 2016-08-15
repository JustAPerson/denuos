#![feature(asm)]
#![feature(associated_consts)]
#![feature(collections)]
#![feature(const_fn)]
#![feature(lang_items)]
#![feature(unique)]
#![no_std]

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
