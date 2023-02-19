.section .text.init
.globl ap_start
.globl _init_section_start
.globl _init_section_end

# the init section is copied to phys address 0x10000,
# the trampoline will be one page below that: (see smp/mod.rs)
# 0x10000 - 0x1000 = 0xF000

.equ TRAMPOLINE,     0xF000
.equ AP_READY,       TRAMPOLINE + 0
.equ AP_ID,          TRAMPOLINE + 1
.equ AP_PAGE_TABLE,  TRAMPOLINE + 8
.equ AP_STACK_START, TRAMPOLINE + 16
.equ AP_STACK_END,   TRAMPOLINE + 24
.equ AP_GDT,         TRAMPOLINE + 32
.equ AP_ENTRY_CODE,  TRAMPOLINE + 40

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

    # enable PGE, PAE, PSE
    mov    eax, cr4
    or     eax, (1 << 7) | (1 << 5)
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

    # enable paging and write protect
    mov    eax, cr0
    or     eax, (1 << 31) | (1 << 16) | (1 << 0)
    mov    cr0, eax

    # jump to _64bit_start
    push 8
    push 0x10140
    retf
    nop
    nop

    .align 64
    .code64
_64bit_start:
    #mov rax, 0
    #mov ss, ax
    #mov ds, ax
    #mov es, ax
    #mov fs, ax
    #mov gs, ax

halt:
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    hlt
    jmp halt

_init_section_end:
