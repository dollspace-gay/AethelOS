//! # System Call Infrastructure
//!
//! The bridge between Ring 3 (user mode) and Ring 0 (kernel mode).
//! When a Vessel needs kernel services, it invokes a system call via the
//! syscall instruction (not INT 0x80).
//!
//! ## Philosophy
//! System calls are not commands but requests. The kernel is not a servant
//! but a guardian. Each request is evaluated against the Vessel's Fate
//! (RBAC permissions) and granted only if harmonious with the system.
//!
//! ## Architecture
//! - syscall instruction triggers fast system call entry
//! - RAX contains syscall number
//! - RDI, RSI, RDX, R10, R8, R9 contain arguments (up to 6)
//! - Return value in RAX
//! - Error codes are negative (Linux convention)
//!
//! ## Calling Convention
//! User space:
//! ```asm
//! mov rax, SYSCALL_NUMBER
//! mov rdi, arg1
//! mov rsi, arg2
//! mov rdx, arg3
//! mov r10, arg4  ; Note: r10, not rcx (rcx is clobbered by syscall instruction)
//! mov r8,  arg5
//! mov r9,  arg6
//! syscall
//! ; Result in rax
//! ```

use super::ThreadId;
use x86_64::VirtAddr;
use x86_64::registers::model_specific::{Efer, EferFlags, LStar, Msr};

/// Initialize syscall/sysret mechanism
///
/// This configures the MSRs needed for fast system calls:
/// - IA32_EFER: Enable syscall/sysret extension
/// - IA32_STAR: Configure segment selectors for ring transitions
/// - IA32_LSTAR: Set syscall entry point
/// - IA32_FMASK: Mask RFLAGS on entry
///
/// # Safety
///
/// Must be called AFTER GDT setup with proper segment selectors.
/// Must be called AFTER per-CPU data (GS register) is initialized.
///
/// # Panics
///
/// Panics if called before GDT or per-CPU data initialization.
pub unsafe fn init_syscall() {
    crate::serial_println!("[SYSCALL] Initializing syscall/sysret mechanism...");

    // Step 1: Enable syscall/sysret in EFER
    let efer = Efer::read();
    Efer::write(efer | EferFlags::SYSTEM_CALL_EXTENSIONS);
    crate::serial_println!("[SYSCALL]   ✓ SCE bit enabled in IA32_EFER");

    // Step 2: Configure STAR - segment selectors
    // Bits 32:47 = kernel code segment (0x08)
    // Bits 48:63 = user segment base (NOT the final selector!)
    //   sysret calculates: SS = base + 8, CS = base + 16
    // With our GDT (user_data=0x18, user_code=0x20), base must be 0x10:
    //   0x10 + 8 = 0x18 (USER_DATA/SS)
    //   0x10 + 16 = 0x20 (USER_CODE/CS)
    let mut star = Msr::new(0xC0000081);  // IA32_STAR
    star.write(0x0010_0008_0000_0000u64);
    crate::serial_println!("[SYSCALL]   ✓ IA32_STAR configured (kernel CS=0x08, user base=0x10)");

    // Step 3: Set LSTAR - syscall entry point
    let entry_addr = syscall_entry as *const () as u64;
    LStar::write(VirtAddr::new(entry_addr));
    crate::serial_println!("[SYSCALL]   ✓ IA32_LSTAR set to {:#x}", entry_addr);

    // Step 4: Configure SFMASK - mask RFLAGS on entry
    // Clear: IF (0x200) = disable interrupts during stack setup
    //        DF (0x400) = direction flag must be clear per ABI
    //        TF (0x100) = trap flag
    let mut sfmask = Msr::new(0xC0000084);  // IA32_FMASK
    sfmask.write(0x0700u64);
    crate::serial_println!("[SYSCALL]   ✓ IA32_FMASK configured (mask IF, DF, TF)");

    crate::serial_println!("[SYSCALL] ✓ Syscall/sysret mechanism ready");
}

/// Saved register state on kernel stack
///
/// This matches the order that registers are pushed in syscall_entry.
/// The layout must stay in sync with the naked assembly!
#[repr(C)]
struct SavedRegisters {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,  // RFLAGS (saved by CPU)
    r10: u64,  // arg4
    r9: u64,   // arg6
    r8: u64,   // arg5
    rbp: u64,
    rdi: u64,  // arg1
    rsi: u64,  // arg2
    rdx: u64,  // arg3
    rcx: u64,  // return RIP (saved by CPU)
    rbx: u64,
    rax: u64,  // syscall number
}

