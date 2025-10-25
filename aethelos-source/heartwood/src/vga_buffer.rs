//! VGA Buffer - The First Voice of the Heartwood
//!
//! Provides early text output before the full Weave is initialized.

use core::fmt;
use crate::irq_safe_mutex::IrqSafeMutex;
use core::mem::MaybeUninit;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

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
    buffer: &'static mut Buffer,
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
                        core::ptr::write_volatile(
                            &mut self.buffer.chars[row][col] as *mut ScreenChar,
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
                    core::ptr::write_volatile(
                        &mut self.buffer.chars[row][col] as *mut ScreenChar,
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
        unsafe {
            serial_out(b'$'); // Entering write_string
        }
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\x08' => self.write_byte(byte),
                _ => self.write_byte(0xfe), // ■ for unprintable
            }
        }
        unsafe {
            serial_out(b'%'); // Exiting write_string
        }
    }

    fn new_line(&mut self) {
        self.column_position = 0;

        if self.row_position < BUFFER_HEIGHT - 1 {
            // Haven't filled screen yet, just move to next row
            self.row_position += 1;
        } else {
            // Screen is full, scroll up
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    unsafe {
                        let character = core::ptr::read_volatile(
                            &self.buffer.chars[row][col] as *const ScreenChar
                        );
                        core::ptr::write_volatile(
                            &mut self.buffer.chars[row - 1][col] as *mut ScreenChar,
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
        for col in 0..BUFFER_WIDTH {
            unsafe {
                core::ptr::write_volatile(
                    &mut self.buffer.chars[row][col] as *mut ScreenChar,
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
        unsafe {
            serial_out(b'@'); // Entering write_str
        }
        self.write_string(s);
        unsafe {
            serial_out(b'#'); // Exiting write_str
        }
        Ok(())
    }
}

// Global VGA writer using MaybeUninit to avoid allocator dependency
static mut WRITER: MaybeUninit<IrqSafeMutex<Writer>> = MaybeUninit::uninit();
static mut WRITER_INITIALIZED: bool = false;

/// Initialize the VGA text mode writer
pub fn initialize() {
    unsafe {
        // Create writer pointing to VGA text buffer
        let writer = Writer {
            column_position: 0,
            row_position: 0,
            color_code: ColorCode::new(Color::LightCyan, Color::Black),
            buffer: &mut *(0xb8000 as *mut Buffer),
        };

        // Wrap in IRQ-safe mutex and write to static
        let mutex = IrqSafeMutex::new(writer);
        core::ptr::write(core::ptr::addr_of_mut!(WRITER).cast(), mutex);
        WRITER_INITIALIZED = true;

        // Clear the screen
        let mut writer = (*core::ptr::addr_of_mut!(WRITER).cast::<IrqSafeMutex<Writer>>()).lock();
        for row in 0..BUFFER_HEIGHT {
            writer.clear_row(row);
        }
        writer.column_position = 0;
        writer.row_position = 0;
        move_cursor(0, 0);
    }
}

/// Get reference to WRITER (assumes initialized)
unsafe fn get_writer() -> &'static IrqSafeMutex<Writer> {
    &*core::ptr::addr_of!(WRITER).cast::<IrqSafeMutex<Writer>>()
}

/// Force unlock the VGA writer (for use before context switches)
///
/// This is a dangerous function that forcibly releases the VGA buffer lock.
/// It should ONLY be called right before the Great Hand-Off when we know
/// no other code will try to use the lock.
pub unsafe fn force_unlock() {
    if WRITER_INITIALIZED {
        let writer = &mut *core::ptr::addr_of_mut!(WRITER).cast::<IrqSafeMutex<Writer>>();
        writer.force_unlock();
    }
}

/// Write a single character without locking (for interrupt handlers)
/// This is unsafe because it bypasses the mutex, but it's necessary for
/// interrupt handlers to avoid deadlock
pub unsafe fn write_char_unlocked(ch: char) {
    if !WRITER_INITIALIZED {
        return;
    }

    // Get raw pointer to the writer inside the IRQ-safe mutex
    let writer_ptr = core::ptr::addr_of_mut!(WRITER).cast::<IrqSafeMutex<Writer>>();
    // Access the inner Mutex to get the data (unsafe pointer dereference)
    let inner_mutex = &mut (*writer_ptr).inner;
    let writer = inner_mutex.get_mut();

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
            core::ptr::write_volatile(
                &mut writer.buffer.chars[row][col] as *mut ScreenChar,
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

        let mut writer = get_writer().lock();
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
        let mut writer = get_writer().lock();

        writer.set_color(Color::LightCyan, Color::Black);
        writer.write_string("\n");
        writer.write_string("  ====================================\n");
        writer.set_color(Color::White, Color::Black);
        writer.write_string("       AethelOS - The Heartwood\n");
        writer.set_color(Color::LightGray, Color::Black);
        writer.write_string("    Symbiotic Computing Awakens\n");
        writer.set_color(Color::LightCyan, Color::Black);
        writer.write_string("  ====================================\n");
        writer.set_color(Color::LightCyan, Color::Black);
        writer.write_string("\n");
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
        serial_out(b'V'); // VGA print called
        if WRITER_INITIALIZED {
            serial_out(b'W'); // Writer initialized

            // IrqSafeMutex automatically disables interrupts when locking
            // and re-enables them when the guard is dropped
            serial_out(b'G'); // About to get writer
            let mut writer = get_writer().lock();
            serial_out(b'+'); // Got lock!
            writer.write_fmt(args).unwrap();
            serial_out(b'E'); // Write complete
            serial_out(b'['); // About to drop
            drop(writer); // Explicitly drop the guard HERE
            serial_out(b']'); // Drop complete
            serial_out(b'D'); // Done writing (after lock released)
        } else {
            serial_out(b'U'); // Uninitialized!
        }
    }
}
