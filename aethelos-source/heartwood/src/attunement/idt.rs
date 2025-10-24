//! # The Interrupt Conduit
//!
//! Where hardware whispers to the kernel.
//! Interrupts are invitations, not intrusions.
//! Each interrupt is a message from hardware seeking harmony.

use core::arch::asm;
use core::mem::size_of;

#[path = "idt_handlers.rs"]
pub(super) mod idt_handlers;

/// The Interrupt Descriptor Table - 256 entries
pub struct InterruptDescriptorTable {
    entries: [IdtEntry; 256],
}

/// A single entry in the IDT
#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IdtEntry {
    offset_low: u16,      // Lower 16 bits of handler address
    selector: u16,        // Code segment selector
    ist: u8,              // Interrupt Stack Table offset (0 = don't use)
    flags: u8,            // Type and attributes
    offset_mid: u16,      // Middle 16 bits of handler address
    offset_high: u32,     // Upper 32 bits of handler address
    reserved: u32,        // Must be zero
}

/// IDT Pointer structure for loading
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

impl IdtEntry {
    /// Create a null entry
    const fn null() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create an interrupt gate entry
    fn new(handler: usize, selector: u16) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: 0,
            flags: 0x8E, // Present, DPL=0, Type=Interrupt Gate (0x8E)
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Create a trap gate entry (for exceptions)
    fn new_trap(handler: usize, selector: u16) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: 0,
            flags: 0x8F, // Present, DPL=0, Type=Trap Gate (0x8F)
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

impl InterruptDescriptorTable {
    /// Create a new IDT with all null entries
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::null(); 256],
        }
    }

    /// Attune to hardware interrupts and exceptions
    pub fn initialize(&mut self) {
        use idt_handlers::*;

        // CPU Exceptions (0-31) - These are disharmony alerts
        self.set_trap_handler(0, divide_by_zero_handler);
        self.set_trap_handler(1, debug_handler);
        self.set_trap_handler(2, nmi_handler);
        self.set_trap_handler(3, breakpoint_handler);
        self.set_trap_handler(4, overflow_handler);
        self.set_trap_handler(5, bound_range_handler);
        self.set_trap_handler(6, invalid_opcode_handler);
        self.set_trap_handler(7, device_not_available_handler);
        self.set_trap_handler(8, double_fault_handler);
        // 9 is reserved (legacy)
        self.set_trap_handler(10, invalid_tss_handler);
        self.set_trap_handler(11, segment_not_present_handler);
        self.set_trap_handler(12, stack_segment_fault_handler);
        self.set_trap_handler(13, general_protection_fault_handler);
        self.set_trap_handler(14, page_fault_handler);
        // 15 is reserved
        self.set_trap_handler(16, x87_floating_point_handler);
        self.set_trap_handler(17, alignment_check_handler);
        self.set_trap_handler(18, machine_check_handler);
        self.set_trap_handler(19, simd_floating_point_handler);
        self.set_trap_handler(20, virtualization_handler);
        // 21-29 are reserved
        self.set_trap_handler(30, security_exception_handler);
        // 31 is reserved

        // Hardware Interrupts (32-47) - IRQs remapped from PIC
        // These are invitations from hardware
        self.set_interrupt_handler(32, timer_handler);        // IRQ 0 - The Pulse of Time
        self.set_interrupt_handler(33, keyboard_handler);     // IRQ 1 - The Voice of Intent
        self.set_interrupt_handler(34, cascade_handler);      // IRQ 2 - PIC cascade
        self.set_interrupt_handler(35, com2_handler);         // IRQ 3 - Serial port 2
        self.set_interrupt_handler(36, com1_handler);         // IRQ 4 - Serial port 1
        self.set_interrupt_handler(37, lpt2_handler);         // IRQ 5 - Parallel port 2
        self.set_interrupt_handler(38, floppy_handler);       // IRQ 6 - Floppy disk
        self.set_interrupt_handler(39, lpt1_handler);         // IRQ 7 - Parallel port 1
        self.set_interrupt_handler(40, rtc_handler);          // IRQ 8 - Real-time clock
        self.set_interrupt_handler(41, acpi_handler);         // IRQ 9 - ACPI
        self.set_interrupt_handler(42, available1_handler);   // IRQ 10 - Available
        self.set_interrupt_handler(43, available2_handler);   // IRQ 11 - Available
        self.set_interrupt_handler(44, mouse_handler);        // IRQ 12 - PS/2 Mouse
        self.set_interrupt_handler(45, fpu_handler);          // IRQ 13 - FPU
        self.set_interrupt_handler(46, ata1_handler);         // IRQ 14 - Primary ATA
        self.set_interrupt_handler(47, ata2_handler);         // IRQ 15 - Secondary ATA

        // Software interrupts (48-255)
        // Used for system calls and inter-process communication
        self.set_interrupt_handler(128, syscall_handler);     // Int 0x80 - System call
    }

    /// Set a trap gate handler (for exceptions)
    fn set_trap_handler(&mut self, index: u8, handler: extern "C" fn()) {
        let handler_addr = handler as usize;
        self.entries[index as usize] = IdtEntry::new_trap(handler_addr, 0x08); // 0x08 = kernel code segment
    }

    /// Set an interrupt gate handler (for hardware interrupts)
    fn set_interrupt_handler(&mut self, index: u8, handler: extern "C" fn()) {
        let handler_addr = handler as usize;
        self.entries[index as usize] = IdtEntry::new(handler_addr, 0x08); // 0x08 = kernel code segment
    }

    /// Load this IDT into the CPU
    pub fn load(&self) {
        let ptr = IdtPointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        };

        unsafe {
            asm!(
                "lidt [{}]",
                in(reg) &ptr,
                options(readonly, nostack, preserves_flags)
            );
        }
    }
}

impl Default for InterruptDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}