/// Rust syscall handler (called from naked assembly)
///
/// # Arguments
/// * `regs` - Pointer to saved register state on kernel stack
///
/// # Returns
/// Result in RAX (positive for success, negative for error)
///
/// # Safety
/// Must only be called from syscall_entry with valid register state.
unsafe extern "C" fn syscall_handler_rust(regs: *const SavedRegisters) -> i64 {
    let regs = &*regs;

    // Extract arguments from saved registers
    let syscall_num = regs.rax;
    let arg1 = regs.rdi;
    let arg2 = regs.rsi;
    let arg3 = regs.rdx;
    let arg4 = regs.r10;  // Note: r10, not rcx!
    let arg5 = regs.r8;
    let arg6 = regs.r9;

    crate::serial_println!(
        "[SYSCALL] syscall {} called with args: {:#x}, {:#x}, {:#x}",
        syscall_num, arg1, arg2, arg3
    );

    // Dispatch to syscall implementation
    dispatch_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5, arg6)
}

/// Naked syscall entry point
///
/// This is the function that the CPU jumps to when userspace executes 'syscall'.
/// It handles:
/// 1. Swapping to kernel GS (per-CPU data)
/// 2. Switching from user stack to kernel stack
/// 3. Saving all registers
/// 4. Calling the Rust dispatcher
/// 5. Restoring registers
/// 6. Returning to userspace via sysretq
///
/// # Register State on Entry
/// - RAX: syscall number
/// - RDI, RSI, RDX, R10, R8, R9: arguments 1-6
/// - RCX: return RIP (saved by CPU)
/// - R11: RFLAGS (saved by CPU)
/// - RSP: user stack pointer
///
/// # Safety
/// This function uses inline assembly and directly manipulates CPU state.
/// The per-CPU data structure offsets must match PerCpuData layout exactly.
#[unsafe(naked)]
extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // === GUARD: Detect spurious kernel-mode syscall ===
        // If RSP is in kernel address space (>= 0xFFFF800000000000),
        // this syscall was triggered from kernel mode - abort!
        "test rsp, rsp",                // Test if RSP is negative (kernel space)
        "js 99f",                        // Jump to error handler if kernel mode

        // === CRITICAL: swapgs FIRST ===
        // Swap GS base to access per-CPU kernel data
        "swapgs",

        // === Save user RSP, load kernel RSP ===
        // SECURITY: User RSP could be malicious, validate before use!
        "mov gs:[16], rsp",     // Save user RSP at offset 16 (user_stack_saved)
        "mov rsp, gs:[8]",      // Load kernel RSP from offset 8 (kernel_stack_top)

        // DEBUG: Print 'S' to serial port AFTER stack switch (safe with SMAP)
        "push rax",
        "push rdx",
        "mov dx, 0x3f8",
        "mov al, 0x53",  // 'S'
        "out dx, al",
        "pop rdx",
        "pop rax",

        // === Build interrupt-like frame on kernel stack ===
        // User SS: 0x18 | 3 = 0x1B (USER_DATA with RPL=3)
        // User CS: 0x20 | 3 = 0x23 (USER_CODE with RPL=3)
        "push 0x1B",              // User SS (0x18 | 3)
        "push qword ptr gs:[16]", // User RSP
        "push r11",               // RFLAGS (saved by CPU)
        "push 0x23",              // User CS (0x20 | 3)
        "push rcx",               // Return RIP (saved by CPU)

        // === Save all general-purpose registers ===
        "push rax",  // syscall number
        "push rbx",
        "push rcx",  // return RIP (duplicate for easier access)
        "push rdx",  // arg3
        "push rsi",  // arg2
        "push rdi",  // arg1
        "push rbp",
        "push r8",   // arg5
        "push r9",   // arg6
        "push r10",  // arg4
        "push r11",  // RFLAGS (duplicate)
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // === Call Rust syscall dispatcher ===
        "mov rdi, rsp",          // Pass pointer to SavedRegisters
        "call {handler}",        // Returns result in RAX

        // === Restore registers ===
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",  // RFLAGS (restore into r11 for sysretq)
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",  // Return RIP (restore into rcx for sysretq)
        "pop rbx",
        // Skip RAX - it contains the return value!
        "add rsp, 8",

        // === Clean up interrupt frame ===
        "pop rcx",   // User return RIP
        "add rsp, 8", // Skip user CS (sysretq sets CS from STAR MSR)
        "pop r11",   // User RFLAGS
        "pop rsp",   // Restore user RSP (sysretq sets SS from STAR MSR)
        // Note: User SS is now unreachable on kernel stack, but that's OK -
        // sysretq will set SS automatically to (STAR[63:48] + 8) | 3

        // === Return to userspace ===
        "swapgs",    // Restore user GS
        "sysretq",   // Return to ring 3 (jumps to RCX, restores RFLAGS from R11)

        // === ERROR HANDLER: Spurious kernel-mode syscall ===
        "99:",
        // We detected a syscall from kernel mode (RSP in kernel space)
        // This should never happen in normal operation!
        // Debug: output 'K' to serial to indicate kernel-mode syscall detected
        "push rax",
        "push rdx",
        "mov dx, 0x3f8",
        "mov al, 75",    // 'K' = Kernel-mode syscall detected!
        "out dx, al",
        "pop rdx",
        "pop rax",
        // Return immediately without touching GS or stack
        "mov rax, -38",  // -ENOSYS (invalid syscall)
        // We can't use sysretq because we're in kernel mode
        // Instead, return via the address in RCX (which syscall saved)
        "jmp rcx",       // Jump back to caller (dangerous but better than corrupting state)

        handler = sym syscall_handler_rust,
    )
}

