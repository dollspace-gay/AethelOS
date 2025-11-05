//! # The Laws of Reaction - Interrupt Descriptor Table
//!
//! Using x86_64 crate for proper, safe interrupt handling
//!
//! The IDT is placed in the .rune section and becomes read-only after boot,
//! protecting it from data-only attacks that might try to hijack interrupt handlers.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use core::mem::MaybeUninit;

/// The Interrupt Descriptor Table - placed in .rune section for permanence
///
/// After boot, this becomes read-only at the hardware level, preventing any
/// modification to interrupt handler addresses.
#[link_section = ".rune"]
static mut IDT: MaybeUninit<InterruptDescriptorTable> = MaybeUninit::uninit();

/// Track whether IDT has been initialized
static mut IDT_INITIALIZED: bool = false;

/// Initialize and load the IDT into the CPU
///
/// This MUST be called before seal_rune_section() is called, as it needs
/// to write to the IDT structure.
pub fn init() {
    unsafe {
        // Create and configure the IDT
        let mut idt = InterruptDescriptorTable::new();

        // The Keyboard Spell - IRQ 1 = Interrupt 33
        idt[33].set_handler_fn(keyboard_interrupt_handler);

        // The Timer Spell - IRQ 0 = Interrupt 32
        idt[32].set_handler_fn(timer_interrupt_handler);

        // The Page Fault Handler - Exception 14
        idt.page_fault.set_handler_fn(page_fault_handler);

        // The Ring 1 System Call Gate - INT 0x81
        // Ring 1 services (Groves) use this to make kernel calls.
        // We use INT 0x81 instead of the syscall instruction because:
        //   1. Ring 1 services share kernel address space (simpler)
        //   2. Software interrupts can escalate privilege (Ring 1 → Ring 0)
        //   3. CPU automatically handles stack switch via TSS.rsp[0]
        //
        // CRITICAL: Set DPL=3 to allow both Ring 1 and Ring 3 to invoke this interrupt.
        // By default, IDT entries have DPL=0 (kernel only). Without setting DPL, Ring 1
        // code will get a GPF when executing INT 0x81.
        idt[0x81]
            .set_handler_fn(ring1_syscall_handler)
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3);  // DPL=3 allows Ring1 and Ring3

        // TODO: The Ring 3 System Call Gate - INT 0x80 or SYSCALL instruction
        // Disabled until we implement proper userspace. For now, Ring 3 will use
        // the syscall/sysret instructions (MSR-based, not IDT).
        // See: https://github.com/rust-osdev/x86_64/issues/392

        // Write to the static
        IDT.write(idt);
        IDT_INITIALIZED = true;

        // Load into CPU
        IDT.assume_init_ref().load();
    }
}

/// Get a reference to the IDT (for debugging/introspection)
///
/// # Safety
/// Must only be called after init()
pub unsafe fn get_idt() -> &'static InterruptDescriptorTable {
    if !IDT_INITIALIZED {
        panic!("IDT not initialized!");
    }
    IDT.assume_init_ref()
}

/// The Keyboard Interrupt Handler - The Spell of Perception
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Call the keyboard driver's interrupt handler
    crate::attunement::keyboard::on_interrupt();

    // CRITICAL: Send End of Interrupt to the PIC
    // Without this, no more keyboard interrupts will fire!
    unsafe {
        super::PICS.lock().notify_end_of_interrupt(33);
    }
}

