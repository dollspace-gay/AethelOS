//! # Thread Context
//!
//! The essence of a thread - its complete state at a moment in time.
//! When we preserve context, we capture the thread's entire being:
//! every register, every thought, every intention.
//!
//! ## Philosophy
//! Context switches are not interruptions, but transitions.
//! We preserve the current moment with reverence, knowing we will
//! restore it faithfully when the thread's turn returns.

use core::arch::asm;

/// The complete state of a thread
///
/// This structure holds all CPU registers needed to resume execution
/// exactly where the thread left off. The layout matches x86-64 calling
/// conventions and interrupt frames.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ThreadContext {
    // General purpose registers (callee-saved)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,

    // Additional general purpose registers
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,

    // Special registers
    pub rip: u64,      // Instruction pointer - where to resume
    pub cs: u64,       // Code segment
    pub rflags: u64,   // CPU flags
    pub rsp: u64,      // Stack pointer
    pub ss: u64,       // Stack segment
}

impl ThreadContext {
    /// Create a new context for a thread starting at the given entry point
    ///
    /// # Arguments
    /// * `entry_point` - The function pointer where the thread will begin
    /// * `stack_top` - The top of the thread's stack (high address)
    ///
    /// # Returns
    /// A context ready to be restored, which will begin executing at the entry point
    pub fn new(entry_point: u64, stack_top: u64) -> Self {
        ThreadContext {
            // General purpose registers start at zero
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rax: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,

            // Special registers
            rip: entry_point,
            cs: 0x08,  // Kernel code segment (from GDT)
            rflags: 0x202,  // Interrupts enabled (IF flag)
            rsp: stack_top,
            ss: 0x10,  // Kernel data segment (from GDT)
        }
    }

    /// Create an empty context (all zeros)
    pub const fn empty() -> Self {
        ThreadContext {
            r15: 0, r14: 0, r13: 0, r12: 0,
            rbp: 0, rbx: 0, r11: 0, r10: 0,
            r9: 0, r8: 0, rax: 0, rcx: 0,
            rdx: 0, rsi: 0, rdi: 0,
            rip: 0, cs: 0, rflags: 0,
            rsp: 0, ss: 0,
        }
    }
}

/// Switch from the current thread context to a new thread context
///
/// This is the heart of cooperative multitasking. It saves the current
/// thread's state and restores the new thread's state in one atomic operation.
///
/// # Arguments
/// * `old_context` - Where to save the current thread's state
/// * `new_context` - The context to restore and resume
///
/// # Safety
/// This function performs raw register manipulation and must only be called
/// from the scheduler with interrupts disabled.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(_old_context: *mut ThreadContext, _new_context: *const ThreadContext) {
    core::arch::naked_asm!(
        // Save current context
        // rdi = old_context pointer (first argument)
        // rsi = new_context pointer (second argument)

        // Save general purpose registers
        "mov [rdi + 0x00], r15",
        "mov [rdi + 0x08], r14",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], rbp",
        "mov [rdi + 0x28], rbx",
        "mov [rdi + 0x30], r11",
        "mov [rdi + 0x38], r10",
        "mov [rdi + 0x40], r9",
        "mov [rdi + 0x48], r8",
        "mov [rdi + 0x50], rax",
        "mov [rdi + 0x58], rcx",
        "mov [rdi + 0x60], rdx",
        "mov [rdi + 0x68], rsi",
        "mov [rdi + 0x70], rdi",

        // Save RIP (return address is on stack)
        "mov rax, [rsp]",
        "mov [rdi + 0x78], rax",

        // Save CS
        "mov ax, cs",
        "mov [rdi + 0x80], rax",

        // Save RFLAGS
        "pushfq",
        "pop rax",
        "mov [rdi + 0x88], rax",

        // Save RSP (before return address was pushed)
        "lea rax, [rsp + 8]",
        "mov [rdi + 0x90], rax",

        // Save SS
        "mov ax, ss",
        "mov [rdi + 0x98], rax",

        // Now restore new context
        // rsi = new_context pointer

        // Restore general purpose registers
        "mov r15, [rsi + 0x00]",
        "mov r14, [rsi + 0x08]",
        "mov r13, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov rbp, [rsi + 0x20]",
        "mov rbx, [rsi + 0x28]",
        "mov r11, [rsi + 0x30]",
        "mov r10, [rsi + 0x38]",
        "mov r9,  [rsi + 0x40]",
        "mov r8,  [rsi + 0x48]",
        "mov rax, [rsi + 0x50]",
        "mov rcx, [rsi + 0x58]",
        "mov rdx, [rsi + 0x60]",

        // Restore RSP first (we'll need it)
        "mov rsp, [rsi + 0x90]",

        // Push new context's RIP onto the new stack (for ret)
        "push qword ptr [rsi + 0x78]",

        // Restore RFLAGS
        "push qword ptr [rsi + 0x88]",
        "popfq",

        // Restore remaining registers
        "mov rdi, [rsi + 0x70]",
        "mov rsi, [rsi + 0x68]",

        // Return to new thread's RIP
        "ret",
    );
}

/// Initialize a new thread's stack with the proper frame for first execution
///
/// This sets up the stack so that when we "restore" the context for the first time,
/// it will cleanly jump to the thread's entry point.
///
/// # Arguments
/// * `stack_top` - The top of the stack (high address)
/// * `entry_point` - The function where the thread starts
///
/// # Returns
/// The adjusted stack pointer after initialization
#[allow(dead_code)]
pub unsafe fn init_thread_stack(stack_top: u64, _entry_point: fn() -> !) -> u64 {
    // The stack grows downward, so we work backward from the top
    // When context is restored, ret will pop RIP from stack
    // So we don't need to push anything here - the context struct handles it

    stack_top
}

/// Helper wrapper for thread entry points
///
/// This function is called when a new thread is first scheduled.
/// It handles any setup needed before calling the actual entry point.
pub extern "C" fn thread_entry_wrapper(entry_point: fn() -> !) -> ! {
    // Enable interrupts for this new thread
    unsafe {
        asm!("sti", options(nomem, nostack));
    }

    // Call the actual entry point
    entry_point()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = ThreadContext::new(0x1000, 0x5000);
        assert_eq!(ctx.rip, 0x1000);
        assert_eq!(ctx.rsp, 0x5000);
        assert_eq!(ctx.cs, 0x08);
        assert_eq!(ctx.ss, 0x10);
        assert_ne!(ctx.rflags & 0x200, 0); // IF flag set
    }

    #[test]
    fn test_empty_context() {
        let ctx = ThreadContext::empty();
        assert_eq!(ctx.rip, 0);
        assert_eq!(ctx.rsp, 0);
        assert_eq!(ctx.rax, 0);
    }
}