/// System call numbers (AethelOS ABI)
///
/// These are the magical numbers that identify each system call.
/// They are stable across kernel versions (ABI compatibility).
pub mod syscall_numbers {
    pub const SYS_YIELD: u64 = 0;   // Yield CPU to another thread
    pub const SYS_WRITE: u64 = 1;   // Write to file descriptor
    pub const SYS_EXIT: u64 = 2;    // Exit current thread
    pub const SYS_GETPID: u64 = 3;  // Get process (Vessel) ID
    pub const SYS_GETTID: u64 = 4;  // Get thread ID
}

/// System call result type
///
/// Success returns a non-negative value.
/// Errors return negative errno values (Linux-style).
pub type SyscallResult = i64;

/// System call error codes
#[repr(i64)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallError {
    /// Invalid syscall number
    ENOSYS = -38,

    /// Bad address (pointer validation failed)
    EFAULT = -14,

    /// Invalid argument
    EINVAL = -22,

    /// Permission denied
    EPERM = -1,

    /// No such process
    ESRCH = -3,
}

impl From<SyscallError> for SyscallResult {
    fn from(err: SyscallError) -> Self {
        err as i64
    }
}

/// Dispatch a system call based on the syscall number and arguments
///
/// This is called from the INT 0x80 handler with the thread's register state.
///
/// # Arguments
/// * `syscall_num` - The syscall number (from RAX)
/// * `arg1` - First argument (from RDI)
/// * `arg2` - Second argument (from RSI)
/// * `arg3` - Third argument (from RDX)
/// * `arg4` - Fourth argument (from R10)
/// * `arg5` - Fifth argument (from R8)
/// * `arg6` - Sixth argument (from R9)
///
/// # Returns
/// The result to return in RAX (positive for success, negative for error)
///
/// # Safety
/// This function must be called from kernel mode with valid arguments.
/// Pointer arguments must be validated before dereferencing.
pub unsafe fn dispatch_syscall(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    match syscall_num {
        syscall_numbers::SYS_YIELD => sys_yield(),
        syscall_numbers::SYS_WRITE => sys_write(arg1, arg2, arg3),
        syscall_numbers::SYS_EXIT => sys_exit(arg1 as i32),
        syscall_numbers::SYS_GETPID => sys_getpid(),
        syscall_numbers::SYS_GETTID => sys_gettid(),
        _ => SyscallError::ENOSYS.into(),
    }
}

/// SYS_YIELD: Yield the CPU to another thread
///
/// This allows cooperative multitasking from user space.
/// The thread voluntarily gives up its time slice.
///
/// # Returns
/// Always returns 0 (success)
fn sys_yield() -> SyscallResult {
    // Call the existing yield_now() function
    super::yield_now();
    0
}

