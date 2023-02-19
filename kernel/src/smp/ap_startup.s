.section .text.init
.globl ap_start
.globl _init_section_start
.globl _init_section_end

.equ KERNEL_OFFSET, 0xFFFFFFFF80000000

# the init section is copied to phys address 0x10000,
# the trampoline will be one page below that: (see smp/mod.rs)
# 0x10000 - 0x1000 = 0xF000

.equ TRAMPOLINE,     0xF000
.equ AP_ID,          TRAMPOLINE + 0
.equ AP_PAGE_TABLE,  TRAMPOLINE + 8
.equ AP_STACK_START, TRAMPOLINE + 16
.equ AP_STACK_END,   TRAMPOLINE + 24
.equ AP_ENTRY_CODE,  TRAMPOLINE + 32

_init_section_start:
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
create_gdt:
    mov ax, 0x0
    mov es, ax
    mov di, 0x800

    # null descriptor
    mov cx, 4
    rep stosw

    # code segment descriptor
    mov word ptr es:[di],   0xFFFF # limit
    mov word ptr es:[di+2], 0x0000 # base
    mov byte ptr es:[di+4], 0x00   # base
    mov byte ptr es:[di+5], 0x9A   # access
    mov byte ptr es:[di+6], 0xCF   # flags + limit
    mov byte ptr es:[di+7], 0x00   # base
    add di, 8

    # data segment descriptor
    mov word ptr es:[di],   0xFFFF # limit
    mov word ptr es:[di+2], 0x0000 # base
    mov byte ptr es:[di+4], 0x00   # base
    mov byte ptr es:[di+5], 0x92   # access
    mov byte ptr es:[di+6], 0xCF   # flags + limit
    mov byte ptr es:[di+7], 0x00   # base
    add di, 8

    mov word ptr es:[di], 23
    mov dword ptr es:[di+2], 0x800

    lgdt es:[di]

    # enable A20 line
    in al, 0x92
    or al, 0x2
    out 0x92, al

    # disable NMI
    in al, 0x70
    or al, 0x80
    out 0x70, al
    in al, 0x71

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax

    mov eax, cr0
    or eax, 1
    mov cr0, eax


    # jump to _32bit_start
    push 8
    push dword ptr 0x100c0
    retf
    nop
    nop

    .align 32
    .code32
_32bit_start:
    mov    ax, 0x10
    mov    ds, ax
    mov    es, ax
    mov    fs, ax
    mov    gs, ax
    mov    ss, ax

    mov esp, 0xfc00

_setup_paging:
    # first, disable paging
    mov    eax, cr0
    and    eax, 0x7FFFFFFF
    mov    cr0, eax

    # enable PAE, MCE, OSXMMEXCPT OSFXSR, DE
    mov    eax, cr4
    or     eax, (1 << 5) | (1 << 6) | (1 << 9) | (1 << 10) | (1 << 3)
    mov    cr4, eax

    # load P4 to cr3
    mov    eax, [AP_PAGE_TABLE]
    mov    cr3, eax

    # set no execute bit, long mode
    # in the EFER MSR
    mov    ecx, 0xC0000080
    rdmsr
    or     eax, (1 << 11) | (1 << 8)
    wrmsr

    # enable paging, write protect and protected mode
    mov    eax, cr0
    or     eax, (1 << 31) | (1 << 16) | (1 << 0)
    mov    cr0, eax

_create_gdt_64:
    # load 64 bit GDT
    #lgdt [GDT_AP.ptr]

    lea edi, GDT_AP.ptr
    lgdt dword ptr [edi]

    # jump to _64bit_start
    push 8
    push dword ptr 0x10140
    retf
    #ljmp 8, 0x10140
    nop
    nop

    .align 64
    .code64
_64bit_start:
    mov rax, qword ptr [AP_STACK_END]
    mov rsp, rax

    mov rax, qword ptr [AP_STACK_START]
    mov rbp, rax

    mov rax, qword ptr [AP_ENTRY_CODE]
    call rax

halt:
    hlt
    jmp halt

_init_section_end:
    nop



.section .data.init
.globl GDT_AP

GDT_AP:
    GDT_AP.null:
        .quad 0
    GDT_AP.code:
#        .long 0xFFFF        # Limit & Base (low, bits 0-15)
#        .byte 0             # Base (mid, bits 16-23)
#        .byte 0x94          # Access: Present, not system, exec, RW
#        .byte 0xAF          # Flags: 4K, LONG_MODE & Limit (high, bits 16-19)
#        .byte 0             # Base (high, bits 24-31)
        .quad 0x00AF9B000000FFFF
    GDT_AP.data:
#        .long 0xFFFF        # Limit & Base (low, bits 0-15)
#        .byte 0             # Base (mid, bits 16-23)
#        .byte 0x92          # Access: Present, not system, RW
#        .byte 0xCF          # Flags: 4K, SZ_32 & Limit (high, bits 16-19)
#        .byte 0             # Base (high, bits 24-31)
        .quad 0x00CF93000000FFFF
    GDT_AP.TSS:
        .long 0x00000068
        .long 0x00CF8900
    GDT_AP.ptr:
        .word GDT_AP.ptr - GDT_AP - 1
        .quad GDT_AP

.equ GDT_AP_CODE, GDT_AP.code - GDT_AP
.equ GDT_AP_DATA, GDT_AP.data - GDT_AP
.equ GDT_AP_PTR, GDT_AP.ptr - GDT_AP
