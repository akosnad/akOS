.section .text.init
.globl ap_start
.globl _ap_section_start
.globl _ap_section_end

_ap_section_start:
    .code16
ap_start:
    cli
    cld

    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov sp, 0xFC00

    .align 32
    .code32
_32bit_start:
    mov    ax, 16
    mov    ds, ax
    mov    ss, ax

    jmp _setup_paging

_setup_paging:
    # first, disable paging
    mov    eax, cr0
    and    eax, 0x7FFFFFFF
    mov    cr0, eax

    # enable PGE, PAE, PSE
    mov    eax, cr4
    or     eax, (1 << 7) | (1 << 5)
    mov    cr4, eax

    # load P4 to cr3


    .code64
_64bit_start:

halt:
    hlt
    jmp halt

_ap_section_end:
    .byte 0x00
