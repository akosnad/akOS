.section .text.ap
.globl ap_start
.globl _ap_section_start
.globl _ap_section_end
.code16

_ap_section_start:
ap_start:
    cli

    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    mov ax, 0xBEEF

    jmp halt

setup_gdt:
    mov ax, 0
    mov es, ax
    mov di, 0x800

    call kernel_ap_main

    # kernel_ap_main should never return
halt:
    hlt
    jmp halt


_ap_section_end:
    .byte 0x00
