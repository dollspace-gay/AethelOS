//! # Programmable Interval Timer (PIT)
//!
//! The heartbeat of AethelOS - the system's sense of time's passage.
//! The PIT provides rhythmic interrupts that drive scheduling, timeouts,
//! and our awareness of temporal flow.
//!
//! ## Philosophy
//! Time in AethelOS is not a tyrant demanding rigid adherence,
//! but a gentle rhythm that guides cooperation.
//! The PIT ticks not to command, but to remind us that moments pass.
//!
//! ## Technical Details
//! The Intel 8253/8254 PIT has three channels:
//! - Channel 0: System timer (IRQ 0)
//! - Channel 1: DRAM refresh (legacy, unused)
//! - Channel 2: PC speaker
//!
//! We use Channel 0 in mode 3 (square wave generator) for periodic interrupts.

use core::arch::asm;

/// PIT I/O ports
const PIT_CHANNEL0: u16 = 0x40; // Channel 0 data port (system timer)
const PIT_CHANNEL1: u16 = 0x41; // Channel 1 data port (unused)
const PIT_CHANNEL2: u16 = 0x42; // Channel 2 data port (PC speaker)
const PIT_COMMAND: u16 = 0x43;  // Mode/Command register

/// PIT base frequency (Hz)
const PIT_BASE_FREQ: u32 = 1193182;

/// Default timer frequency (100 Hz = 10ms per tick)
/// This provides a good balance between precision and overhead
const DEFAULT_FREQ: u32 = 100;

/// PIT command byte components
/// Format: [Channel (2 bits)][Access mode (2 bits)][Operating mode (3 bits)][BCD (1 bit)]
const CMD_CHANNEL0: u8 = 0b00_000000;      // Select channel 0
const CMD_ACCESS_LOHI: u8 = 0b00_11_0000;  // Access mode: lobyte/hibyte
const CMD_MODE3: u8 = 0b00_00_011_0;       // Operating mode 3: square wave
const CMD_BINARY: u8 = 0b00_00_000_0;      // Binary mode (not BCD)

/// Helper function to output a byte to an I/O port
#[inline]
unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

/// Helper function to input a byte from an I/O port
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

/// The Programmable Interval Timer
pub struct Pit {
    frequency: u32,
    divisor: u16,
    ticks_per_second: u32,
}

impl Pit {
    /// Create a new PIT configuration with default frequency
    pub const fn new() -> Self {
        let divisor = (PIT_BASE_FREQ / DEFAULT_FREQ) as u16;
        Pit {
            frequency: DEFAULT_FREQ,
            divisor,
            ticks_per_second: DEFAULT_FREQ,
        }
    }

    /// Create a PIT with custom frequency
    ///
    /// # Arguments
    /// * `freq` - Desired frequency in Hz (18.2 Hz to 1.19 MHz)
    ///
    /// # Note
    /// The actual frequency may differ slightly due to integer division.
    pub fn with_frequency(freq: u32) -> Self {
        let freq = freq.clamp(18, PIT_BASE_FREQ);
        let divisor = (PIT_BASE_FREQ / freq) as u16;
        let actual_freq = PIT_BASE_FREQ / divisor as u32;

        Pit {
            frequency: actual_freq,
            divisor,
            ticks_per_second: actual_freq,
        }
    }

    /// Initialize the PIT and start generating interrupts
    ///
    /// # Safety
    /// This function writes to I/O ports and should only be called once during boot.
    pub unsafe fn initialize(&self) {
        // Disable interrupts during PIT programming
        asm!("cli", options(nomem, nostack));

        // Send command byte: Channel 0, lobyte/hibyte, mode 3, binary
        let command = CMD_CHANNEL0 | CMD_ACCESS_LOHI | CMD_MODE3 | CMD_BINARY;
        outb(PIT_COMMAND, command);

        // Send divisor (low byte, then high byte)
        outb(PIT_CHANNEL0, (self.divisor & 0xFF) as u8);
        outb(PIT_CHANNEL0, ((self.divisor >> 8) & 0xFF) as u8);

        // Re-enable interrupts
        asm!("sti", options(nomem, nostack));
    }

    /// Get the configured frequency in Hz
    pub fn frequency(&self) -> u32 {
        self.frequency
    }

    /// Get the divisor value
    pub fn divisor(&self) -> u16 {
        self.divisor
    }

    /// Get ticks per second (same as frequency)
    pub fn ticks_per_second(&self) -> u32 {
        self.ticks_per_second
    }

    /// Read the current counter value
    ///
    /// # Safety
    /// Reads from I/O ports. The value may be unreliable if called during a count update.
    pub unsafe fn read_count(&self) -> u16 {
        // Send latch command
        outb(PIT_COMMAND, 0b00_00_0000);

        // Read count (low byte, then high byte)
        let low = inb(PIT_CHANNEL0) as u16;
        let high = inb(PIT_CHANNEL0) as u16;

        (high << 8) | low
    }
}

/// Calculate milliseconds per tick for the current PIT frequency
pub fn ms_per_tick(pit: &Pit) -> u32 {
    1000 / pit.frequency()
}

/// Calculate microseconds per tick for the current PIT frequency
pub fn us_per_tick(pit: &Pit) -> u32 {
    1_000_000 / pit.frequency()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divisor_calculation() {
        let pit = Pit::new();
        // For 100 Hz: 1193182 / 100 = 11931.82, should round to 11931
        assert_eq!(pit.divisor(), 11931);
    }

    #[test]
    fn test_frequency() {
        let pit = Pit::new();
        assert!(pit.frequency() >= 99 && pit.frequency() <= 101);
    }

    #[test]
    fn test_custom_frequency() {
        let pit = Pit::with_frequency(1000);
        assert!(pit.frequency() >= 999 && pit.frequency() <= 1001);
    }

    #[test]
    fn test_ms_per_tick() {
        let pit = Pit::new();
        let ms = ms_per_tick(&pit);
        assert!(ms >= 9 && ms <= 11); // Should be ~10ms
    }
}
