//! # Keyboard - The Voice of Intent
//!
//! Minimal keyboard initialization - we trust that BIOS has already set up the PS/2 controller

use spin::Mutex;
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

static mut KEYBOARD: MaybeUninit<Mutex<Keyboard>> = MaybeUninit::uninit();
static mut KEYBOARD_INITIALIZED: bool = false;

/// Initialize keyboard - just create the state structure
/// The BIOS has already initialized the PS/2 controller
/// The pic8259 crate will handle enabling IRQ 1
pub fn init() {
    unsafe {
        let keyboard = Keyboard::new();
        let mutex = Mutex::new(keyboard);
        core::ptr::write(KEYBOARD.as_mut_ptr(), mutex);
        KEYBOARD_INITIALIZED = true;
    }
}

/// Called when a keyboard interrupt occurs
pub fn on_interrupt() {
    unsafe {
        if !KEYBOARD_INITIALIZED {
            return;
        }

        let mut keyboard = (*KEYBOARD.as_ptr()).lock();
        let scancode = keyboard.read_scancode();

        // Ignore key release events (scancode >= 0x80)
        if scancode >= 0x80 {
            return;
        }

        // Basic scancode to character mapping (US QWERTY, lowercase only)
        // Only handle key PRESS events
        let ch = match scancode {
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
            0x1E => Some('a'),
            0x1F => Some('s'),
            0x20 => Some('d'),
            0x21 => Some('f'),
            0x22 => Some('g'),
            0x23 => Some('h'),
            0x24 => Some('j'),
            0x25 => Some('k'),
            0x26 => Some('l'),
            0x2C => Some('z'),
            0x2D => Some('x'),
            0x2E => Some('c'),
            0x2F => Some('v'),
            0x30 => Some('b'),
            0x31 => Some('n'),
            0x32 => Some('m'),
            0x39 => Some(' '),  // Space
            0x1C => Some('\n'), // Enter
            _ => None,  // Ignore other keys
        };

        // If we got a valid character, send it to both display and shell buffer
        if let Some(character) = ch {
            crate::print!("{}", character);
            crate::eldarin::handle_char(character);
        }
    }
}