/// SYS_WRITE: Write data to a file descriptor
///
/// # Arguments
/// * `fd` - File descriptor (1 = stdout, 2 = stderr)
/// * `buf` - Pointer to buffer in user space
/// * `count` - Number of bytes to write
///
/// # Returns
/// Number of bytes written on success, negative error code on failure
///
/// # Safety
/// Validates that buf is a valid user-space pointer before accessing.
unsafe fn sys_write(fd: u64, buf: u64, count: u64) -> SyscallResult {
    // Validate file descriptor
    if fd != 1 && fd != 2 {
        // Only stdout (1) and stderr (2) are supported for now
        return SyscallError::EINVAL.into();
    }

    // Validate count
    if count > 4096 {
        // Prevent excessively large writes
        return SyscallError::EINVAL.into();
    }

    // Validate buffer pointer (must be in user space < 0x8000_0000_0000)
    if buf >= 0x8000_0000_0000 {
        return SyscallError::EFAULT.into();
    }

    // TODO(phase-3): Use sanctified_copy_from_mortal to safely read user buffer
    // For now, we'll use a direct access with STAC/CLAC

    // Temporarily disable SMAP to access user memory
    core::arch::asm!("stac", options(nomem, nostack, preserves_flags));

    // DEBUG: Mark that STAC completed
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") b'T',
        options(nomem, nostack, preserves_flags)
    );

    // Read bytes from user buffer and write to console
    let bytes_written = {
        // DEBUG: Mark before creating slice
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'B',
            options(nomem, nostack, preserves_flags)
        );

        let slice = core::slice::from_raw_parts(buf as *const u8, count as usize);

        // DEBUG: Mark after creating slice
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'A',
            options(nomem, nostack, preserves_flags)
        );

        // Write directly to serial port (avoid VGA/print! which may cause page faults)
        for &byte in slice {
            // Output via direct port I/O to avoid print infrastructure
            unsafe {
                core::arch::asm!(
                    "out dx, al",
                    in("dx") 0x3f8u16,
                    in("al") byte,
                    options(nomem, nostack, preserves_flags)
                );
            }
        }

        count as i64
    };

    // Re-enable SMAP protection
    core::arch::asm!("clac", options(nomem, nostack, preserves_flags));

    bytes_written
}

/// SYS_EXIT: Exit the current thread
///
/// # Arguments
/// * `exit_code` - Exit status code
///
/// # Returns
/// Never returns (thread is terminated)
fn sys_exit(_exit_code: i32) -> SyscallResult {
    // Output simple debug marker via direct serial port I/O
    // (avoid print! macros which can cause page faults in syscall context)
    unsafe {
        let msg = b"[EXIT]\n";
        for &byte in msg {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    // Mark the current thread as Fading so it won't be scheduled again
    use super::thread::ThreadState;

    super::without_interrupts(|| {
        unsafe {
            let loom = super::get_loom();
            let mut loom_lock = loom.lock();

            // Get current thread ID
            if let Some(current_tid) = loom_lock.current_thread_id() {
                // Find the thread and mark it as Fading
                if let Some(thread) = loom_lock.threads.iter_mut().find(|t| t.id() == current_tid) {
                    thread.set_state(ThreadState::Fading);

                    // Debug output
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3f8u16,
                        in("al") b'[',
                        options(nomem, nostack, preserves_flags)
                    );
                    let msg = b"FADING]\n";
                    for &byte in msg {
                        core::arch::asm!(
                            "out dx, al",
                            in("dx") 0x3f8u16,
                            in("al") byte,
                            options(nomem, nostack, preserves_flags)
                        );
                    }
                }
            }
        }
    });

    // Yield one final time to switch to another thread
    // This thread will never be scheduled again because it's marked as Fading
    super::yield_now();

    // We should never reach here, but just in case, loop forever
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

/// SYS_GETPID: Get the current process (Vessel) ID
///
/// # Returns
/// The VesselId of the current thread's Vessel, or 0 if not in a Vessel
fn sys_getpid() -> SyscallResult {
    // Get current thread
    let current_tid = super::current_thread();

    if let Some(tid) = current_tid {
        // TODO(phase-3): Look up the Vessel for this thread
        // For now, return a dummy value
        crate::serial_println!("SYS_GETPID called for thread {:?}", tid);
        1 // Dummy PID
    } else {
        0
    }
}

/// SYS_GETTID: Get the current thread ID
///
/// # Returns
/// The ThreadId of the current thread
fn sys_gettid() -> SyscallResult {
    let current_tid = super::current_thread();

    if let Some(tid) = current_tid {
        tid.0 as i64
    } else {
        0
    }
}

/// Validate a user-space pointer
///
/// Ensures the pointer is in user space and properly aligned.
///
/// # Arguments
/// * `ptr` - The pointer to validate
/// * `len` - The length of the region to access
///
/// # Returns
/// Ok(()) if valid, Err(SyscallError) if invalid
#[allow(dead_code)]
fn validate_user_pointer(ptr: u64, len: usize) -> Result<(), SyscallError> {
    // Check that pointer is in user space (< 0x8000_0000_0000)
    if ptr >= 0x8000_0000_0000 {
        return Err(SyscallError::EFAULT);
    }

    // Check for overflow
    if ptr.checked_add(len as u64).is_none() {
        return Err(SyscallError::EFAULT);
    }

    // Check that end is still in user space
    if ptr + len as u64 >= 0x8000_0000_0000 {
        return Err(SyscallError::EFAULT);
    }

    Ok(())
}
