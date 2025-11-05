#![no_std]
#![no_main]

use corelib::syscalls::{sys_exit, sys_write};
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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Test the syscall interface
    // When syscalls are actually implemented in the kernel, this will write to stdout
    let msg = b"Hello from AethelOS userspace!\n";
    let _ = sys_write(1, msg);  // Ignore result for now (syscalls not implemented yet)

    // Exit with code 42
    sys_exit(42);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sys_exit(1);
}
