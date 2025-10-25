//! # PS/2 Keyboard Driver
//!
//! The voice of the user - how humans commune with AethelOS.
//! Each keystroke is a word in the conversation between human and machine.
//!
//! ## Philosophy
//! Input is not commands to be obeyed, but intentions to be understood.
//! We listen to the keyboard not as servants awaiting orders,
//! but as partners engaged in dialogue.
//!
//! ## Technical Details
//! The PS/2 keyboard controller uses IRQ 1 (interrupt 33 after remapping).
//! Scancodes arrive as single or multi-byte sequences via port 0x60.
//! We translate scan codes (Set 1) into meaningful key events.

use core::arch::asm;
use core::mem::MaybeUninit;
use spin::Mutex;

/// PS/2 keyboard data port
const KEYBOARD_DATA: u16 = 0x60;

/// PS/2 keyboard status/command port
const KEYBOARD_STATUS: u16 = 0x64;

/// Status register flags
const STATUS_OUTPUT_FULL: u8 = 0x01;

/// Global keyboard state using MaybeUninit to avoid allocator dependency
static mut KEYBOARD: MaybeUninit<Mutex<Keyboard>> = MaybeUninit::uninit();
static mut KEYBOARD_INITIALIZED: bool = false;

/// Get reference to keyboard (assumes initialized)
unsafe fn get_keyboard() -> &'static Mutex<Keyboard> {
    KEYBOARD.assume_init_ref()
}

/// Keyboard modifier state
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub left_shift: bool,
    pub right_shift: bool,
    pub left_ctrl: bool,
    pub right_ctrl: bool,
    pub left_alt: bool,
    pub right_alt: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
    pub scroll_lock: bool,
}

impl Modifiers {
    /// Check if any shift key is pressed
    pub fn shift(&self) -> bool {
        self.left_shift || self.right_shift
    }

    /// Check if any ctrl key is pressed
    pub fn ctrl(&self) -> bool {
        self.left_ctrl || self.right_ctrl
    }

    /// Check if any alt key is pressed
    pub fn alt(&self) -> bool {
        self.left_alt || self.right_alt
    }
}

/// A keyboard event
#[derive(Debug, Clone, Copy)]
pub enum KeyEvent {
    /// Key was pressed
    Pressed(Key),
    /// Key was released
    Released(Key),
}

/// Represents a key on the keyboard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers
    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,

    // Function keys
    F1, F2, F3, F4, F5, F6,
    F7, F8, F9, F10, F11, F12,

    // Special keys
    Escape,
    Backspace,
    Tab,
    Enter,
    Space,

    // Modifiers
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,

    // Lock keys
    CapsLock,
    NumLock,
    ScrollLock,

    // Navigation
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Punctuation and symbols
    Minus,        // -
    Equal,        // =
    LeftBracket,  // [
    RightBracket, // ]
    Backslash,    // \
    Semicolon,    // ;
    Quote,        // '
    Grave,        // `
    Comma,        // ,
    Period,       // .
    Slash,        // /

    // Numpad
    NumpadDivide,
    NumpadMultiply,
    NumpadMinus,
    NumpadPlus,
    NumpadEnter,
    NumpadPeriod,
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,

    // Unknown key
    Unknown(u8),
}

