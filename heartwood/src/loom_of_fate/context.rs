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
    pub cr3: u64,      // Page table base (physical address) - for per-Vessel address spaces
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
            cr3: 0,  // 0 means "use current CR3" (kernel threads share kernel page tables)
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
            rsp: 0, ss: 0, cr3: 0,
        }
    }

    /// Create a new context for a user-mode thread (Ring 3)
    ///
    /// This creates a context that will start executing in ring 3 (user mode)
    /// with the specified entry point and user stack.
    ///
    /// # Arguments
    /// * `entry_point` - User space entry point address
    /// * `user_stack_top` - Top of user stack
    /// * `page_table_phys` - Physical address of PML4 (CR3 value)
    ///
    /// # Returns
    /// A context ready for ring 3 execution via IRETQ or context switch
    pub fn new_user_mode(
        entry_point: u64,
        user_stack_top: u64,
        page_table_phys: u64,
    ) -> Self {
        ThreadContext {
            // General purpose registers start at zero
            r15: 0, r14: 0, r13: 0, r12: 0,
            rbp: 0, rbx: 0, r11: 0, r10: 0,
            r9: 0, r8: 0, rax: 0, rcx: 0,
            rdx: 0, rsi: 0, rdi: 0,

            // Special registers for RING 3
            rip: entry_point,
            cs: 0x20 | 3,  // User code segment (GDT index 4) | RPL=3
            rflags: 0x202,  // Interrupts enabled (IF flag set)
            rsp: user_stack_top,      // User stack (must be 16-byte aligned for IRETQ)
            ss: 0x18 | 3,  // User data segment (GDT index 3) | RPL=3
            cr3: page_table_phys,  // Vessel's page table
        }
    }

    /// Create a new context for Ring 1 service mode (privileged Grove)
    ///
    /// # Arguments
    /// * `entry_point` - Where the service thread should start executing
    /// * `service_stack_top` - Top of the service's stack (in service address space)
    /// * `page_table_phys` - Physical address of PML4 (CR3 value)
    ///
    /// # Returns
    /// A context ready for Ring 1 execution via IRETQ or context switch
    pub fn new_service_mode(
        entry_point: u64,
        service_stack_top: u64,
        page_table_phys: u64,
    ) -> Self {
        ThreadContext {
            // General purpose registers start at zero
            r15: 0, r14: 0, r13: 0, r12: 0,
            rbp: 0, rbx: 0, r11: 0, r10: 0,
            r9: 0, r8: 0, rax: 0, rcx: 0,
            rdx: 0, rsi: 0, rdi: 0,

            // Special registers for RING 1 (privileged service)
            rip: entry_point,
            cs: 0x28 | 1,  // Service code segment (GDT index 5) | RPL=1
            rflags: 0x1202,  // IF (bit 9) + IOPL=1 (bits 12-13) + Reserved bit 1
            rsp: service_stack_top,  // Service stack (must be 16-byte aligned)
            ss: 0x30 | 1,  // Service data segment (GDT index 6) | RPL=1
            cr3: page_table_phys,  // Service's page table (isolated address space)
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
        // DEBUG: Mark entry to switch_context
        "mov dx, 0x3f8",
        "mov al, '['",
        "out dx, al",
        "mov al, 'S'",
        "out dx, al",
        "mov al, 'W'",
        "out dx, al",
        "mov al, ']'",
        "out dx, al",

        // NOTE: No need to adjust RSP for red zone because interrupts are
        // disabled by the caller (yield_now uses without_interrupts).
        // The target spec already has "disable-redzone": true for compiler-generated code.

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

        // Check if we need to switch page tables (CR3)
        // If new_context.cr3 != 0, load it into CR3
        "mov rax, [rsi + 0xA0]",  // Load CR3 from new context
        "test rax, rax",           // Check if it's non-zero
        "jz 2f",                   // Skip if zero (kernel thread)

        // Switch to new page table
        "mov cr3, rax",

        // DEBUG: Mark CR3 switched
        "push rax",
        "push rdx",
        "mov dx, 0x3f8",
        "mov al, 'C'",
        "out dx, al",
        "mov al, '3'",
        "out dx, al",
        "pop rdx",
        "pop rax",

        "2:",  // Continue with normal context restore

        // DEBUG: Mark before register restore
        "push rax",
        "mov dx, 0x3f8",
        "mov al, 'R'",
        "out dx, al",
        "pop rax",

        // Restore general purpose registers
        "mov r15, [rsi + 0x00]",

        // DEBUG: After first restore
        "push rax",
        "mov dx, 0x3f8",
        "mov al, '1'",
        "out dx, al",
        "pop rax",

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

        // CRITICAL: Build IRETQ frame on KERNEL stack (current RSP), NOT user stack!
        // This avoids SMAP issues - we stay on kernel stack, IRETQ switches to user stack
        // DO NOT do "mov rsp, [rsi + 0x90]" here - that would switch to user stack!

        // Push interrupt frame for IRETQ (in reverse order: SS, RSP, RFLAGS, CS, RIP)
        "push qword ptr [rsi + 0x98]",  // SS (user's SS from context)
        "push qword ptr [rsi + 0x90]",  // RSP (user's RSP from context)

        // Push RFLAGS with IF (interrupt enable) bit set
        "mov rax, [rsi + 0x88]",
        "or rax, 0x200",                // Set IF flag (bit 9) - enable interrupts
        "push rax",                      // RFLAGS

        "push qword ptr [rsi + 0x80]",  // CS
        "push qword ptr [rsi + 0x78]",  // RIP

        // DEBUG: Output CS value to serial port
        "push rdx",
        "push rax",
        "mov rdx, 0x3f8",
        "mov al, '<'",
        "out dx, al",
        "mov al, 'C'",
        "out dx, al",
        "mov al, 'S'",
        "out dx, al",
        "mov al, '='",
        "out dx, al",
        "mov rax, [rsi + 0x80]",       // Load CS value
        "mov al, ah",                   // Get high byte
        "shr al, 4",
        "add al, '0'",
        "cmp al, '9'",
        "jle 2f",
        "add al, 7",                    // Convert to A-F
        "2:",
        "out dx, al",
        "mov rax, [rsi + 0x80]",
        "mov al, ah",
        "and al, 0x0F",
        "add al, '0'",
        "cmp al, '9'",
        "jle 2f",
        "add al, 7",
        "2:",
        "out dx, al",
        "mov rax, [rsi + 0x80]",
        "and al, 0xF0",
        "shr al, 4",
        "add al, '0'",
        "cmp al, '9'",
        "jle 2f",
        "add al, 7",
        "2:",
        "out dx, al",
        "mov rax, [rsi + 0x80]",
        "and al, 0x0F",
        "add al, '0'",
        "cmp al, '9'",
        "jle 2f",
        "add al, 7",
        "2:",
        "out dx, al",
        "mov al, '>'",
        "out dx, al",
        "pop rax",
        "pop rdx",

        // Check if we're switching to ring 3 (CS & 3 == 3)
        // NOTE: We do NOT swapgs for Ring 1! Ring 1 services need per-CPU data.
        "mov rax, [rsi + 0x80]",        // Load CS value
        "and al, 3",                     // Mask RPL bits
        "cmp al, 3",                     // Check if RPL == 3 (Ring 3)
        "jne 3f",                        // Skip swapgs if not Ring 3

        // Switching to ring 3: Execute swapgs to set up GS for userspace
        // After swapgs: GsBase = 0 (user), KernelGsBase = per-CPU (for syscall)
        "swapgs",

        "3:",  // Continue to iretq

        // DEBUG: Mark right before iretq
        "push rdx",
        "push rax",
        "mov rdx, 0x3f8",
        "mov al, '>'",
        "out dx, al",
        "mov al, '>'",
        "out dx, al",
        "mov al, 'Q'",
        "out dx, al",
        "pop rax",
        "pop rdx",

        // Restore remaining registers
        "mov rdi, [rsi + 0x70]",
        "mov rsi, [rsi + 0x68]",

        // Use IRETQ to properly restore the interrupt frame and re-enable interrupts
        // IRETQ pops: RIP, CS, RFLAGS, RSP, SS
        // This switches from kernel stack to user stack (if going to ring 3)
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
        // DEBUG: Mark entry to switch_context_cooperative
        "mov dx, 0x3f8",
        "mov al, '['",
        "out dx, al",
        "mov al, 'C'",
        "out dx, al",
        "mov al, 'O'",
        "out dx, al",
        "mov al, 'O'",
        "out dx, al",
        "mov al, 'P'",
        "out dx, al",
        "mov al, ']'",
        "out dx, al",

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

        // Check if we need to switch page tables (CR3)
        // If new_context.cr3 != 0, load it into CR3
        "mov rax, [rsi + 0xA0]",  // Load CR3 from new context
        "test rax, rax",           // Check if it's non-zero
        "jz 2f",                   // Skip if zero (kernel thread)

        // Switch to new page table
        "mov cr3, rax",

        "2:",  // Continue with normal context restore

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

        // Restore RFLAGS (preserve interrupt state)
        "mov rax, [rsi + 0x88]",
        // NOTE: Do NOT force interrupts on! Preserve the saved IF flag state.
        // The without_interrupts() wrapper in yield_now() needs interrupts to
        // remain disabled until the lock is released.
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

        // Check if we need to switch page tables (CR3)
        // If new_context.cr3 != 0, load it into CR3
        "mov rax, [rdi + 0xA0]",  // Load CR3 from new context
        "test rax, rax",           // Check if it's non-zero
        "jz 2f",                   // Skip if zero (kernel thread)

        // Switch to new page table
        "mov cr3, rax",

        "2:",  // Continue with normal context restore

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

        // CRITICAL: Build IRETQ frame on KERNEL stack, NOT user stack!
        // This avoids SMAP issues - stay on kernel stack, IRETQ switches to user stack
        // DO NOT do "mov rsp, [rdi + 0x90]" here!

        // Build interrupt frame on the KERNEL stack for IRETQ
        // Push in reverse order: SS, RSP, RFLAGS, CS, RIP
        "push qword ptr [rdi + 0x98]",  // SS (user's SS from context)
        "push qword ptr [rdi + 0x90]",  // RSP (user's RSP from context)

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
pub extern "C" fn thread_entry_wrapper(entry_point: extern "C" fn() -> !) -> ! {
    // Enable interrupts for this new thread
    unsafe {
        asm!("sti", options(nomem, nostack));
    }

    // Call the actual entry point
    entry_point()
}

/// Enter user mode for the first time
///
/// This function uses IRETQ to transition from ring 0 to ring 3.
/// It's used when starting a user-mode thread for the first time.
///
/// # Safety
/// This function never returns - it jumps to user mode.
/// The context must be properly initialized for user mode with:
/// - CS = user code segment (0x20 | 3)
/// - SS = user data segment (0x18 | 3)
/// - CR3 = valid user page table
/// - RIP = valid user entry point
/// - RSP = valid user stack
///
/// # Arguments
/// * `context` - Pointer to a ThreadContext initialized for user mode
#[unsafe(naked)]
pub unsafe extern "C" fn enter_user_mode(_context: *const ThreadContext) -> ! {
    core::arch::naked_asm!(
        // rdi = context pointer

        // Load CR3 (page table) first
        "mov rax, [rdi + 0xA0]",  // cr3 offset (from struct definition)
        "mov cr3, rax",

        // Set up IRETQ frame on stack:
        // Stack layout (pushed in reverse order):
        // [SS]      <- Top
        // [RSP]
        // [RFLAGS]
        // [CS]
        // [RIP]     <- Bottom (RSP points here after pushes)

        // Push SS (user data segment)
        "push qword ptr [rdi + 0x98]",  // ss offset

        // Push RSP (user stack pointer)
        "push qword ptr [rdi + 0x90]",  // rsp offset

        // Push RFLAGS
        "push qword ptr [rdi + 0x88]",  // rflags offset

        // Push CS (user code segment)
        "push qword ptr [rdi + 0x80]",  // cs offset

        // Push RIP (user entry point)
        "push qword ptr [rdi + 0x78]",  // rip offset

        // Zero out all registers for security (don't leak kernel data)
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor rdi, rdi",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",
        "xor rbp, rbp",

        // Enter user mode!
        "iretq",
    );
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
