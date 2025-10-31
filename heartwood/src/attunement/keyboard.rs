//! # Keyboard - The Voice of Intent
//!
//! Minimal keyboard initialization - we trust that BIOS has already set up the PS/2 controller

use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;
use x86_64::instructions::port::Port;

/// Keyboard state (placeholder for now)
pub struct Keyboard {
    data_port: Port<u8>,
}

impl Keyboard {
    pub const fn new() -> Self {
        Keyboard {
            data_port: Port::new(0x60),
        }
    }

    /// Read a scancode from the keyboard data port
    pub fn read_scancode(&mut self) -> u8 {
        unsafe { self.data_port.read() }
    }
}

// Interrupt-safe keyboard state (accessed from keyboard interrupt handler)
static mut KEYBOARD: MaybeUninit<InterruptSafeLock<Keyboard>> = MaybeUninit::uninit();
static mut KEYBOARD_INITIALIZED: bool = false;

/// Initialize keyboard - just create the state structure
/// The BIOS has already initialized the PS/2 controller
/// The pic8259 crate will handle enabling IRQ 1
pub fn init() {
    unsafe {
        let keyboard = Keyboard::new();
        let lock = InterruptSafeLock::new(keyboard, "KEYBOARD");
        core::ptr::write(core::ptr::addr_of_mut!(KEYBOARD).cast(), lock);
        KEYBOARD_INITIALIZED = true;
    }
}

/// Called when a keyboard interrupt occurs
pub fn on_interrupt() {
    unsafe {
        if !KEYBOARD_INITIALIZED {
            return;
        }

        let mut keyboard = (*core::ptr::addr_of!(KEYBOARD).cast::<InterruptSafeLock<Keyboard>>()).lock();
        let scancode = keyboard.read_scancode();

        // Ignore key release events (scancode >= 0x80)
        if scancode >= 0x80 {
            return;
        }

        // Basic scancode to character mapping (US QWERTY, lowercase only)
        // Only handle key PRESS events
        let ch = match scancode {
            // Number row
            0x02 => Some('1'),
            0x03 => Some('2'),
            0x04 => Some('3'),
            0x05 => Some('4'),
            0x06 => Some('5'),
            0x07 => Some('6'),
            0x08 => Some('7'),
            0x09 => Some('8'),
            0x0A => Some('9'),
            0x0B => Some('0'),
            0x0C => Some('-'),  // Minus/Hyphen
            0x0D => Some('='),  // Equals

            // Top letter row
            0x10 => Some('q'),
            0x11 => Some('w'),
            0x12 => Some('e'),
            0x13 => Some('r'),
            0x14 => Some('t'),
            0x15 => Some('y'),
            0x16 => Some('u'),
            0x17 => Some('i'),
            0x18 => Some('o'),
            0x19 => Some('p'),
            0x1A => Some('['),
            0x1B => Some(']'),

            // Middle letter row
            0x1E => Some('a'),
            0x1F => Some('s'),
            0x20 => Some('d'),
            0x21 => Some('f'),
            0x22 => Some('g'),
            0x23 => Some('h'),
            0x24 => Some('j'),
            0x25 => Some('k'),
            0x26 => Some('l'),
            0x27 => Some(';'),
            0x28 => Some('\''),

            // Bottom letter row
            0x2C => Some('z'),
            0x2D => Some('x'),
            0x2E => Some('c'),
            0x2F => Some('v'),
            0x30 => Some('b'),
            0x31 => Some('n'),
            0x32 => Some('m'),
            0x33 => Some(','),
            0x34 => Some('.'),
            0x35 => Some('/'),

            // Special keys
            0x39 => Some(' '),    // Space
            0x1C => Some('\n'),   // Enter
            0x0E => Some('\x08'), // Backspace
            0x48 => Some('\x01'), // Up arrow (special control char)
            0x50 => Some('\x02'), // Down arrow (special control char)
            _ => None,  // Ignore other keys
        };

        // Handle the character
        if let Some(character) = ch {
            match character {
                '\x08' => {
                    // Backspace: erase character visually and update buffer
                    crate::eldarin::handle_backspace();
                }
                '\x01' => {
                    // Up arrow: navigate to previous command
                    crate::eldarin::handle_arrow_up();
                }
                '\x02' => {
                    // Down arrow: navigate to next command
                    crate::eldarin::handle_arrow_down();
                }
                _ => {
                    // Regular character: echo it to screen
                    // SAFETY: We're already in an interrupt handler (interrupts disabled),
                    // but we need to ensure VGA buffer access is safe. Since VGA Writer
                    // doesn't use proper locking, we just access it directly.
                    // The real fix would be to make WRITER use InterruptSafeLock.
                    crate::print!("{}", character);
                    crate::eldarin::handle_char(character);
                }
            }
        }
    }
}