impl Key {
    /// Convert key to ASCII character if possible
    pub fn to_ascii(&self, modifiers: &Modifiers) -> Option<char> {
        let shift = modifiers.shift() ^ modifiers.caps_lock;

        match self {
            // Letters
            Key::A => Some(if shift { 'A' } else { 'a' }),
            Key::B => Some(if shift { 'B' } else { 'b' }),
            Key::C => Some(if shift { 'C' } else { 'c' }),
            Key::D => Some(if shift { 'D' } else { 'd' }),
            Key::E => Some(if shift { 'E' } else { 'e' }),
            Key::F => Some(if shift { 'F' } else { 'f' }),
            Key::G => Some(if shift { 'G' } else { 'g' }),
            Key::H => Some(if shift { 'H' } else { 'h' }),
            Key::I => Some(if shift { 'I' } else { 'i' }),
            Key::J => Some(if shift { 'J' } else { 'j' }),
            Key::K => Some(if shift { 'K' } else { 'k' }),
            Key::L => Some(if shift { 'L' } else { 'l' }),
            Key::M => Some(if shift { 'M' } else { 'm' }),
            Key::N => Some(if shift { 'N' } else { 'n' }),
            Key::O => Some(if shift { 'O' } else { 'o' }),
            Key::P => Some(if shift { 'P' } else { 'p' }),
            Key::Q => Some(if shift { 'Q' } else { 'q' }),
            Key::R => Some(if shift { 'R' } else { 'r' }),
            Key::S => Some(if shift { 'S' } else { 's' }),
            Key::T => Some(if shift { 'T' } else { 't' }),
            Key::U => Some(if shift { 'U' } else { 'u' }),
            Key::V => Some(if shift { 'V' } else { 'v' }),
            Key::W => Some(if shift { 'W' } else { 'w' }),
            Key::X => Some(if shift { 'X' } else { 'x' }),
            Key::Y => Some(if shift { 'Y' } else { 'y' }),
            Key::Z => Some(if shift { 'Z' } else { 'z' }),

            // Numbers
            Key::Num0 => Some(if modifiers.shift() { ')' } else { '0' }),
            Key::Num1 => Some(if modifiers.shift() { '!' } else { '1' }),
            Key::Num2 => Some(if modifiers.shift() { '@' } else { '2' }),
            Key::Num3 => Some(if modifiers.shift() { '#' } else { '3' }),
            Key::Num4 => Some(if modifiers.shift() { '$' } else { '4' }),
            Key::Num5 => Some(if modifiers.shift() { '%' } else { '5' }),
            Key::Num6 => Some(if modifiers.shift() { '^' } else { '6' }),
            Key::Num7 => Some(if modifiers.shift() { '&' } else { '7' }),
            Key::Num8 => Some(if modifiers.shift() { '*' } else { '8' }),
            Key::Num9 => Some(if modifiers.shift() { '(' } else { '9' }),

            // Special keys
            Key::Space => Some(' '),
            Key::Enter => Some('\n'),
            Key::Tab => Some('\t'),
            Key::Backspace => Some('\x08'),

            // Punctuation
            Key::Minus => Some(if modifiers.shift() { '_' } else { '-' }),
            Key::Equal => Some(if modifiers.shift() { '+' } else { '=' }),
            Key::LeftBracket => Some(if modifiers.shift() { '{' } else { '[' }),
            Key::RightBracket => Some(if modifiers.shift() { '}' } else { ']' }),
            Key::Backslash => Some(if modifiers.shift() { '|' } else { '\\' }),
            Key::Semicolon => Some(if modifiers.shift() { ':' } else { ';' }),
            Key::Quote => Some(if modifiers.shift() { '"' } else { '\'' }),
            Key::Grave => Some(if modifiers.shift() { '~' } else { '`' }),
            Key::Comma => Some(if modifiers.shift() { '<' } else { ',' }),
            Key::Period => Some(if modifiers.shift() { '>' } else { '.' }),
            Key::Slash => Some(if modifiers.shift() { '?' } else { '/' }),

            // Numpad (when numlock is on)
            Key::Numpad0 => if modifiers.num_lock { Some('0') } else { None },
            Key::Numpad1 => if modifiers.num_lock { Some('1') } else { None },
            Key::Numpad2 => if modifiers.num_lock { Some('2') } else { None },
            Key::Numpad3 => if modifiers.num_lock { Some('3') } else { None },
            Key::Numpad4 => if modifiers.num_lock { Some('4') } else { None },
            Key::Numpad5 => if modifiers.num_lock { Some('5') } else { None },
            Key::Numpad6 => if modifiers.num_lock { Some('6') } else { None },
            Key::Numpad7 => if modifiers.num_lock { Some('7') } else { None },
            Key::Numpad8 => if modifiers.num_lock { Some('8') } else { None },
            Key::Numpad9 => if modifiers.num_lock { Some('9') } else { None },
            Key::NumpadPeriod => if modifiers.num_lock { Some('.') } else { None },
            Key::NumpadDivide => Some('/'),
            Key::NumpadMultiply => Some('*'),
            Key::NumpadMinus => Some('-'),
            Key::NumpadPlus => Some('+'),
            Key::NumpadEnter => Some('\n'),

            // Non-printable keys
            _ => None,
        }
    }
}

