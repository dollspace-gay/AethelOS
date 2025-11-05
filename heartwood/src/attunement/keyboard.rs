//! # Keyboard - The Voice of Intent
//!
//! Minimal keyboard initialization - we trust that BIOS has already set up the PS/2 controller

use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;
use x86_64::instructions::port::Port;

/// Keyboard state
pub struct Keyboard {
    data_port: Port<u8>,
    shift_pressed: bool,
}

impl Keyboard {
    pub const fn new() -> Self {
        Keyboard {
            data_port: Port::new(0x60),
            shift_pressed: false,
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

        // Track shift key state
        match scancode {
            0x2A | 0x36 => {
                // Left shift (0x2A) or right shift (0x36) pressed
                keyboard.shift_pressed = true;
                return;
            }
            0xAA | 0xB6 => {
                // Left shift (0xAA) or right shift (0xB6) released
                keyboard.shift_pressed = false;
                return;
            }
            _ => {}
        }

        // Ignore other key release events (scancode >= 0x80)
        if scancode >= 0x80 {
            return;
        }

        let shift = keyboard.shift_pressed;

        // Scancode to character mapping (US QWERTY)
        // Handle both shifted and unshifted variants
        let ch = match (scancode, shift) {
            // Number row (with shift: symbols)
            (0x02, false) => Some('1'),
            (0x02, true) => Some('!'),
            (0x03, false) => Some('2'),
            (0x03, true) => Some('@'),
            (0x04, false) => Some('3'),
            (0x04, true) => Some('#'),
            (0x05, false) => Some('4'),
            (0x05, true) => Some('$'),
            (0x06, false) => Some('5'),
            (0x06, true) => Some('%'),
            (0x07, false) => Some('6'),
            (0x07, true) => Some('^'),
            (0x08, false) => Some('7'),
            (0x08, true) => Some('&'),
            (0x09, false) => Some('8'),
            (0x09, true) => Some('*'),
            (0x0A, false) => Some('9'),
            (0x0A, true) => Some('('),
            (0x0B, false) => Some('0'),
            (0x0B, true) => Some(')'),
            (0x0C, false) => Some('-'),
            (0x0C, true) => Some('_'),
            (0x0D, false) => Some('='),
            (0x0D, true) => Some('+'),

            // Top letter row
            (0x10, false) => Some('q'),
            (0x10, true) => Some('Q'),
            (0x11, false) => Some('w'),
            (0x11, true) => Some('W'),
            (0x12, false) => Some('e'),
            (0x12, true) => Some('E'),
            (0x13, false) => Some('r'),
            (0x13, true) => Some('R'),
            (0x14, false) => Some('t'),
            (0x14, true) => Some('T'),
            (0x15, false) => Some('y'),
            (0x15, true) => Some('Y'),
            (0x16, false) => Some('u'),
            (0x16, true) => Some('U'),
            (0x17, false) => Some('i'),
            (0x17, true) => Some('I'),
            (0x18, false) => Some('o'),
            (0x18, true) => Some('O'),
            (0x19, false) => Some('p'),
            (0x19, true) => Some('P'),
            (0x1A, false) => Some('['),
            (0x1A, true) => Some('{'),
            (0x1B, false) => Some(']'),
            (0x1B, true) => Some('}'),

            // Middle letter row
            (0x1E, false) => Some('a'),
            (0x1E, true) => Some('A'),
            (0x1F, false) => Some('s'),
            (0x1F, true) => Some('S'),
            (0x20, false) => Some('d'),
            (0x20, true) => Some('D'),
            (0x21, false) => Some('f'),
            (0x21, true) => Some('F'),
            (0x22, false) => Some('g'),
            (0x22, true) => Some('G'),
            (0x23, false) => Some('h'),
            (0x23, true) => Some('H'),
            (0x24, false) => Some('j'),
            (0x24, true) => Some('J'),
            (0x25, false) => Some('k'),
            (0x25, true) => Some('K'),
            (0x26, false) => Some('l'),
            (0x26, true) => Some('L'),
            (0x27, false) => Some(';'),
            (0x27, true) => Some(':'),
            (0x28, false) => Some('\''),
            (0x28, true) => Some('"'),  // THIS IS THE FIX!

            // Bottom letter row
            (0x2C, false) => Some('z'),
            (0x2C, true) => Some('Z'),
            (0x2D, false) => Some('x'),
            (0x2D, true) => Some('X'),
            (0x2E, false) => Some('c'),
            (0x2E, true) => Some('C'),
            (0x2F, false) => Some('v'),
            (0x2F, true) => Some('V'),
            (0x30, false) => Some('b'),
            (0x30, true) => Some('B'),
            (0x31, false) => Some('n'),
            (0x31, true) => Some('N'),
            (0x32, false) => Some('m'),
            (0x32, true) => Some('M'),
            (0x33, false) => Some(','),
            (0x33, true) => Some('<'),
            (0x34, false) => Some('.'),
            (0x34, true) => Some('>'),
            (0x35, false) => Some('/'),
            (0x35, true) => Some('?'),

            // Special keys (backslash and backtick)
            (0x29, false) => Some('`'),
            (0x29, true) => Some('~'),
            (0x2B, false) => Some('\\'),
            (0x2B, true) => Some('|'),

            // Control keys (not affected by shift)
            (0x39, _) => Some(' '),    // Space
            (0x1C, _) => Some('\n'),   // Enter
            (0x0E, _) => Some('\x08'), // Backspace
            (0x48, _) => Some('\x01'), // Up arrow
            (0x50, _) => Some('\x02'), // Down arrow

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
