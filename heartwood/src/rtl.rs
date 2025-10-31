//! Runtime Library
//!
//! Provides essential C-like functions that the compiler expects.
//! These replace the broken or missing functions from compiler_builtins.

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Test function to verify rtl module is linked
#[no_mangle]
pub unsafe extern "C" fn rtl_test() {
    serial_out(b'R');
    serial_out(b'T');
    serial_out(b'L');
}

// DISABLED: Let compiler-builtins-mem provide these functions
// Our custom implementations may conflict with optimized compiler-builtins versions
/*
/// Simple, correct memcpy implementation
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    serial_out(b'M'); // Entered memcpy
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
        if i % 1000 == 0 {
            serial_out(b'.'); // Progress marker
        }
    }
    serial_out(b'm'); // Exiting memcpy
    dest
}

/// Memory move (handles overlapping regions)
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest {
        // Copy from back to front to handle overlap
        let mut i = n;
        while i != 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    } else {
        // Copy from front to back
        let mut i = 0;
        while i < n {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
    }
    dest
}

/// Memory set
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

/// Memory compare
#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}
*/
