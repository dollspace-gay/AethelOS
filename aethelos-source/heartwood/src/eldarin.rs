//! # The Eldarin Shell
//!
//! The voice of symbiotic communion - where human intentions meet system capabilities.
//! Named after the ancient tongue of wisdom, Eldarin translates human wishes
//! into precise system actions.
//!
//! ## Philosophy
//! Commands are not orders to be blindly obeyed, but requests to be understood.
//! The shell does not merely execute - it interprets, validates, and responds
//! with both action and wisdom.

use core::fmt::Write;
use spin::Mutex;
use core::mem::MaybeUninit;

/// Maximum command buffer size
const BUFFER_SIZE: usize = 256;

/// The Scroll Buffer - stores the command being typed
pub struct CommandBuffer {
    buffer: [u8; BUFFER_SIZE],
    position: usize,
}

impl CommandBuffer {
    pub const fn new() -> Self {
        CommandBuffer {
            buffer: [0; BUFFER_SIZE],
            position: 0,
        }
    }

    /// Add a character to the buffer
    pub fn push(&mut self, ch: char) -> bool {
        if self.position < BUFFER_SIZE {
            self.buffer[self.position] = ch as u8;
            self.position += 1;
            true
        } else {
            false // Buffer full
        }
    }

    /// Remove the last character (backspace)
    pub fn pop(&mut self) -> bool {
        if self.position > 0 {
            self.position -= 1;
            self.buffer[self.position] = 0;
            true
        } else {
            false
        }
    }

    /// Get the current command as a string slice
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.position]).unwrap_or("")
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.position = 0;
        self.buffer = [0; BUFFER_SIZE];
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.position == 0
    }
}

/// Global command buffer
static mut COMMAND_BUFFER: MaybeUninit<Mutex<CommandBuffer>> = MaybeUninit::uninit();
static mut BUFFER_INITIALIZED: bool = false;

/// Initialize the shell
pub fn init() {
    unsafe {
        let buffer = CommandBuffer::new();
        let mutex = Mutex::new(buffer);
        core::ptr::write(COMMAND_BUFFER.as_mut_ptr(), mutex);
        BUFFER_INITIALIZED = true;
    }
}

/// Get reference to command buffer
unsafe fn get_buffer() -> &'static Mutex<CommandBuffer> {
    COMMAND_BUFFER.assume_init_ref()
}

/// Handle a character from keyboard input
pub fn handle_char(ch: char) {
    unsafe {
        if !BUFFER_INITIALIZED {
            return;
        }

        match ch {
            '\n' => {
                // Enter pressed - just buffer it
                // We can't safely execute commands from interrupt context
                let mut buffer = get_buffer().lock();
                buffer.push('\n');
            }
            '\x08' => {
                // Backspace
                let mut buffer = get_buffer().lock();
                if buffer.pop() {
                    // Visual backspace already handled by keyboard driver
                }
            }
            _ => {
                // Regular character
                let mut buffer = get_buffer().lock();
                if buffer.push(ch) {
                    // Character already echoed by keyboard driver
                } else {
                    // Buffer full - could beep or show error
                }
            }
        }
    }
}

/// Check for pending commands and execute them (call from shell thread)
pub fn poll() {
    unsafe {
        if !BUFFER_INITIALIZED {
            return;
        }

        // Check if buffer contains a newline
        let should_execute = {
            let buffer = get_buffer().lock();
            let has_newline = buffer.as_str().contains('\n');
            if has_newline {
                // Debug: we found a newline!
                let mut port = 0x3f8u16;
                core::arch::asm!(
                    "out dx, al",
                    in("dx") port,
                    in("al") b'!' as u8,
                    options(nomem, nostack, preserves_flags)
                );
            }
            has_newline
        };

        if should_execute {
            // Copy command out of buffer
            let mut cmd_copy: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            let cmd_len = {
                let buffer = get_buffer().lock();
                let s = buffer.as_str();
                let len = if let Some(pos) = s.find('\n') {
                    pos
                } else {
                    s.len()
                };
                cmd_copy[..len].copy_from_slice(&s.as_bytes()[..len]);
                len
            };

            // Clear buffer
            {
                let mut buffer = get_buffer().lock();
                buffer.clear();
            }

            // Execute command
            if cmd_len > 0 {
                if let Ok(cmd_str) = core::str::from_utf8(&cmd_copy[..cmd_len]) {
                    execute_command(cmd_str);
                }
            }

            display_prompt();
        }
    }
}

