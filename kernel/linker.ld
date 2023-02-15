ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)

/* original layout dump:
  0 .rodata       00014302  0000000000200200  0000000000200200  00000200  2**4
                  CONTENTS, ALLOC, LOAD, READONLY, DATA
  1 .bootloader-config 00000073  0000000000214502  0000000000214502  00014502  2**0
                  CONTENTS, ALLOC, LOAD, READONLY, DATA
  2 .eh_frame_hdr 0000003c  0000000000214578  0000000000214578  00014578  2**2
                  CONTENTS, ALLOC, LOAD, READONLY, DATA
  3 .eh_frame     000000b4  00000000002145b8  00000000002145b8  000145b8  2**3
                  CONTENTS, ALLOC, LOAD, READONLY, DATA
  4 .text         0009ea0f  0000000000215670  0000000000215670  00014670  2**4
                  CONTENTS, ALLOC, LOAD, READONLY, CODE
  5 .data.rel.ro  000009e0  00000000002b5080  00000000002b5080  000b3080  2**3
                  CONTENTS, ALLOC, LOAD, DATA
  6 .got          000001b8  00000000002b5a60  00000000002b5a60  000b3a60  2**3
                  CONTENTS, ALLOC, LOAD, DATA
  7 .data         00000968  00000000002b6c80  00000000002b6c80  000b3c80  2**7
                  CONTENTS, ALLOC, LOAD, DATA
  8 .bss          00007f88  00000000002b7600  00000000002b7600  000b45e8  2**7
                  ALLOC
  ... debug info ...
*/

KERNEL_OFFSET = 0xFFFFFFFF80000000;

SECTIONS
{
    . = 8M;

    .text.ap : {
        *(.text.ap)
    }

    . += KERNEL_OFFSET;

    .bootloader-config ALIGN(4K) : AT(ADDR(.bootloader-config) - KERNEL_OFFSET) {
        *(.bootloader-config)
    }
    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_OFFSET) {
        *(.text .text.*)
    }
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_OFFSET) {
        *(.rodata .rodata.*)
    }
    .eh_frame ALIGN(4K) : AT(ADDR(.eh_frame) - KERNEL_OFFSET) {
        *(.eh_frame)
    }
    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_OFFSET) {
        *(.data .data.*)
    }
    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_OFFSET) {
        *(.bss .bss.*)
    }
    .got ALIGN(4K) : AT(ADDR(.got) - KERNEL_OFFSET) {
        *(.got .got.*)
    }
}
