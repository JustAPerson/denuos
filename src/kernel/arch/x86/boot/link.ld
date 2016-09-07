/*
 * This file originated from Philipp Oppermann's Rust OS blog series.
 * Copyright 2015 Philipp Oppermann. Please see the original license:
 * https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT
 * This file has been modified from its original form.
 */

ENTRY(start32)

KERNEL_BASE = 0xffffffff80000000;

SECTIONS {
    . = 1M;

    .boot : {
        /* ensure that the multiboot header is at the beginning */
        KEEP(*(.multiboot_header))
        *boot32.o (.text)
        *boot64.o (.text)
        *boot32.o (.data)
        *boot32.o (.bss)
    }

    . += KERNEL_BASE;

    /* use AT keyword to load sections sequentially around 1M physical mem */
    /* but link/relocate as if loaded around KERNEL_BASE virtual mem */
    .text : AT (ADDR(.text)-KERNEL_BASE) { *(.text*) }

    .data : AT (ADDR(.data)-KERNEL_BASE) { *(.data*) }

    .rodata : AT(ADDR(.rodata)-KERNEL_BASE) { *(.rodata*) }

    .bss : AT (ADDR(.bss)-KERNEL_BASE) { *(.bss*) }

    /DISCARD/ : { *(.debug*) }
}