/// Display the shell prompt
pub fn display_prompt() {
    // Debug: signal that we're about to print prompt
    unsafe {
        let mut port = 0x3f8u16;
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") b'>' as u8,
            options(nomem, nostack, preserves_flags)
        );
    }
    crate::print!("aethel> ");
}

/// The Lexicon - Parse a command into command name and arguments
fn parse_command(input: &str) -> (&str, &str) {
    let trimmed = input.trim();

    if let Some(space_pos) = trimmed.find(' ') {
        let (cmd, args) = trimmed.split_at(space_pos);
        (cmd, args.trim())
    } else {
        (trimmed, "")
    }
}

/// The Codex - Execute a parsed command
fn execute_command(input: &str) {
    let (command, args) = parse_command(input);

    match command {
        "harmony" => cmd_harmony(),
        "echo" => cmd_echo(args),
        "clear" => cmd_clear(),
        "help" => cmd_help(),
        "" => {
            // Empty command, just show new prompt
        }
        _ => {
            crate::println!("Unknown command: '{}'. Type 'help' for available commands.", command);
        }
    }
}

// ==================== THE SPELLS ====================

/// The Harmony Spell - Display system harmony and scheduler statistics
fn cmd_harmony() {
    crate::println!("◈ System Harmony Report");
    crate::println!();

    let stats = crate::loom_of_fate::stats();

    // Check if we're in single-threaded mode
    if stats.total_threads == 0 {
        crate::println!("  Mode: Single-threaded (threading disabled for testing)");
        crate::println!("    • Main Loop: Active and responding");
        crate::println!("    • Keyboard Input: Functioning");
        crate::println!("    • Shell: Processing commands");
    } else {
        crate::println!("  Threads:");
        crate::println!("    • Total: {}", stats.total_threads);
        crate::println!("    • Weaving: {}", stats.weaving_threads);
        crate::println!("    • Resting: {}", stats.resting_threads);
        crate::println!("    • Tangled: {}", stats.tangled_threads);
    }
    crate::println!();

    crate::println!("  Harmony Metrics:");
    crate::println!("    • System Harmony: {:.2}", stats.system_harmony);
    crate::println!("    • Average Harmony: {:.2}", stats.average_harmony);
    crate::println!("    • Parasites Detected: {}", stats.parasite_count);
    crate::println!();

    crate::println!("  Performance:");
    crate::println!("    • Context Switches: {}", stats.context_switches);
    crate::println!();

    // Interpret the harmony level
    if stats.system_harmony >= 0.9 {
        crate::println!("  Status: ✓ The realm is in perfect harmony");
    } else if stats.system_harmony >= 0.7 {
        crate::println!("  Status: ◈ The realm is harmonious");
    } else if stats.system_harmony >= 0.5 {
        crate::println!("  Status: ⚠ Minor disharmony detected");
    } else {
        crate::println!("  Status: ⚠ The realm requires attention");
    }
}

/// The Echo Spell - Repeat the arguments (tests the parser)
fn cmd_echo(args: &str) {
    if args.is_empty() {
        crate::println!();
    } else {
        crate::println!("{}", args);
    }
}

/// The Clear Spell - Clear the screen
fn cmd_clear() {
    crate::vga_buffer::clear_screen();
    crate::vga_buffer::print_banner();
    crate::println!();
}

/// The Help Spell - Show available commands
fn cmd_help() {
    crate::println!("◈ Eldarin Shell - Available Commands");
    crate::println!();
    crate::println!("  harmony     - Display system harmony and scheduler statistics");
    crate::println!("  echo [text] - Echo the provided text back to the screen");
    crate::println!("  clear       - Clear the screen and redisplay the banner");
    crate::println!("  help        - Show this help message");
    crate::println!();
    crate::println!("The shell listens to your intentions and translates them into action.");
}
