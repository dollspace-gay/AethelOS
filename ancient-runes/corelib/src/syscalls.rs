//! # Syscall Interface for AethelOS
//!
//! This module provides the low-level interface between userspace programs
//! and the AethelOS kernel via the syscall/sysret mechanism.
//!
//! ## Architecture
//!
//! AethelOS uses the modern x86-64 `syscall`/`sysret` instructions for
//! fast system calls. This is significantly faster than the older INT 0x80
//! mechanism (~60-80 cycles vs ~100-300 cycles).
//!
//! ## Calling Convention
//!
//! - **RAX**: Syscall number (input) / Return value (output)
//! - **RDI**: Argument 1
//! - **RSI**: Argument 2
//! - **RDX**: Argument 3
//! - **R10**: Argument 4 (NOT RCX - clobbered by syscall!)
//! - **R8**:  Argument 5
//! - **R9**:  Argument 6
//! - **RCX**: Clobbered (return RIP)
//! - **R11**: Clobbered (RFLAGS)
//!
//! ## Return Values
//!
//! - **>= 0**: Success (return value)
//! - **< 0**:  Error code (negative errno)
//!
//! ## Safety
//!
//! The low-level syscall wrapper functions are marked `unsafe` because:
//! 1. They use inline assembly
//! 2. Incorrect syscall numbers or arguments can cause kernel panics
//! 3. Invalid pointers passed to kernel can compromise system integrity
//!
//! The high-level wrappers provide safe interfaces with proper validation.

#![allow(dead_code)]

// ============================================================================
// Syscall Numbers
// ============================================================================

/// Exit the current thread/process
pub const SYS_EXIT: u64 = 0;

/// Write bytes to a file descriptor
pub const SYS_WRITE: u64 = 1;

/// Read bytes from a file descriptor
pub const SYS_READ: u64 = 2;

/// Open a file/scroll
pub const SYS_OPEN: u64 = 3;

/// Close a file descriptor
pub const SYS_CLOSE: u64 = 4;

/// Get current thread ID
pub const SYS_GETPID: u64 = 5;

/// Query a scroll in the World-Tree
pub const SYS_QUERY: u64 = 6;

/// Commit changes to the World-Tree
pub const SYS_COMMIT: u64 = 7;

/// Allocate memory pages
pub const SYS_MMAP: u64 = 8;

/// Free memory pages
pub const SYS_MUNMAP: u64 = 9;

/// Sleep for a duration (in heartbeats/ticks)
pub const SYS_SLEEP: u64 = 10;

/// Yield CPU to another thread
pub const SYS_YIELD: u64 = 11;

/// Create a new thread
pub const SYS_THREAD_CREATE: u64 = 12;

/// Wait for a thread to terminate
pub const SYS_THREAD_JOIN: u64 = 13;

/// Send an IPC message
pub const SYS_IPC_SEND: u64 = 14;

/// Receive an IPC message
pub const SYS_IPC_RECV: u64 = 15;

/// Get current time (in heartbeats since boot)
pub const SYS_TIME: u64 = 16;

/// Execute a Glimmer-Weave script (Ring 1 only)
pub const SYS_EXEC_SCRIPT: u64 = 17;

// ============================================================================
// Error Codes (POSIX-like for compatibility)
// ============================================================================

/// Operation not permitted
pub const EPERM: i32 = -1;

/// No such file or directory
pub const ENOENT: i32 = -2;

/// No such process
pub const ESRCH: i32 = -3;

/// Interrupted system call
pub const EINTR: i32 = -4;

/// I/O error
pub const EIO: i32 = -5;

/// Bad file descriptor
pub const EBADF: i32 = -9;

/// Try again
pub const EAGAIN: i32 = -11;

/// Out of memory
pub const ENOMEM: i32 = -12;

/// Permission denied
pub const EACCES: i32 = -13;

/// Bad address
pub const EFAULT: i32 = -14;

/// Device or resource busy
pub const EBUSY: i32 = -16;

/// File exists
pub const EEXIST: i32 = -17;

/// Invalid argument
pub const EINVAL: i32 = -22;

/// Too many open files
pub const EMFILE: i32 = -24;

/// Function not implemented
pub const ENOSYS: i32 = -38;

/// Directory not empty
pub const ENOTEMPTY: i32 = -39;

/// Operation would block
pub const EWOULDBLOCK: i32 = EAGAIN;

// ============================================================================
// Low-Level Syscall Wrappers
// ============================================================================

/// Make a syscall with 0 arguments
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - The syscall does not require arguments
#[inline(always)]
pub unsafe fn syscall0(num: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        lateout("rax") ret,
        out("rcx") _,   // Clobbered by syscall
        out("r11") _,   // Clobbered by syscall
        options(nostack, preserves_flags)
    );
    ret
}

