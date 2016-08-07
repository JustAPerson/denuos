#![feature(lang_items, const_fn, unique)]
#![no_std]

extern crate rlibc;
extern crate spin;

// Import macros first
#[macro_use]
mod vga;

pub mod main;
pub mod vestige;
