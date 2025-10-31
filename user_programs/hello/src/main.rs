#![no_std]
#![no_main]

// Syscall numbers (from heartwood/src/loom_of_fate/syscalls.rs)
const SYS_WRITE: u64 = 1;
const SYS_EXIT: u64 = 2;  // AethelOS uses 2, not Linux's 60

/// Raw syscall with 3 arguments
#[unsafe(naked)]
unsafe extern "C" fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    core::arch::naked_asm!(
        "mov rax, rdi",  // syscall number
        "mov rdi, rsi",  // arg1
        "mov rsi, rdx",  // arg2
        "mov rdx, rcx",  // arg3
        "syscall",
        "ret",
    );
}

/// Write to file descriptor
fn write(fd: u64, buf: &[u8]) -> i64 {
    unsafe {
        syscall3(SYS_WRITE, fd, buf.as_ptr() as u64, buf.len() as u64)
    }
}

/// Exit with code
fn exit(code: u64) -> ! {
    unsafe {
        syscall3(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

/// Entry point for user space program
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from user space! (Ring 3)\n";
    write(1, msg);
    exit(0)
}

/// Panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1)
}
