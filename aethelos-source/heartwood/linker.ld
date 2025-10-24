/* Linker script for AethelOS Heartwood kernel */

ENTRY(_start)

/* Define the memory layout */
SECTIONS
{
    /* The kernel starts at 1MB (0x100000) - standard for higher-half kernels */
    . = 1M;

    /* Multiboot2 header MUST be in the first 32KB and 8-byte aligned */
    .boot : ALIGN(8)
    {
        /* Multiboot header must be first */
        KEEP(*(.multiboot))
        . = ALIGN(8);
    }

    /* Code section immediately follows */
    .text ALIGN(4K) : AT(ADDR(.text))
    {
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata ALIGN(4K) : AT(ADDR(.rodata))
    {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data ALIGN(4K) : AT(ADDR(.data))
    {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss ALIGN(4K) : AT(ADDR(.bss))
    {
        *(COMMON)
        *(.bss .bss.*)
    }

    /* Thread-local storage */
    .tdata ALIGN(4K) : AT(ADDR(.tdata))
    {
        *(.tdata .tdata.*)
    }

    .tbss ALIGN(4K) : AT(ADDR(.tbss))
    {
        *(.tbss .tbss.*)
    }

    /* Debug sections */
    .debug_info     0 : { *(.debug_info) }
    .debug_abbrev   0 : { *(.debug_abbrev) }
    .debug_line     0 : { *(.debug_line) }
    .debug_str      0 : { *(.debug_str) }
    .debug_ranges   0 : { *(.debug_ranges) }

    /* Discard unnecessary sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.note .note.*)
    }
}
