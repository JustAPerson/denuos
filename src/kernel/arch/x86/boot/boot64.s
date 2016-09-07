; This file originated from Philipp Oppermann's Rust OS blog series.
; Copyright 2015 Philipp Oppermann. Please see the original license:
; https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT
; This file has been modified from its original form.

global start64

section .text
bits 64
start64:
    ; rust kstart will remove identity mapping for first 2GB but will still
    ; continue using stack defined in boot32.s. Thus we need to adjust address
    ; so it is mapped correctly.
    add rsp, 0xffffffff80000000
    extern kstart, KERNEL_BASE
    ; multiboot info in edi
    call kstart
    hlt
