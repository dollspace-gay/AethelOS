/* The Unbreakable Binding - Ultimate Linker Script for AethelOS */
ENTRY(boot32_start)

PHDRS
{
    text PT_LOAD FLAGS(7);  /* 7 = Read + Write + Execute */
}

SECTIONS
{
    /* Start at 1 MiB - standard for multiboot kernels */
    . = 1M;

    /* Multiboot2 header MUST be at file offset 0 for GRUB to find it */
    /* VMA = 1M (where it runs), LMA = 0x100000 (where GRUB loads it) */
    .boot ALIGN(8) : AT(1M)
    {
        KEEP(*(.multiboot))
        KEEP(*(.boot))
    } :text

    /* 32-bit bootstrap code immediately follows */
    .boot.text ALIGN(4) : AT(1M + SIZEOF(.boot))
    {
        *(.boot.text)
    } :text

    /* Main executable code */
    .text ALIGN(4K) : AT(ADDR(.text))
    {
        *(.text .text.*)
    } :text

    /* Read-only data */
    .rodata ALIGN(4K) : AT(ADDR(.rodata))
    {
        *(.rodata .rodata.*)
    } :text

    /* Initialized data */
    .data ALIGN(4K) : AT(ADDR(.data))
    {
        *(.data .data.*)
    } :text

    /* Uninitialized data */
    .bss ALIGN(4K) : AT(ADDR(.bss))
    {
        *(.bss .bss.*)
        *(COMMON)
    } :text

    /* Thread-local storage */
    .tdata ALIGN(4K) : AT(ADDR(.tdata))
    {
        *(.tdata .tdata.*)
    } :text

    .tbss ALIGN(4K) : AT(ADDR(.tbss))
    {
        *(.tbss .tbss.*)
    } :text

    /* Exception handling frame - needed but can be at end */
    .eh_frame_hdr ALIGN(4) : AT(ADDR(.eh_frame_hdr))
    {
        *(.eh_frame_hdr)
    } :text

    .eh_frame ALIGN(4) : AT(ADDR(.eh_frame))
    {
        *(.eh_frame)
    } :text

    /* Discard sections we don't need */
    /DISCARD/ :
    {
        *(.comment)
        *(.note .note.*)
    }
}
