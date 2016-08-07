; This file originated from Philipp Oppermann's Rust OS blog series.
; Copyright 2015 Philipp Oppermann. Please see the original license:
; https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT

global start64

section .text
bits 64
start64:
    ; multiboot info in edi
    extern kstart
    call kstart
    hlt
