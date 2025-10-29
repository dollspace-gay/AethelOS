//! Kernel Section Boundaries
//!
//! Provides access to linker-defined symbols that mark kernel section boundaries.
//! These are essential for ensuring all kernel sections are properly mapped in page tables.

extern "C" {
    /// Start of kernel (beginning of .requests section)
    static __kernel_start: u8;

    /// Start of .text section
    static __text_start: u8;

    /// Start of .data section
    static __data_start: u8;

    /// Start of .bss section
    static __bss_start: u8;

    /// End of .bss section
    static __bss_end: u8;

    /// End of entire kernel
    static __kernel_end: u8;
}

/// Get the virtual address where the kernel starts
pub fn kernel_start() -> usize {
    unsafe { &__kernel_start as *const _ as usize }
}

/// Get the virtual address where the kernel ends (including .bss)
pub fn kernel_end() -> usize {
    unsafe { &__kernel_end as *const _ as usize }
}

/// Get the virtual address where .bss starts
pub fn bss_start() -> usize {
    unsafe { &__bss_start as *const _ as usize }
}

/// Get the virtual address where .bss ends
pub fn bss_end() -> usize {
    unsafe { &__bss_end as *const _ as usize }
}

/// Get the total size of the kernel in bytes
pub fn kernel_size() -> usize {
    kernel_end() - kernel_start()
}

/// Get the size of .bss section in bytes
pub fn bss_size() -> usize {
    bss_end() - bss_start()
}

/// Print kernel section information for debugging
pub fn print_kernel_sections() {
    unsafe fn serial_out(c: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") c,
            options(nomem, nostack, preserves_flags)
        );
    }

    unsafe fn serial_str(s: &str) {
        for byte in s.bytes() {
            serial_out(byte);
        }
    }

    unsafe fn print_hex(val: usize) {
        for i in (0..16).rev() {
            let nibble = ((val >> (i * 4)) & 0xF) as u8;
            let ch = if nibble < 10 { b'0' + nibble } else { b'a' + (nibble - 10) };
            serial_out(ch);
        }
    }

    unsafe fn print_dec(mut val: usize) {
        if val == 0 {
            serial_out(b'0');
            return;
        }
        let mut divisor = 1_000_000_000;
        let mut started = false;
        while divisor > 0 {
            let digit = val / divisor;
            if digit > 0 || started {
                serial_out(b'0' + digit as u8);
                started = true;
            }
            val %= divisor;
            divisor /= 10;
        }
    }

    let start = kernel_start();
    let end = kernel_end();
    let size = kernel_size();
    let bss_sz = bss_size();

    unsafe {
        serial_str("[KERNEL] Section Boundaries:\n");
        serial_str("  Start:  0x");
        print_hex(start);
        serial_str("\n  End:    0x");
        print_hex(end);
        serial_str("\n  Size:   ");
        print_dec(size);
        serial_str(" bytes (");
        print_dec(size / 1024 / 1024);
        serial_str(" MB)\n  .bss:   ");
        print_dec(bss_sz);
        serial_str(" bytes (");
        print_dec(bss_sz / 1024 / 1024);
        serial_str(" MB)\n");
    }
}
