/* The Unbreakable Binding - Higher-Half Linker Script for AethelOS */
/* Physical load at 1MB, virtual execution in top 2GB */
/* Pattern based on Linux kernel vmlinux.lds.S */
/* Uses PIC relocation model for RIP-relative addressing in higher-half */
ENTRY(boot32_start)

/* Force linker to keep _start symbol (Rust kernel entry point) */
EXTERN(_start)

/* Higher-half kernel virtual base address - top 2GB of address space */
KERNEL_VMA = 0xFFFFFFFF80000000;

PHDRS
{
    boot PT_LOAD FLAGS(7);   /* Boot code at low addresses */
    kernel PT_LOAD FLAGS(7); /* All kernel sections at higher-half */
}

SECTIONS
{
    /* Boot sections at LOW addresses (identity-mapped before paging) */
    . = 1M;

    .boot ALIGN(8) : AT(1M)
    {
        KEEP(*(.multiboot))
        KEEP(*(.boot))
    } :boot

    .boot.text ALIGN(4) : AT(1M + SIZEOF(.boot))
    {
        *(.boot.text)
    } :boot

    /* Track where boot sections end physically */
    __boot_phys_end = 1M + SIZEOF(.boot) + SIZEOF(.boot.text);

    /* Switch to higher-half virtual addresses */
    /* Use Linux kernel pattern: VMA at high address, LMA continues after boot */
    . = KERNEL_VMA + __boot_phys_end;

    /* Main kernel sections - VMA in higher half, LMA continues from boot */
    /* All kernel sections go in one segment to avoid overlaps */
    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VMA)
    {
        *(.text .text.*)
    } :kernel

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VMA)
    {
        *(.rodata .rodata.*)
        *(.lrodata .lrodata.*)  /* Medium/large code model rodata */
    } :kernel

    .rune ALIGN(4K) : AT(ADDR(.rune) - KERNEL_VMA)
    {
        PROVIDE(__rune_start = .);
        *(.rune .rune.*)
        PROVIDE(__rune_end = .);
    } :kernel

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VMA)
    {
        *(.data .data.*)
    } :kernel

    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VMA)
    {
        PROVIDE(__bss_start = .);
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(__bss_end = .);
    } :kernel

    .tdata ALIGN(4K) : AT(ADDR(.tdata) - KERNEL_VMA)
    {
        *(.tdata .tdata.*)
    } :kernel

    .tbss ALIGN(4K) : AT(ADDR(.tbss) - KERNEL_VMA)
    {
        *(.tbss .tbss.*)
    } :kernel

    .eh_frame_hdr ALIGN(4) : AT(ADDR(.eh_frame_hdr) - KERNEL_VMA)
    {
        *(.eh_frame_hdr)
    } :kernel

    .eh_frame ALIGN(4) : AT(ADDR(.eh_frame) - KERNEL_VMA)
    {
        *(.eh_frame)
    } :kernel

    /* Discard sections we don't need */
    /DISCARD/ :
    {
        *(.comment)
        *(.note .note.*)
    }
}
