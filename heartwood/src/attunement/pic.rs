//! # Programmable Interrupt Controller (PIC)
//!
//! The 8259 PIC manages hardware interrupts.
//! We remap IRQs 0-15 to interrupts 32-47 to avoid conflicts with CPU exceptions.

use core::arch::asm;

/// I/O ports for the PICs
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// PIC initialization command words
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

/// End of Interrupt command
const EOI: u8 = 0x20;

/// The dual 8259 PIC configuration
pub struct Pic {
    offset1: u8,  // Offset for PIC1 (usually 32)
    offset2: u8,  // Offset for PIC2 (usually 40)
}

impl Pic {
    /// Create a new PIC configuration
    pub const fn new(offset1: u8, offset2: u8) -> Self {
        Self { offset1, offset2 }
    }

    /// Initialize and remap the PICs
    ///
    /// # Safety
    ///
    /// This must be called exactly once during system initialization.
    /// Incorrect PIC configuration can cause system instability.
    pub unsafe fn initialize(&self) {
        // Save masks
        let mask1 = inb(PIC1_DATA);
        let mask2 = inb(PIC2_DATA);

        // Start initialization sequence
        outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();

        // Set vector offsets
        outb(PIC1_DATA, self.offset1);
        io_wait();
        outb(PIC2_DATA, self.offset2);
        io_wait();

        // Tell PIC1 about PIC2 (cascade)
        outb(PIC1_DATA, 4);  // PIC2 is at IRQ2
        io_wait();
        outb(PIC2_DATA, 2);  // Cascade identity
        io_wait();

        // Set 8086 mode
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();

        // Restore masks
        outb(PIC1_DATA, mask1);
        outb(PIC2_DATA, mask2);
    }

    /// Disable all interrupts (mask all IRQs)
    pub unsafe fn disable_all(&self) {
        outb(PIC1_DATA, 0xFF);
        outb(PIC2_DATA, 0xFF);
    }

    /// Enable specific IRQ
    pub unsafe fn enable_irq(&self, irq: u8) {
        let port = if irq < 8 {
            PIC1_DATA
        } else {
            PIC2_DATA
        };

        let value = inb(port);
        let mask = !(1 << (irq % 8));
        outb(port, value & mask);
    }

    /// Disable specific IRQ
    pub unsafe fn disable_irq(&self, irq: u8) {
        let port = if irq < 8 {
            PIC1_DATA
        } else {
            PIC2_DATA
        };

        let value = inb(port);
        let mask = 1 << (irq % 8);
        outb(port, value | mask);
    }

    /// Send End of Interrupt signal
    pub unsafe fn send_eoi(&self, irq: u8) {
        if irq >= 8 {
            outb(PIC2_COMMAND, EOI);
        }
        outb(PIC1_COMMAND, EOI);
    }
}

/// Read a byte from an I/O port
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nostack, preserves_flags)
    );
    value
}

/// Write a byte to an I/O port
#[inline]
unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nostack, preserves_flags)
    );
}

/// Wait for an I/O operation to complete
/// Uses port 0x80 (POST diagnostic port) for a brief delay
#[inline]
unsafe fn io_wait() {
    outb(0x80, 0);
}