/// Make a syscall with 1 argument
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - `arg1` is valid for the syscall
#[inline(always)]
pub unsafe fn syscall1(num: u64, arg1: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

/// Make a syscall with 2 arguments
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - `arg1`, `arg2` are valid for the syscall
#[inline(always)]
pub unsafe fn syscall2(num: u64, arg1: u64, arg2: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

/// Make a syscall with 3 arguments
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - `arg1`, `arg2`, `arg3` are valid for the syscall
#[inline(always)]
pub unsafe fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

/// Make a syscall with 4 arguments
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - `arg1`, `arg2`, `arg3`, `arg4` are valid for the syscall
///
/// # Note
///
/// Argument 4 goes in R10, NOT RCX (which is clobbered by syscall)
#[inline(always)]
pub unsafe fn syscall4(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,  // Note: R10, not RCX!
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

/// Make a syscall with 5 arguments
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - All arguments are valid for the syscall
#[inline(always)]
pub unsafe fn syscall5(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

/// Make a syscall with 6 arguments
///
/// # Safety
///
/// Caller must ensure:
/// - `num` is a valid syscall number
/// - All arguments are valid for the syscall
#[inline(always)]
pub unsafe fn syscall6(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

// ============================================================================
// High-Level Safe Syscall Wrappers
// ============================================================================

/// Exit the current thread/process with the given exit code
///
/// This function never returns.
pub fn sys_exit(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as u64);
    }
    // If syscall returns (shouldn't happen), loop forever
    loop {
        core::hint::spin_loop();
    }
}

/// Write bytes to a file descriptor
///
/// # Arguments
///
/// * `fd` - File descriptor (0=stdin, 1=stdout, 2=stderr)
/// * `buf` - Bytes to write
///
/// # Returns
///
/// * `Ok(n)` - Number of bytes written
/// * `Err(errno)` - Error code
pub fn sys_write(fd: i32, buf: &[u8]) -> Result<usize, i32> {
    let ret = unsafe {
        syscall3(SYS_WRITE, fd as u64, buf.as_ptr() as u64, buf.len() as u64)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(ret as usize)
    }
}

/// Read bytes from a file descriptor
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `buf` - Buffer to read into
///
/// # Returns
///
/// * `Ok(n)` - Number of bytes read (0 = EOF)
/// * `Err(errno)` - Error code
pub fn sys_read(fd: i32, buf: &mut [u8]) -> Result<usize, i32> {
    let ret = unsafe {
        syscall3(SYS_READ, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(ret as usize)
    }
}

/// Yield the CPU to another thread
///
/// This is a cooperative scheduling hint to the Loom of Fate.
pub fn sys_yield() {
    unsafe {
        syscall0(SYS_YIELD);
    }
}

/// Sleep for the given number of heartbeats (timer ticks)
///
/// # Arguments
///
/// * `ticks` - Number of timer ticks to sleep (typically 1000 ticks/second)
///
/// # Returns
///
/// * `Ok(())` - Sleep completed successfully
/// * `Err(errno)` - Error code (e.g., EINTR if interrupted)
pub fn sys_sleep(ticks: u64) -> Result<(), i32> {
    let ret = unsafe {
        syscall1(SYS_SLEEP, ticks)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(())
    }
}

/// Get the current thread ID
///
/// # Returns
///
/// The thread ID of the calling thread
pub fn sys_getpid() -> u64 {
    unsafe {
        syscall0(SYS_GETPID) as u64
    }
}

/// Get the current time in heartbeats (timer ticks since boot)
///
/// # Returns
///
/// Number of timer ticks since system boot
pub fn sys_time() -> u64 {
    unsafe {
        syscall0(SYS_TIME) as u64
    }
}

/// Open a file/scroll
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `flags` - Open flags (e.g., read, write, create)
///
/// # Returns
///
/// * `Ok(fd)` - File descriptor
/// * `Err(errno)` - Error code
pub fn sys_open(path: &str, flags: i32) -> Result<i32, i32> {
    let ret = unsafe {
        syscall2(SYS_OPEN, path.as_ptr() as u64, flags as u64)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(ret as i32)
    }
}

/// Close a file descriptor
///
/// # Arguments
///
/// * `fd` - File descriptor to close
///
/// # Returns
///
/// * `Ok(())` - File closed successfully
/// * `Err(errno)` - Error code
pub fn sys_close(fd: i32) -> Result<(), i32> {
    let ret = unsafe {
        syscall1(SYS_CLOSE, fd as u64)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(())
    }
}

// ============================================================================
// Error Code Utilities
// ============================================================================

/// Convert an error code to a human-readable string
pub fn errno_str(errno: i32) -> &'static str {
    match errno {
        EPERM => "Operation not permitted",
        ENOENT => "No such file or directory",
        ESRCH => "No such process",
        EINTR => "Interrupted system call",
        EIO => "I/O error",
        EBADF => "Bad file descriptor",
        EAGAIN => "Resource temporarily unavailable",
        ENOMEM => "Out of memory",
        EACCES => "Permission denied",
        EFAULT => "Bad address",
        EBUSY => "Device or resource busy",
        EEXIST => "File exists",
        EINVAL => "Invalid argument",
        EMFILE => "Too many open files",
        ENOSYS => "Function not implemented",
        ENOTEMPTY => "Directory not empty",
        _ => "Unknown error",
    }
}

// ============================================================================
// Tests (for when test infrastructure exists)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_errno_str() {
        assert_eq!(errno_str(EPERM), "Operation not permitted");
        assert_eq!(errno_str(ENOMEM), "Out of memory");
        assert_eq!(errno_str(-999), "Unknown error");
    }
}