/// PS/2 Keyboard state machine
pub struct Keyboard {
    modifiers: Modifiers,
    extended_scancode: bool,
}

impl Keyboard {
    /// Create a new keyboard state
    pub const fn new() -> Self {
        Keyboard {
            modifiers: Modifiers {
                left_shift: false,
                right_shift: false,
                left_ctrl: false,
                right_ctrl: false,
                left_alt: false,
                right_alt: false,
                caps_lock: false,
                num_lock: false,
                scroll_lock: false,
            },
            extended_scancode: false,
        }
    }

    /// Get current modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Process a scancode byte and return a key event
    pub fn process_scancode(&mut self, scancode: u8) -> Option<KeyEvent> {
        // Debug: output raw scancode for Enter key
        unsafe {
            if scancode == 0x1C || scancode == 0x9C {  // Enter make/break
                serial_out(b'[');
                serial_out(if scancode == 0x1C { b'M' } else { b'B' });  // M=make, B=break
                serial_out(b']');
            }
        }

        // Handle extended scancodes (0xE0 prefix)
        if scancode == 0xE0 {
            self.extended_scancode = true;
            return None;
        }

        // Check if this is a key release (high bit set)
        let released = (scancode & 0x80) != 0;
        let code = scancode & 0x7F;

        // Translate scancode to key
        let key = if self.extended_scancode {
            self.extended_scancode = false;
            match code {
                0x1D => Key::RightCtrl,
                0x38 => Key::RightAlt,
                0x48 => Key::ArrowUp,
                0x50 => Key::ArrowDown,
                0x4B => Key::ArrowLeft,
                0x4D => Key::ArrowRight,
                0x49 => Key::PageUp,
                0x51 => Key::PageDown,
                0x47 => Key::Home,
                0x4F => Key::End,
                0x52 => Key::Insert,
                0x53 => Key::Delete,
                0x35 => Key::NumpadDivide,
                _ => Key::Unknown(scancode),
            }
        } else {
            scancode_to_key(code)
        };

        // Update modifier state
        self.update_modifiers(&key, released);

        // Return the event
        if released {
            Some(KeyEvent::Released(key))
        } else {
            Some(KeyEvent::Pressed(key))
        }
    }

    /// Update modifier key states
    fn update_modifiers(&mut self, key: &Key, released: bool) {
        match key {
            Key::LeftShift => self.modifiers.left_shift = !released,
            Key::RightShift => self.modifiers.right_shift = !released,
            Key::LeftCtrl => self.modifiers.left_ctrl = !released,
            Key::RightCtrl => self.modifiers.right_ctrl = !released,
            Key::LeftAlt => self.modifiers.left_alt = !released,
            Key::RightAlt => self.modifiers.right_alt = !released,

            // Toggle lock keys on press
            Key::CapsLock if !released => self.modifiers.caps_lock = !self.modifiers.caps_lock,
            Key::NumLock if !released => self.modifiers.num_lock = !self.modifiers.num_lock,
            Key::ScrollLock if !released => self.modifiers.scroll_lock = !self.modifiers.scroll_lock,

            _ => {}
        }
    }
}

/// Translate a scancode (Set 1) to a Key
fn scancode_to_key(code: u8) -> Key {
    match code {
        // Row 1
        0x01 => Key::Escape,
        0x3B => Key::F1,
        0x3C => Key::F2,
        0x3D => Key::F3,
        0x3E => Key::F4,
        0x3F => Key::F5,
        0x40 => Key::F6,
        0x41 => Key::F7,
        0x42 => Key::F8,
        0x43 => Key::F9,
        0x44 => Key::F10,
        0x57 => Key::F11,
        0x58 => Key::F12,

        // Row 2
        0x29 => Key::Grave,
        0x02 => Key::Num1,
        0x03 => Key::Num2,
        0x04 => Key::Num3,
        0x05 => Key::Num4,
        0x06 => Key::Num5,
        0x07 => Key::Num6,
        0x08 => Key::Num7,
        0x09 => Key::Num8,
        0x0A => Key::Num9,
        0x0B => Key::Num0,
        0x0C => Key::Minus,
        0x0D => Key::Equal,
        0x0E => Key::Backspace,

        // Row 3
        0x0F => Key::Tab,
        0x10 => Key::Q,
        0x11 => Key::W,
        0x12 => Key::E,
        0x13 => Key::R,
        0x14 => Key::T,
        0x15 => Key::Y,
        0x16 => Key::U,
        0x17 => Key::I,
        0x18 => Key::O,
        0x19 => Key::P,
        0x1A => Key::LeftBracket,
        0x1B => Key::RightBracket,
        0x2B => Key::Backslash,

        // Row 4
        0x3A => Key::CapsLock,
        0x1E => Key::A,
        0x1F => Key::S,
        0x20 => Key::D,
        0x21 => Key::F,
        0x22 => Key::G,
        0x23 => Key::H,
        0x24 => Key::J,
        0x25 => Key::K,
        0x26 => Key::L,
        0x27 => Key::Semicolon,
        0x28 => Key::Quote,
        0x1C => Key::Enter,

        // Row 5
        0x2A => Key::LeftShift,
        0x2C => Key::Z,
        0x2D => Key::X,
        0x2E => Key::C,
        0x2F => Key::V,
        0x30 => Key::B,
        0x31 => Key::N,
        0x32 => Key::M,
        0x33 => Key::Comma,
        0x34 => Key::Period,
        0x35 => Key::Slash,
        0x36 => Key::RightShift,

        // Row 6
        0x1D => Key::LeftCtrl,
        0x38 => Key::LeftAlt,
        0x39 => Key::Space,

        // Numpad
        0x45 => Key::NumLock,
        0x46 => Key::ScrollLock,
        0x47 => Key::Numpad7,
        0x48 => Key::Numpad8,
        0x49 => Key::Numpad9,
        0x4A => Key::NumpadMinus,
        0x4B => Key::Numpad4,
        0x4C => Key::Numpad5,
        0x4D => Key::Numpad6,
        0x4E => Key::NumpadPlus,
        0x4F => Key::Numpad1,
        0x50 => Key::Numpad2,
        0x51 => Key::Numpad3,
        0x52 => Key::Numpad0,
        0x53 => Key::NumpadPeriod,
        0x37 => Key::NumpadMultiply,

        // Unknown
        _ => Key::Unknown(code),
    }
}

/// Read a scancode from the keyboard data port
///
/// # Safety
/// Reads from I/O port 0x60. Should only be called from keyboard interrupt handler.
pub unsafe fn read_scancode() -> u8 {
    inb(KEYBOARD_DATA)
}

/// Check if keyboard data is available
pub unsafe fn data_available() -> bool {
    (inb(KEYBOARD_STATUS) & STATUS_OUTPUT_FULL) != 0
}

/// Helper function to input a byte from an I/O port
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

/// Initialize the keyboard
pub fn init() {
    unsafe {
        serial_out(b'['); // Keyboard init start

        // Read PS/2 controller configuration
        asm!("out dx, al", in("dx") 0x64u16, in("al") 0x20u8, options(nomem, nostack, preserves_flags));
        for _ in 0..1000 { core::hint::spin_loop(); }

        let mut config: u8;
        asm!("in al, dx", out("al") config, in("dx") 0x60u16, options(nomem, nostack, preserves_flags));
        serial_out(config); // Show config before modification

        // Enable keyboard interrupt in PS/2 controller (bit 0 = keyboard interrupt enable)
        config |= 0x01;

        // Write back configuration
        asm!("out dx, al", in("dx") 0x64u16, in("al") 0x60u8, options(nomem, nostack, preserves_flags));
        for _ in 0..1000 { core::hint::spin_loop(); }
        asm!("out dx, al", in("dx") 0x60u16, in("al") config, options(nomem, nostack, preserves_flags));
        serial_out(b'W'); // Wrote config

        // Initialize keyboard state
        let keyboard = Keyboard::new();
        let mutex = Mutex::new(keyboard);
        core::ptr::write(KEYBOARD.as_mut_ptr(), mutex);
        KEYBOARD_INITIALIZED = true;

        serial_out(b'K'); // Keyboard struct initialized

        // Enable IRQ 1 in PIC
        super::get_pic().lock().enable_irq(1);

        serial_out(b']'); // Keyboard init complete
    }
}

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Handle a keyboard interrupt (called from IRQ 1 handler)
pub fn on_interrupt() {
    unsafe {
        serial_out(b'K'); // Keyboard interrupt received!

        if !KEYBOARD_INITIALIZED {
            serial_out(b'!'); // Not initialized
            return;
        }

        // ALWAYS read the scancode when interrupt fires
        // The interrupt itself tells us data is available!
        let scancode = read_scancode();
        serial_out(b'S'); // Scancode read
        serial_out(scancode); // Echo the raw scancode

        // Filter out command responses (not actual scancodes)
        match scancode {
            0xFA => { serial_out(b'A'); return; } // ACK - ignore
            0xFE => { serial_out(b'R'); return; } // Resend - ignore
            0xAA => { serial_out(b'T'); return; } // Self-test passed - ignore
            0xFC | 0xFD => { serial_out(b'E'); return; } // Error codes - ignore
            _ => {} // Process normal scancodes
        }

        let mut keyboard = get_keyboard().lock();

        if let Some(event) = keyboard.process_scancode(scancode) {
            serial_out(b'E'); // Event generated
            handle_key_event(event, &keyboard.modifiers());
        } else {
            serial_out(b'N'); // No event (probably multi-byte sequence)
        }
    }
}

/// Handle a key event (override this for custom behavior)
fn handle_key_event(event: KeyEvent, modifiers: &Modifiers) {
    match event {
        KeyEvent::Pressed(key) => {
            // Debug: output the key type
            unsafe {
                match key {
                    Key::Enter => serial_out(b'@'),  // @ for Enter key
                    _ => {}
                }
            }

            if let Some(ch) = key.to_ascii(modifiers) {
                unsafe { serial_out(b'C'); } // Character ready
                unsafe { serial_out(ch as u8); } // Echo the actual character to serial

                // Echo character to screen using lock-free write (visual feedback)
                // This is safe because we're in an interrupt handler and need to avoid deadlock
                unsafe {
                    crate::vga_buffer::write_char_unlocked(ch);
                    serial_out(b'P'); // Printed to screen
                }

                // Route character to Eldarin shell for processing
                crate::eldarin::handle_char(ch);
            } else {
                // Non-ASCII key pressed
                unsafe { serial_out(b'X'); } // Non-printable key
            }
        }
        KeyEvent::Released(_) => {
            unsafe { serial_out(b'R'); } // Key released
        }
    }
}
