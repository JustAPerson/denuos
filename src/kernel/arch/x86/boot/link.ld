/*
 * This file originated from Philipp Oppermann's Rust OS blog series.
 * Copyright 2015 Philipp Oppermann. Please see the original license:
 * https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT
 * This file has been modified from its original form.
 */

ENTRY(start32)

SECTIONS {
    . = 1M;

    .boot : {
        /* ensure that the multiboot header is at the beginning */
        KEEP(*(.multiboot_header))
    }

    .text : { *(.text*) }

    .data : { *(.data*) }

    .rodata : { *(.rodata*) }

    /DISCARD/ : { *(.debug*) }
}