/// The Ring 1 System Call Handler - Naked Wrapper
///
/// This handles system calls from Ring 1 services (Groves) via INT 0x81.
/// Ring 1 services share the kernel's address space, making the handler simpler
/// than Ring 3 syscalls (no stack switch, no GS swap needed).
///
/// Register convention:
/// - RAX: syscall number
/// - RDI, RSI, RDX, R10, R8, R9: arguments 1-6
/// - Return value in RAX
///
/// # Why Naked?
/// The `extern "x86-interrupt"` calling convention clobbers RAX in its prologue,
/// which destroys the syscall number. A naked function lets us save RAX BEFORE
/// any compiler-generated code runs.
#[unsafe(naked)]
extern "x86-interrupt" fn ring1_syscall_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        core::arch::naked_asm!(
            // At entry: CPU pushed interrupt frame (RIP, CS, RFLAGS, RSP, SS)
            // CPU already switched to Ring 0 and loaded TSS.rsp[0]
            // All general-purpose registers contain Ring 1 service values

            // Save ALL registers in order (must match SavedRegisters struct)
            "push rbp",
            "push rax",      // syscall number
            "push rbx",
            "push rcx",
            "push rdx",      // arg3
            "push rsi",      // arg2
            "push rdi",      // arg1
            "push r8",       // arg5
            "push r9",       // arg6
            "push r10",      // arg4
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            // RSP now points to SavedRegisters struct
            // Stack alignment: After CPU interrupt frame (40 bytes) + our 15 pushes (120 bytes)
            // Total = 160 bytes = 16n+0. x86-64 ABI requires RSP = 16n+8 before CALL.
            "sub rsp, 8",                // Align stack to 16n+8

            // Call the Rust handler: ring1_syscall_handler_rust(regs: *mut SavedRegisters)
            "lea rdi, [rsp + 8]",        // First arg: pointer to SavedRegisters (skip padding)
            "call {handler}",            // Call Rust function

            // RAX now contains the syscall result
            // Update the saved RAX on the stack
            "mov [rsp + 8 + 13*8], rax", // Overwrite saved RAX with result

            // Remove alignment padding
            "add rsp, 8",

            // Restore all registers (result is now in RAX)
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",      // This now contains the result
            "pop rbp",

            // Return to Ring 1 service (CPU pops interrupt frame)
            "iretq",

            handler = sym ring1_syscall_handler_rust,
        )
    }
}

/// The actual Ring 1 syscall handler (called from naked wrapper)
///
/// # Safety
/// Must only be called from ring1_syscall_handler with a valid pointer
/// to a SavedRegisters struct on the stack.
unsafe extern "C" fn ring1_syscall_handler_rust(regs: *mut SavedRegisters) -> i64 {
    let regs = &*regs;

    // Extract syscall number and arguments from saved registers
    let syscall_num = regs.rax;
    let arg1 = regs.rdi;
    let arg2 = regs.rsi;
    let arg3 = regs.rdx;
    let arg4 = regs.r10;
    let arg5 = regs.r8;
    let arg6 = regs.r9;

    crate::serial_println!(
        "[RING1_SYSCALL] INT 0x81: syscall={}, args=[{:#x}, {:#x}, {:#x}]",
        syscall_num,
        arg1,
        arg2,
        arg3
    );

    // Dispatch the syscall using the existing infrastructure
    crate::loom_of_fate::syscalls::dispatch_syscall(
        syscall_num,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
        arg6,
    )
}

/// Saved register state from syscall
///
/// This struct mirrors the order registers are pushed onto the stack
/// in the naked syscall handler.
#[repr(C)]
struct SavedRegisters {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,  // syscall number
    rbp: u64,
}

/// The System Call Handler - Naked Wrapper
///
/// This naked function preserves ALL registers (including RAX which contains
/// the syscall number) before calling the Rust handler.
///
/// # Why Naked?
/// The `extern "x86-interrupt"` calling convention clobbers RAX in its prologue,
/// which destroys the syscall number. A naked function lets us save RAX BEFORE
/// any compiler-generated code runs.
///
/// Register convention (Linux/AethelOS ABI):
/// - RAX: syscall number
/// - RDI, RSI, RDX, R10, R8, R9: arguments 1-6
/// - Return value in RAX
#[unsafe(naked)]
extern "x86-interrupt" fn syscall_handler_naked(_stack_frame: InterruptStackFrame) {
    unsafe {
        core::arch::naked_asm!(
            // At entry: CPU pushed interrupt frame (RIP, CS, RFLAGS, RSP, SS)
            // All general-purpose registers contain userspace values

            // Save ALL registers in order (must match SavedRegisters struct)
            "push rbp",
            "push rax",      // syscall number
            "push rbx",
            "push rcx",
            "push rdx",      // arg3
            "push rsi",      // arg2
            "push rdi",      // arg1
            "push r8",       // arg5
            "push r9",       // arg6
            "push r10",      // arg4
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            // RSP now points to SavedRegisters struct
            // Stack alignment check: After CPU pushes interrupt frame (40 bytes) + our 15 pushes (120 bytes)
            // Total = 160 bytes = 16n+0. But x86-64 ABI requires RSP = 16n+8 before CALL.
            "sub rsp, 8",                // Align stack to 16n+8 for ABI compliance

            // Call the Rust handler: syscall_handler_rust(regs: *mut SavedRegisters)
            "lea rdi, [rsp + 8]",        // First arg: pointer to SavedRegisters (skip alignment padding)
            "call {handler}",            // Call Rust function

            // RAX now contains the syscall result
            // Update the saved RAX on the stack (accounting for alignment padding)
            "mov [rsp + 8 + 13*8], rax", // Overwrite saved RAX with result (rsp+8 = r15, +13*8 = rax offset)

            // Remove alignment padding
            "add rsp, 8",

            // Restore all registers (result is now in RAX)
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",      // This now contains the result
            "pop rbp",

            // Return to userspace (CPU pops interrupt frame)
            "iretq",

            handler = sym syscall_handler_rust,
        )
    }
}

