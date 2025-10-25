//! # The Attunement Layer - Rebuilt with Architectural Purity
//!
//! The Grand Design: Three Quests to Enable Keyboard Input
//! 1. The Guardian (PIC) - Using pic8259 for proper remapping
//! 2. The Law (IDT) - Using x86_64 for proper interrupt handling
//! 3. The Spell (Handler) - Simple, clean interrupt processing

pub mod gdt;
pub mod idt;
pub mod keyboard;
pub mod timer;

use pic8259::ChainedPics;
use crate::mana_pool::InterruptSafeLock;

/// The standard offset for remapping the PICs
/// IRQs 0-15 become interrupts 32-47
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// The Guardian - Our Programmable Interrupt Controller
/// This is THE source of truth for PIC management
/// CRITICAL: Must be interrupt-safe since accessed from interrupt handlers for EOI
pub static PICS: InterruptSafeLock<ChainedPics> =
    InterruptSafeLock::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// Initialize the Attunement Layer
/// This follows The Grand Unification sequence
pub fn init() {
    crate::println!("◈ Beginning the Grand Attunement...");

    unsafe {
        // Quest 1: Tame the Guardian (Remap PICs to 32-47)
        crate::println!("  ⟡ Quest 1: Taming the Guardian (PIC remapping)...");
        PICS.lock().initialize();

        // Quest 2: Scribe the Laws (Setup IDT)
        crate::println!("  ⟡ Quest 2: Scribing the Laws of Reaction (IDT)...");
        idt::init();

        // Quest 3: Initialize keyboard state (no PS/2 commands, trust BIOS)
        crate::println!("  ⟡ Quest 3: Preparing keyboard state...");
        keyboard::init();

        // Final Step: Open the gates (enable interrupts)
        crate::println!("  ⟡ Opening the gates to the outside world...");
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::println!("  ✓ The Chain of Listening has been forged!");
}
