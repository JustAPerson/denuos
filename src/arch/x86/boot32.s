; This file originated from Philipp Oppermann's Rust OS blog series.
; Copyright 2015 Philipp Oppermann. Please see the original license:
; https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT

global start32

section .text
bits 32
start32:
    mov dword [0xb8000], 0x1f691f48
    hlt
