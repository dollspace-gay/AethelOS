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
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment the timer tick counter
    crate::attunement::timer::tick();

    // REMOVED: Calling yield_now() from interrupt handler causes preemptive
    // context switches that can interrupt critical sections (like Drop implementations).
    // For cooperative multitasking, threads should only yield explicitly.
    // TODO: If you want preemptive multitasking, you need to:
    //   1. Ensure all critical sections are interrupt-safe
    //   2. Use proper interrupt-safe context switching
    // crate::loom_of_fate::yield_now();

    // Send End of Interrupt
    unsafe {
        super::PICS.lock().notify_end_of_interrupt(32);
    }
}
