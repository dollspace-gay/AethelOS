//! # The Laws of Reaction - Interrupt Descriptor Table
//!
//! Using x86_64 crate for proper, safe interrupt handling

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // The Keyboard Spell - IRQ 1 = Interrupt 33
        idt[33].set_handler_fn(keyboard_interrupt_handler);

        // The Timer Spell - IRQ 0 = Interrupt 32
        idt[32].set_handler_fn(timer_interrupt_handler);

        idt
    };
}

/// Load the IDT into the CPU
pub fn init() {
    IDT.load();
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

/// The Timer Interrupt Handler - The Rhythm of Time
///
/// This handler is called on every timer tick (typically 1ms).
/// It increments the tick counter and, if preemptive multitasking is enabled,
/// tracks quantum usage and triggers context switches.
extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    // Increment the timer tick counter
    crate::attunement::timer::tick();

    // === PREEMPTIVE MULTITASKING (Phase 3) ===
    // Now that all critical sections are interrupt-safe (Phase 1 complete),
    // we can safely perform preemptive context switches from interrupt context.

    unsafe {
        let should_preempt = {
            let mut loom = crate::loom_of_fate::get_loom().lock();

            // Decrement the current thread's quantum
            loom.tick_quantum();

            // Check if we should preempt
            loom.should_preempt()
            // Lock is dropped here
        };

        // If quantum expired and preemption is enabled, switch threads
        if should_preempt {
            // Send End of Interrupt BEFORE context switch
            // This ensures the PIC is ready for the next interrupt
            super::PICS.lock().notify_end_of_interrupt(32);

            // Create an array with the interrupt frame values in the correct order
            // The order matches the hardware interrupt frame: RIP, CS, RFLAGS, RSP, SS
            let frame_values: [u64; 5] = [
                stack_frame.instruction_pointer.as_u64(),
                stack_frame.code_segment,
                stack_frame.cpu_flags,
                stack_frame.stack_pointer.as_u64(),
                stack_frame.stack_segment,
            ];
            let frame_ptr = frame_values.as_ptr();

            // Perform preemptive context switch
            // This function never returns - it uses IRETQ to jump to the new thread
            crate::loom_of_fate::preemptive_yield(frame_ptr);
        }
    }

    // Send End of Interrupt (only if we didn't preempt)
    unsafe {
        super::PICS.lock().notify_end_of_interrupt(32);
    }
}
