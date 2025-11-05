//! # Test Service - Minimal Ring 1 Service for AethelOS
//!
//! This is a minimal Ring 1 service used to validate the Grove Manager
//! and service loading infrastructure. It demonstrates:
//! - Ring 1 execution
//! - Syscall interface usage
//! - Cooperative scheduling with sys_yield
//! - Basic service lifecycle

#![no_std]
#![no_main]

use corelib::syscalls::{sys_exit, sys_write, sys_yield, sys_getpid, sys_time};
use core::alloc::{GlobalAlloc, Layout};

/// Dummy allocator for minimal programs
/// (Real allocator would use sys_mmap/sys_munmap)
struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Do nothing
    }
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

/// Simple byte-to-hex conversion for debugging
fn byte_to_hex(b: u8, buf: &mut [u8; 2]) {
    const HEX_CHARS: &[u8] = b"0123456789ABCDEF";
    buf[0] = HEX_CHARS[(b >> 4) as usize];
    buf[1] = HEX_CHARS[(b & 0x0F) as usize];
}

/// Convert u64 to hex string (for debugging without alloc)
fn u64_to_hex(mut n: u64, buf: &mut [u8; 16]) {
    for i in (0..16).rev() {
        buf[i] = b"0123456789ABCDEF"[(n & 0xF) as usize];
        n >>= 4;
    }
}

/// Entry point for the Ring 1 test service
///
/// This service will:
/// 1. Print a startup message
/// 2. Display its thread ID
/// 3. Run a loop yielding to other threads
/// 4. Exit cleanly after a number of iterations
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Startup message
    let startup_msg = b"[Test Service] Ring 1 service starting...\n";
    let _ = sys_write(1, startup_msg);

    // Get and display thread ID
    let tid = sys_getpid();
    let mut tid_buf = [0u8; 16];
    u64_to_hex(tid, &mut tid_buf);

    let _ = sys_write(1, b"[Test Service] Thread ID: 0x");
    let _ = sys_write(1, &tid_buf);
    let _ = sys_write(1, b"\n");

    // Get and display boot time
    let boot_time = sys_time();
    let mut time_buf = [0u8; 16];
    u64_to_hex(boot_time, &mut time_buf);

    let _ = sys_write(1, b"[Test Service] Boot time: 0x");
    let _ = sys_write(1, &time_buf);
    let _ = sys_write(1, b" ticks\n");

    // Main service loop - demonstrate cooperative scheduling
    let _ = sys_write(1, b"[Test Service] Entering main loop (10 iterations)...\n");

    for i in 0..10 {
        // Display iteration number
        let mut iter_buf = [0u8; 2];
        byte_to_hex(i as u8, &mut iter_buf);

        let _ = sys_write(1, b"[Test Service] Iteration 0x");
        let _ = sys_write(1, &iter_buf);
        let _ = sys_write(1, b" - yielding CPU\n");

        // Yield to other threads (cooperative multitasking)
        sys_yield();
    }

    // Service completed successfully
    let _ = sys_write(1, b"[Test Service] Service completed successfully!\n");
    let _ = sys_write(1, b"[Test Service] Exiting with code 0\n");

    // Exit cleanly
    sys_exit(0);
}

/// Panic handler - called when the service panics
///
/// This should ideally send an IPC message to the Grove Manager
/// to report the crash, but for now we just exit with an error code.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Try to write panic info to stderr (fd 2)
    let _ = sys_write(2, b"[Test Service] PANIC: ");

    // Extract panic message if available
    if let Some(location) = info.location() {
        let _ = sys_write(2, b"at ");
        let _ = sys_write(2, location.file().as_bytes());
        let _ = sys_write(2, b":");

        // Convert line number to string (simplified)
        let line = location.line();
        let mut line_buf = [0u8; 16];
        u64_to_hex(line as u64, &mut line_buf);
        let _ = sys_write(2, &line_buf);
    }

    let _ = sys_write(2, b"\n");
    let _ = sys_write(2, b"[Test Service] Exiting with code 1\n");

    sys_exit(1);
}
