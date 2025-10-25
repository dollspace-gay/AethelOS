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
            // CRITICAL: RSP must be misaligned by 8 for x86-64 ABI
            // (as if a call instruction just pushed a return address)
            rsp: stack_top - 8,
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

        // Push interrupt frame for IRETQ (in reverse order: SS, RSP, RFLAGS, CS, RIP)
        "push qword ptr [rsi + 0x98]",  // SS
        "push qword ptr [rsi + 0x90]",  // RSP

        // Push RFLAGS with IF (interrupt enable) bit set
        "mov rax, [rsi + 0x88]",
        "or rax, 0x200",                // Set IF flag (bit 9) - enable interrupts
        "push rax",                      // RFLAGS

        "push qword ptr [rsi + 0x80]",  // CS
        "push qword ptr [rsi + 0x78]",  // RIP

        // Restore remaining registers
        "mov rdi, [rsi + 0x70]",
        "mov rsi, [rsi + 0x68]",

        // Use IRETQ to properly restore the interrupt frame and re-enable interrupts
        "iretq",
    );
}

/// Switch context without using IRETQ (for cooperative multitasking from normal code)
///
/// This is similar to switch_context but doesn't use IRETQ, making it suitable
/// for context switches initiated from normal code (not interrupt handlers).
///
/// # Safety
/// Must be called with valid context pointers
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context_cooperative(_old_context: *mut ThreadContext, _new_context: *const ThreadContext) {
    core::arch::naked_asm!(
        // Save current context (rdi = old_context, rsi = new_context)

        // Save callee-saved registers
        "mov [rdi + 0x00], r15",
        "mov [rdi + 0x08], r14",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], rbp",
        "mov [rdi + 0x28], rbx",

        // Save RIP (return address is on stack)
        "mov rax, [rsp]",
        "mov [rdi + 0x78], rax",

        // Save RSP (before return address)
        "lea rax, [rsp + 8]",
        "mov [rdi + 0x90], rax",

        // Save RFLAGS
        "pushfq",
        "pop rax",
        "mov [rdi + 0x88], rax",

        // Now restore new context from rsi

        // Restore callee-saved registers
        "mov r15, [rsi + 0x00]",
        "mov r14, [rsi + 0x08]",
        "mov r13, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov rbp, [rsi + 0x20]",
        "mov rbx, [rsi + 0x28]",

        // Restore RSP
        "mov rsp, [rsi + 0x90]",

        // Push return address (RIP) onto the new stack
        "push qword ptr [rsi + 0x78]",

        // Restore RFLAGS (with IF enabled)
        "mov rax, [rsi + 0x88]",
        "or rax, 0x200",  // Ensure interrupts are enabled
        "push rax",
        "popfq",

        // Return to the new thread (pops RIP from stack)
        "ret",
    );
}

