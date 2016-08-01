; This file originated from Philipp Oppermann's Rust OS blog series.
; Copyright 2015 Philipp Oppermann. Please see the original license:
; https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT

global start64

section .text
bits 64
start64:
    ; print `OKAY` to screen
    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt
