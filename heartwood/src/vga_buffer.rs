//! VGA Buffer - The First Voice of the Heartwood
//!
//! Provides early text output before the full Weave is initialized.

use core::fmt;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// Higher-half VGA buffer address - physical 0xB8000 mapped to top 2GB
pub const KERNEL_VMA: usize = 0xFFFFFFFF80000000;
pub const VGA_BUFFER_PHYS: usize = 0xB8000;
pub const VGA_BUFFER_ADDRESS: usize = KERNEL_VMA + VGA_BUFFER_PHYS; // 0xFFFFFFFF800B8000

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: ColorCode,
    buffer: *mut Buffer,  // Raw pointer instead of reference to avoid lifetime issues
}

/// VGA cursor control via I/O ports
fn move_cursor(row: usize, col: usize) {
    let pos = (row * BUFFER_WIDTH + col) as u16;

    unsafe {
        // Command port (0x3D4) - tell VGA which register to write
        // Data port (0x3D5) - write the data

        // Set high byte of cursor position
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3D4u16,
            in("al") 0x0Eu8,  // Register 14: Cursor Location High
            options(nomem, nostack, preserves_flags)
        );
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3D5u16,
            in("al") ((pos >> 8) & 0xFF) as u8,
            options(nomem, nostack, preserves_flags)
        );

        // Set low byte of cursor position
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3D4u16,
            in("al") 0x0Fu8,  // Register 15: Cursor Location Low
            options(nomem, nostack, preserves_flags)
        );
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3D5u16,
            in("al") (pos & 0xFF) as u8,
            options(nomem, nostack, preserves_flags)
        );
    }
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\x08' => {
                // Backspace: move cursor back and erase character
                if self.column_position > 0 {
                    self.column_position -= 1;
                    let row = self.row_position;
                    let col = self.column_position;

                    // Write a space to erase the character
                    unsafe {
                        let buffer = &mut *self.buffer;
                        core::ptr::write_volatile(
                            &mut buffer.chars[row][col] as *mut ScreenChar,
                            ScreenChar {
                                ascii_character: b' ',
                                color_code: self.color_code,
                            }
                        );
                    }

                    move_cursor(self.row_position, self.column_position);
                }
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = self.row_position;
                let col = self.column_position;

                let color_code = self.color_code;

                // Use volatile write to prevent compiler optimization
                unsafe {
                    let buffer = &mut *self.buffer;
                    core::ptr::write_volatile(
                        &mut buffer.chars[row][col] as *mut ScreenChar,
                        ScreenChar {
                            ascii_character: byte,
                            color_code,
                        }
                    );
                }

                self.column_position += 1;
                move_cursor(self.row_position, self.column_position);
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\x08' => self.write_byte(byte),
                _ => self.write_byte(0xfe), // ■ for unprintable
            }
        }
    }

    fn new_line(&mut self) {
        self.column_position = 0;

        if self.row_position < BUFFER_HEIGHT - 1 {
            // Haven't filled screen yet, just move to next row
            self.row_position += 1;
        } else {
            // Screen is full, scroll up
            unsafe {
                let buffer = &mut *self.buffer;
                for row in 1..BUFFER_HEIGHT {
                    for col in 0..BUFFER_WIDTH {
                        let character = core::ptr::read_volatile(
                            &buffer.chars[row][col] as *const ScreenChar
                        );
                        core::ptr::write_volatile(
                            &mut buffer.chars[row - 1][col] as *mut ScreenChar,
                            character
                        );
                    }
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
        }

        move_cursor(self.row_position, self.column_position);
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        unsafe {
            let buffer = &mut *self.buffer;
            for col in 0..BUFFER_WIDTH {
                core::ptr::write_volatile(
                    &mut buffer.chars[row][col] as *mut ScreenChar,
                    blank
                );
            }
        }
    }

    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
    // Default write_fmt() implementation now works correctly with PIC relocation model
}

// Global VGA writer - simple static without mutex for early boot
// TODO: Add proper synchronization once system is fully initialized
static mut WRITER: Option<Writer> = None;
static mut WRITER_INITIALIZED: bool = false;

/// Initialize the VGA text mode writer
pub fn initialize() {
    unsafe {
        serial_out(b'['); // Start of initialize
        serial_out(b'1'); // Before clearing screen

        // Clear the screen directly without creating Writer yet
        let vga = VGA_BUFFER_ADDRESS as *mut u16;

        // Clear entire screen with space characters (white on black)
        for i in 0..(BUFFER_HEIGHT * BUFFER_WIDTH) {
            core::ptr::write_volatile(vga.add(i), 0x0F20); // Space in white on black
        }

        serial_out(b'2'); // Screen cleared

        move_cursor(0, 0);

        serial_out(b'3'); // Cursor moved

        // Mark as initialized - WRITER will be created lazily on first print
        WRITER_INITIALIZED = true;

        serial_out(b']'); // End of initialize
    }
}

/// Get reference to WRITER (creates it lazily on first access)
fn get_writer() -> &'static mut Writer {
    unsafe {
        if WRITER.is_none() {
            serial_out(b'{'); // Creating WRITER lazily

            let writer = Writer {
                column_position: 0,
                row_position: 0,
                color_code: ColorCode::new(Color::LightCyan, Color::Black),
                buffer: VGA_BUFFER_ADDRESS as *mut Buffer,
            };

            serial_out(b'W'); // Writer struct created

            WRITER = Some(writer);

            serial_out(b'}'); // WRITER created
        }

        WRITER.as_mut().unwrap()
    }
}

/// Force unlock the VGA writer (for use before context switches)
///
/// No-op now that we're not using a mutex during early boot
pub unsafe fn force_unlock() {
    // No-op - no mutex to unlock
}

/// DEBUG: Test write_str directly without write_fmt
pub unsafe fn test_write_str() {
    if WRITER_INITIALIZED {
        let writer = get_writer();
        use core::fmt::Write;
        let _ = writer.write_str("DIRECT\n");
    }
}

/// Write a single character without locking (for interrupt handlers)
pub unsafe fn write_char_unlocked(ch: char) {
    if !WRITER_INITIALIZED || WRITER.is_none() {
        return;
    }

    let writer = WRITER.as_mut().unwrap();

    // Write the character directly
    if ch == '\n' {
        writer.new_line();
    } else if ch == '\x08' {
        // Backspace - move cursor back, write space, move back again
        if writer.column_position > 0 {
            writer.column_position -= 1;
            let row = writer.row_position;
            let col = writer.column_position;
            let blank = ScreenChar {
                ascii_character: b' ',
                color_code: writer.color_code,
            };
            let buffer = &mut *writer.buffer;
            core::ptr::write_volatile(
                &mut buffer.chars[row][col] as *mut ScreenChar,
                blank
            );
            move_cursor(row, col);
        }
    } else {
        writer.write_byte(ch as u8);
    }
}

/// Clear the entire screen
pub fn clear_screen() {
    unsafe {
        if !WRITER_INITIALIZED {
            return;
        }

        let writer = get_writer();
        for row in 0..BUFFER_HEIGHT {
            writer.clear_row(row);
        }
        writer.column_position = 0;
        writer.row_position = 0;
        move_cursor(0, 0);
    }
}

pub fn print_banner() {
    unsafe {
        serial_out(b'1');
        let writer = get_writer();

        serial_out(b'2');
        writer.set_color(Color::LightCyan, Color::Black);
        serial_out(b'3');
        writer.write_string("\n");
        serial_out(b'4');
        writer.write_string("  ====================================\n");
        serial_out(b'5');
        writer.set_color(Color::White, Color::Black);
        writer.write_string("       AethelOS - The Heartwood\n");
        serial_out(b'6');
        writer.set_color(Color::LightGray, Color::Black);
        writer.write_string("    Symbiotic Computing Awakens\n");
        serial_out(b'7');
        writer.set_color(Color::LightCyan, Color::Black);
        writer.write_string("  ====================================\n");
        serial_out(b'8');
        writer.set_color(Color::LightCyan, Color::Black);
        writer.write_string("\n");
        serial_out(b'9');
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    unsafe {
        if WRITER_INITIALIZED {
            let writer = get_writer();
            // Use proper fmt::Write trait - works correctly with PIC relocation model
            let _ = writer.write_fmt(args);
        }
    }
}