/// One-way context switch for the first thread (bootstrap to first real thread)
///
/// This is a sacred, one-time ritual. It performs a one-way jump from the
/// bootstrap code (which is not a real thread) to the first actual thread.
/// Unlike normal context switches, this does NOT save the current state,
/// because there is no current thread to save.
///
/// # Safety
/// This function never returns - it jumps directly to the first thread's entry point
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch_first(_new_context: *const ThreadContext) -> ! {
    core::arch::naked_asm!(
        // rdi = new_context pointer (argument in RDI per x86-64 calling convention)

        // The Scroll of Truth has revealed the sacred offsets:
        // rip offset: 0x78, cs: 0x80, rflags: 0x88, rsp: 0x90, ss: 0x98

        // Simplified approach: Just set up RSP and jump
        // Since we're in ring 0 and staying in ring 0, we don't need IRETQ

        // Load the entry point into RAX before modifying other registers
        "mov rax, [rdi + 0x78]",  // Load RIP (entry point)

        // Load the stack pointer
        "mov rsp, [rdi + 0x90]",  // Load RSP

        // Clear base pointer (indicates top of call stack)
        "xor rbp, rbp",

        // Enable interrupts
        "sti",

        // Jump to the entry point
        "jmp rax",
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

/// Save the current thread's context from an interrupt frame (for preemptive multitasking)
///
/// When a timer interrupt fires, the CPU has already pushed an interrupt frame onto the stack.
/// This function captures that state and stores it in the thread's context structure.
///
/// # Arguments
/// * `context` - Where to save the interrupted thread's state
/// * `stack_frame` - The interrupt stack frame pushed by the CPU
///
/// # Safety
/// Must be called from interrupt context with a valid interrupt stack frame
#[unsafe(naked)]
pub unsafe extern "C" fn save_preempted_context(_context: *mut ThreadContext, _stack_frame: *const u64) {
    core::arch::naked_asm!(
        // rdi = context pointer (first argument)
        // rsi = stack_frame pointer (second argument)

        // The interrupt frame on stack contains (from low to high address):
        // [rsi+0]:  RIP (instruction pointer when interrupted)
        // [rsi+8]:  CS  (code segment)
        // [rsi+16]: RFLAGS
        // [rsi+24]: RSP (stack pointer when interrupted)
        // [rsi+32]: SS  (stack segment)

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
        // Save rsi before we overwrite it
        "mov [rdi + 0x68], rsi",
        "mov [rdi + 0x70], rdi",

        // Now save the interrupt frame fields from [rsi]
        "mov rax, [rsi + 0]",    // RIP from interrupt frame
        "mov [rdi + 0x78], rax",

        "mov rax, [rsi + 8]",    // CS from interrupt frame
        "mov [rdi + 0x80], rax",

        "mov rax, [rsi + 16]",   // RFLAGS from interrupt frame
        "mov [rdi + 0x88], rax",

        "mov rax, [rsi + 24]",   // RSP from interrupt frame
        "mov [rdi + 0x90], rax",

        "mov rax, [rsi + 32]",   // SS from interrupt frame
        "mov [rdi + 0x98], rax",

        "ret",
    );
}

/// Switch from a preempted thread to another thread (called from interrupt context)
///
/// This is similar to switch_context but assumes the old context was already saved
/// via save_preempted_context. It only restores the new context and uses IRETQ.
///
/// # Arguments
/// * `new_context` - The context to restore and resume
///
/// # Safety
/// Must be called from interrupt context after save_preempted_context
#[unsafe(naked)]
pub unsafe extern "C" fn restore_context(_new_context: *const ThreadContext) -> ! {
    core::arch::naked_asm!(
        // rdi = new_context pointer

        // Restore general purpose registers
        "mov r15, [rdi + 0x00]",
        "mov r14, [rdi + 0x08]",
        "mov r13, [rdi + 0x10]",
        "mov r12, [rdi + 0x18]",
        "mov rbp, [rdi + 0x20]",
        "mov rbx, [rdi + 0x28]",
        "mov r11, [rdi + 0x30]",
        "mov r10, [rdi + 0x38]",
        "mov r9,  [rdi + 0x40]",
        "mov r8,  [rdi + 0x48]",
        "mov rax, [rdi + 0x50]",
        "mov rcx, [rdi + 0x58]",
        "mov rdx, [rdi + 0x60]",

        // Switch to the new thread's stack
        "mov rsp, [rdi + 0x90]",

        // Build interrupt frame on the new stack for IRETQ
        // Push in reverse order: SS, RSP, RFLAGS, CS, RIP
        "push qword ptr [rdi + 0x98]",  // SS
        "push qword ptr [rdi + 0x90]",  // RSP

        // Push RFLAGS with IF (interrupt flag) set to enable interrupts
        "mov rax, [rdi + 0x88]",
        "or rax, 0x200",                // Set IF flag (bit 9)
        "push rax",                      // RFLAGS

        "push qword ptr [rdi + 0x80]",  // CS
        "push qword ptr [rdi + 0x78]",  // RIP

        // Restore the last two registers
        "mov rsi, [rdi + 0x68]",
        "mov rdi, [rdi + 0x70]",

        // Use IRETQ to return to the new thread
        // This pops RIP, CS, RFLAGS, RSP, SS and properly returns from interrupt
        "iretq",
    );
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
