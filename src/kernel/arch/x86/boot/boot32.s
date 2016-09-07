; This file originated from Philipp Oppermann's Rust OS blog series.
; Copyright 2015 Philipp Oppermann. Please see the original license:
; https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT
;
; Error codes
;   0 - Not loaded via a multiboot compliant bootloader
;   1 - Incompatible CPU (no cpuid instruction)
;   2 - Incompatible CPU (no x86_64 long mode)
;   3 - Incompatible CPU (no SSE)

global start32

section .text
bits 32
start32:
    ; setup stack
    mov esp, stack_top
    mov edi, ebx ; save multiboot info

    ; check CPU compatability
    call check_multiboot
    call check_cpuid
    call check_long_mode

    ; enter long mode
    call set_up_page_tables
    call enable_paging
	call enable_sse

    ; load the 64 bit GDT
    lgdt [GDT.pointer]

    ; update selectors
    mov ax, GDT.data
    mov ss, ax  ; stack selector
    mov ds, ax  ; data selector
    mov es, ax  ; extra selector

    extern start64
    jmp GDT.code:start64

; Prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error

check_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    ; in the FLAGS register. If we can flip it, CPUID is available.

    ; Copy FLAGS in to EAX via stack
    pushfd
    pop eax

    ; Copy to ECX as well for comparing later on
    mov ecx, eax

    ; Flip the ID bit
    xor eax, 1 << 21

    ; Copy EAX to FLAGS via the stack
    push eax
    popfd

    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the
    ; ID bit back if it was ever flipped).
    push ecx
    popfd

    ; Compare EAX and ECX. If they are equal then that means the bit
    ; wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error

check_long_mode:
    ; test if extended processor info in available
    mov eax, 0x80000000    ; implicit argument for cpuid
    cpuid                  ; get highest supported argument
    cmp eax, 0x80000001    ; it needs to be at least 0x80000001
    jb .no_long_mode       ; if it's less, the CPU is too old for long mode

    ; use extended info to test if long mode is available
    mov eax, 0x80000001    ; argument for extended processor info
    cpuid                  ; returns various feature bits in ecx and edx
    test edx, 1 << 29      ; test if the LM-bit is set in the D-register
    jz .no_long_mode       ; If it's not set, there is no long mode
    ret
.no_long_mode:
    mov al, "2"
    jmp error

    ; setup simultaneous -2GB through +2GB mapping
    ; necessary because this assembly code is linked at 0x100000
    ; but the rust code is mapped at 0xffffffff80100000
set_up_page_tables:
    ; point two level 4 entries to a single level 3 table
    mov eax, p3_table
    or eax, 0b11 ; present + writable
    mov [p4_table], eax
    mov [p4_table + 511*8], eax

    ; Because two entries in level 4 table point to this table,
    ; there are 4 extraneous mappings. The comments below describe
    ; the intended / useful mappings
    mov eax, 0x83 ; 1GB mapping to address 0x00000000
    mov [p3_table +   0*8], eax ; 0x00000000 -> 0x00000000
    mov [p3_table + 510*8], eax ; 0xffffffff80000000 -> 0x00000000
    add eax, 1<<30 ; 1GB mapping to address 0x40000000
    mov [p3_table +   1*8], eax ; 0x40000000 -> 0x40000000
    mov [p3_table + 511*8], eax ; 0xffffffffc0000000 -> 0x40000000
    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

enable_sse:
    ; Check for SSE and enable it.
    mov eax, 0x1
    cpuid
    test edx, 1<<25
    jz .no_SSE

    ; enable SSE
    mov eax, cr0
    and ax, 0xFFFB      ; clear coprocessor emulation CR0.EM
    or ax, 0x2          ; set coprocessor monitoring  CR0.MP
    mov cr0, eax
    mov eax, cr4
    or ax, 3 << 9       ; set CR4.OSFXSR and CR4.OSXMMEXCPT at the same time
    mov cr4, eax

    ret
.no_SSE:
    mov al, "3"
    jmp error

%xdefine SYS     0 << 45
%xdefine CODE    3 << 43
%xdefine DATA    2 << 43
%xdefine LONG    1 << 53
%xdefine PRESENT 1 << 47
%xdefine WRITE   1 << 41

section .data

global GDT
GDT:
; null descriptor
    dq 0
; kernel segments
.code: equ $ - GDT
    dq SYS | CODE | PRESENT | LONG
.data: equ $ - GDT
    dq SYS | DATA | PRESENT | WRITE
; userpsace segments
.pointer:
    dw $ - GDT - 1
    dq GDT

; Allocate space for the stack
; Necessary since we don't know the memory map yet
section .bss
stack_bottom:
    resb 4096
stack_top:

; Allocate space for page tables
align 4096
p4_table: resb 4096
p3_table: resb 4096
