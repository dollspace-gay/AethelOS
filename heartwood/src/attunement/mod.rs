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
pub mod ward_of_sacred_boundaries;
pub mod ward_of_unseen_paths;
pub mod ward_of_anonymity;
pub mod per_cpu;

// Export TSS kernel stack update function for context switching
pub use gdt::set_kernel_stack;

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
    InterruptSafeLock::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) }, "PICS");

/// Initialize the Attunement Layer
/// This follows The Grand Unification sequence
pub fn init() {
    crate::println!("◈ Beginning the Grand Attunement...");
    crate::serial_println!("[ATTUNEMENT] Starting attunement sequence...");

    unsafe {
        // Quest -3: Inscribe the Concordance of Fates (RBAC)
        crate::println!("  ⟡ Quest -3: Inscribing the Concordance of Fates (RBAC)...");
        crate::serial_println!("[ATTUNEMENT] About to call init_concordance()...");
        crate::mana_pool::concordance_of_fates::init_concordance();
        crate::serial_println!("[ATTUNEMENT] init_concordance() returned successfully");
        crate::println!("     ✓ The fates of all threads are bound by sacred law");

        // Quest -2: Seal the True Names (Ward of Anonymity)
        crate::println!("  ⟡ Quest -2: Sealing the True Names (Ward of Anonymity)...");
        ward_of_anonymity::init_ward();
        crate::println!("     ✓ The spirits' names are hidden from mortal tongues");

        // Quest -1: Initialize the Ward of Unseen Paths (KASLR) - Must be first!
        crate::println!("  ⟡ Quest -1: Weaving the Ward of Unseen Paths (KASLR)...");
        ward_of_unseen_paths::init_kaslr();
        crate::println!("     ✓ The Heartwood's location is concealed");

        // Quest 0: Raise the Ward of Sacred Boundaries (SMEP/SMAP)
        crate::println!("  ⟡ Quest 0: Raising the Ward of Sacred Boundaries (SMEP/SMAP)...");
        match ward_of_sacred_boundaries::init_ward() {
            Ok(_) => crate::println!("     ✓ The Ward stands vigilant"),
            Err(e) => crate::println!("     ⚠ Ward partially raised: {}", e),
        }

        // Quest 1: Establish the Boundaries (Setup GDT & TSS)
        crate::println!("  ⟡ Quest 1: Establishing privilege boundaries (GDT & TSS)...");
        gdt::init();

        // Quest 1.5: Initialize per-CPU data structures (GS register)
        crate::println!("  ⟡ Quest 1.5: Weaving per-CPU consciousness (GS register)...");
        per_cpu::init_bsp();
        crate::println!("     ✓ The CPU's consciousness is self-aware");

        // Quest 1.6: Initialize syscall/sysret mechanism
        crate::println!("  ⟡ Quest 1.6: Opening the Gates of Invocation (syscall/sysret)...");
        crate::loom_of_fate::syscalls::init_syscall();
        crate::println!("     ✓ User space can now petition the kernel");

        // Quest 2: Tame the Guardian (Remap PICs to 32-47)
        crate::println!("  ⟡ Quest 2: Taming the Guardian (PIC remapping)...");
        // DEBUG: Direct serial output to bypass any println issues
        unsafe {
            for &byte in b"[DEBUG] Before PICS.lock()\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }
        PICS.lock().initialize();
        unsafe {
            for &byte in b"[DEBUG] After PICS.initialize()\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }

        // Quest 3: Scribe the Laws (Setup IDT)
        crate::println!("  ⟡ Quest 3: Scribing the Laws of Reaction (IDT)...");
        unsafe {
            for &byte in b"[DEBUG] Before idt::init()\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }
        idt::init();
        unsafe {
            for &byte in b"[DEBUG] After idt::init()\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }

        // Quest 4: Initialize keyboard state (no PS/2 commands, trust BIOS)
        crate::println!("  ⟡ Quest 4: Preparing keyboard state...");
        unsafe {
            for &byte in b"[DEBUG] Before keyboard::init()\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }
        keyboard::init();
        unsafe {
            for &byte in b"[DEBUG] After keyboard::init()\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }

        // Final Step: Open the gates (enable interrupts)
        crate::println!("  ⟡ Opening the gates to the outside world...");
        unsafe {
            for &byte in b"[DEBUG] Before sti\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
        unsafe {
            for &byte in b"[DEBUG] After sti - interrupts enabled\n".iter() {
                core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
            }
        }
    }

    unsafe {
        for &byte in b"[DEBUG] About to print Chain of Listening\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    crate::println!("  ✓ The Chain of Listening has been forged!");
    unsafe {
        for &byte in b"[DEBUG] After Chain of Listening println\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
}
