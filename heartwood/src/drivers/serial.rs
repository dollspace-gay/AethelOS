//! Serial Port Driver (UART 16550)
//!
//! Provides proper initialization and thread-safe output to COM1

use core::fmt;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// COM1 base port
const COM1: u16 = 0x3F8;

/// Serial port registers (offsets from base)
const DATA: u16 = 0;          // Data register (DLAB=0)
const INT_ENABLE: u16 = 1;    // Interrupt Enable (DLAB=0)
const FIFO_CTRL: u16 = 2;     // FIFO Control
const LINE_CTRL: u16 = 3;     // Line Control
const MODEM_CTRL: u16 = 4;    // Modem Control
const LINE_STATUS: u16 = 5;   // Line Status
const DIVISOR_LSB: u16 = 0;   // Divisor Latch LSB (DLAB=1)
const DIVISOR_MSB: u16 = 1;   // Divisor Latch MSB (DLAB=1)

/// Serial port instance
pub struct SerialPort {
    data: Port<u8>,
    int_enable: Port<u8>,
    fifo_ctrl: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_status: Port<u8>,
}

impl SerialPort {
    /// Create a new serial port instance (doesn't initialize hardware)
    const fn new(base: u16) -> Self {
        Self {
            data: Port::new(base + DATA),
            int_enable: Port::new(base + INT_ENABLE),
            fifo_ctrl: Port::new(base + FIFO_CTRL),
            line_ctrl: Port::new(base + LINE_CTRL),
            modem_ctrl: Port::new(base + MODEM_CTRL),
            line_status: Port::new(base + LINE_STATUS),
        }
    }

    /// Initialize the serial port
    ///
    /// Sets up 115200 baud, 8N1 (8 data bits, no parity, 1 stop bit)
    pub unsafe fn init(&mut self) {
        // Disable interrupts
        self.int_enable.write(0x00);

        // Enable DLAB (set bit 7 of line control register)
        // This allows us to set the baud rate divisor
        self.line_ctrl.write(0x80);

        // Set baud rate divisor to 1 (115200 baud)
        // Divisor = 115200 / desired_baud_rate
        // For 115200: divisor = 1
        Port::<u8>::new(COM1 + DIVISOR_LSB).write(0x01); // Divisor LSB
        Port::<u8>::new(COM1 + DIVISOR_MSB).write(0x00); // Divisor MSB

        // Configure line: 8 bits, no parity, 1 stop bit (8N1)
        // Bit 0-1: Data bits (11 = 8 bits)
        // Bit 2: Stop bits (0 = 1 stop bit)
        // Bit 3-5: Parity (000 = none)
        // Bit 7: DLAB (0 = normal mode)
        self.line_ctrl.write(0x03);

        // Enable FIFO, clear buffers, 14-byte threshold
        self.fifo_ctrl.write(0xC7);

        // Set modem control: DTR, RTS, OUT2 (required for interrupts, though we don't use them)
        self.modem_ctrl.write(0x0B);
    }

    /// Write a byte to the serial port
    pub unsafe fn write_byte(&mut self, byte: u8) {
        // Wait for transmit buffer to be empty
        while self.line_status.read() & 0x20 == 0 {}

        self.data.write(byte);
    }

    /// Write a string to the serial port
    pub unsafe fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            self.write_str(s);
        }
        Ok(())
    }
}

/// Global serial port instance
static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1));

/// Initialize the serial port (call once during boot)
pub unsafe fn init() {
    SERIAL1.lock().init();
}

/// Write a byte to COM1
pub fn write_byte(byte: u8) {
    unsafe {
        SERIAL1.lock().write_byte(byte);
    }
}

/// Write a string to COM1
pub fn write_str(s: &str) {
    unsafe {
        SERIAL1.lock().write_str(s);
    }
}

/// Macro for serial output (like print!)
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(format_args!($($arg)*))
    };
}

/// Macro for serial output with newline (like println!)
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

/// Internal print function for macro
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).unwrap();
}