/// The actual syscall handler (called from naked wrapper)
///
/// # Safety
/// Must only be called from syscall_handler_naked with a valid pointer
/// to a SavedRegisters struct on the stack.
unsafe extern "C" fn syscall_handler_rust(regs: *mut SavedRegisters) -> i64 {
    let regs = &*regs;

    // Extract syscall number and arguments from saved registers
    let syscall_num = regs.rax;
    let arg1 = regs.rdi;
    let arg2 = regs.rsi;
    let arg3 = regs.rdx;
    let arg4 = regs.r10;  // Note: r10, not rcx (rcx is clobbered by syscall instruction)
    let arg5 = regs.r8;
    let arg6 = regs.r9;

    // Dispatch the syscall
    crate::loom_of_fate::syscalls::dispatch_syscall(
        syscall_num,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
        arg6,
    )
}

/// The Timer Interrupt Handler - The Rhythm of Time
///
/// This handler is called on every timer tick (typically 1ms).
/// It increments the tick counter and, if preemptive multitasking is enabled,
/// tracks quantum usage and triggers context switches.
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment the timer tick counter
    crate::attunement::timer::tick();

    // === PREEMPTIVE MULTITASKING (Phase 3) - TEMPORARILY DISABLED ===
    // TEMPORARY: Disable LOOM locking in timer to debug allocator deadlock
    // TODO: Re-enable after fixing the deadlock issue

    unsafe {
        let should_preempt = false;  // TEMPORARY: Disable preemption

        /* TEMPORARILY DISABLED
        let should_preempt = {
            let mut loom = crate::loom_of_fate::get_loom().lock();

            // Decrement the current thread's quantum
            loom.tick_quantum();

            // Check if we should preempt
            loom.should_preempt()
            // Lock is dropped here
        };
        */

        // If quantum expired and preemption is enabled, switch threads
        if should_preempt {
            // TODO: Re-implement preemptive context switching
            // Need to extract interrupt frame from stack and pass to preemptive_yield
            // For now, preemption is disabled (should_preempt = false above)
        }
    }

    // Send End of Interrupt (only if we didn't preempt)
    unsafe {
        super::PICS.lock().notify_end_of_interrupt(32);
    }
}

/// Page Fault Handler - MINIMAL VERSION
///
/// This is deliberately minimal to avoid causing cascading faults.
/// We just output a marker and halt, so we can see if the handler runs at all.
extern "x86-interrupt" fn page_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    // Output [PF:addr] via direct port I/O ONLY (no stack, no heap, no formatting!)
    unsafe {
        // Read CR2 (faulting address)
        let cr2: u64;
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));

        // Helper to output a hex digit via port I/O
        #[inline(always)]
        fn out_hex_digit(digit: u8) {
            let ch = if digit < 10 {
                b'0' + digit
            } else {
                b'a' + (digit - 10)
            };
            unsafe {
                core::arch::asm!(
                    "out dx, al",
                    in("dx") 0x3f8u16,
                    in("al") ch,
                    options(nomem, nostack, preserves_flags)
                );
            }
        }

        core::arch::asm!(
            "mov dx, 0x3f8",
            "mov al, '['",
            "out dx, al",
            "mov al, 'P'",
            "out dx, al",
            "mov al, 'F'",
            "out dx, al",
            "mov al, ':'",
            "out dx, al",
            "mov al, '0'",
            "out dx, al",
            "mov al, 'x'",
            "out dx, al",
            options(nostack, preserves_flags)
        );

        // Output CR2 in hex (64-bit value, 16 hex digits)
        for i in (0..16).rev() {
            let digit = ((cr2 >> (i * 4)) & 0xF) as u8;
            out_hex_digit(digit);
        }

        core::arch::asm!(
            "mov dx, 0x3f8",
            "mov al, ']'",
            "out dx, al",
            options(nostack, preserves_flags)
        );
    }

    // Halt the CPU to prevent cascading faults
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack, nomem));
        }
    }
}
