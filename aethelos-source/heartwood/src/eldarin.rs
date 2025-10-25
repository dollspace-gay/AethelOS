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
use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;

/// Maximum command buffer size
const BUFFER_SIZE: usize = 256;

/// Maximum number of commands to store in history
const HISTORY_SIZE: usize = 32;

/// The Scroll Buffer - stores the command being typed
pub struct CommandBuffer {
    buffer: [u8; BUFFER_SIZE],
    position: usize,
}

/// Command History - stores previous commands for recall
pub struct CommandHistory {
    /// Ring buffer of previous commands
    commands: [[u8; BUFFER_SIZE]; HISTORY_SIZE],
    /// Length of each command in the ring buffer
    lengths: [usize; HISTORY_SIZE],
    /// Current write position in ring buffer
    write_pos: usize,
    /// Current read position when navigating history
    read_pos: usize,
    /// Total number of commands in history (up to HISTORY_SIZE)
    count: usize,
    /// True if currently navigating history
    navigating: bool,
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

    /// Set buffer contents from a byte slice
    pub fn set_from_bytes(&mut self, bytes: &[u8]) {
        self.clear();
        let len = bytes.len().min(BUFFER_SIZE);
        self.buffer[..len].copy_from_slice(&bytes[..len]);
        self.position = len;
    }
}

impl CommandHistory {
    pub const fn new() -> Self {
        CommandHistory {
            commands: [[0; BUFFER_SIZE]; HISTORY_SIZE],
            lengths: [0; HISTORY_SIZE],
            write_pos: 0,
            read_pos: 0,
            count: 0,
            navigating: false,
        }
    }

    /// Add a command to history
    pub fn add(&mut self, cmd: &str) {
        if cmd.is_empty() {
            return;
        }

        let bytes = cmd.as_bytes();
        let len = bytes.len().min(BUFFER_SIZE);

        self.commands[self.write_pos][..len].copy_from_slice(&bytes[..len]);
        self.lengths[self.write_pos] = len;

        self.write_pos = (self.write_pos + 1) % HISTORY_SIZE;
        if self.count < HISTORY_SIZE {
            self.count += 1;
        }

        // Reset navigation state
        self.navigating = false;
        self.read_pos = self.write_pos;
    }

    /// Navigate to previous command (up arrow)
    /// Returns Some(command) if available, None if at the beginning
    pub fn previous(&mut self) -> Option<&[u8]> {
        if self.count == 0 {
            return None;
        }

        if !self.navigating {
            // First time navigating, start from most recent
            self.navigating = true;
            self.read_pos = if self.write_pos == 0 {
                self.count - 1
            } else {
                self.write_pos - 1
            };
        } else {
            // Already navigating, go back one more
            if self.read_pos == 0 {
                self.read_pos = self.count - 1;
            } else {
                self.read_pos -= 1;
            }
        }

        let len = self.lengths[self.read_pos];
        Some(&self.commands[self.read_pos][..len])
    }

    /// Navigate to next command (down arrow)
    /// Returns Some(command) if available, None if at the end
    pub fn next(&mut self) -> Option<&[u8]> {
        if !self.navigating {
            return None;
        }

        self.read_pos = (self.read_pos + 1) % self.count;

        // If we're back at the write position, stop navigating (return to empty)
        if self.read_pos == self.write_pos {
            self.navigating = false;
            return None;
        }

        let len = self.lengths[self.read_pos];
        Some(&self.commands[self.read_pos][..len])
    }
}

/// Global command buffer (interrupt-safe for keyboard input)
static mut COMMAND_BUFFER: MaybeUninit<InterruptSafeLock<CommandBuffer>> = MaybeUninit::uninit();
static mut BUFFER_INITIALIZED: bool = false;

/// Global command history
static mut COMMAND_HISTORY: MaybeUninit<InterruptSafeLock<CommandHistory>> = MaybeUninit::uninit();
static mut HISTORY_INITIALIZED: bool = false;

/// Initialize the shell
pub fn init() {
    unsafe {
        let buffer = CommandBuffer::new();
        let lock = InterruptSafeLock::new(buffer);
        core::ptr::write(COMMAND_BUFFER.as_mut_ptr(), lock);
        BUFFER_INITIALIZED = true;

        let history = CommandHistory::new();
        let history_lock = InterruptSafeLock::new(history);
        core::ptr::write(COMMAND_HISTORY.as_mut_ptr(), history_lock);
        HISTORY_INITIALIZED = true;
    }
}

/// Get reference to command buffer
unsafe fn get_buffer() -> &'static InterruptSafeLock<CommandBuffer> {
    COMMAND_BUFFER.assume_init_ref()
}

/// Get reference to command history
unsafe fn get_history() -> &'static InterruptSafeLock<CommandHistory> {
    COMMAND_HISTORY.assume_init_ref()
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

/// Handle backspace key - erase character visually and from buffer
pub fn handle_backspace() {
    unsafe {
        if !BUFFER_INITIALIZED {
            return;
        }

        let mut buffer = get_buffer().lock();
        if buffer.pop() {
            // Erase character visually (VGA driver handles the erasure)
            crate::print!("\x08");
        }
    }
}

/// Handle up arrow - navigate to previous command in history
pub fn handle_arrow_up() {
    unsafe {
        if !BUFFER_INITIALIZED || !HISTORY_INITIALIZED {
            return;
        }

        let mut history = get_history().lock();
        if let Some(cmd_bytes) = history.previous() {
            let mut buffer = get_buffer().lock();
            let current_len = buffer.as_str().len();

            // Erase current line (VGA driver erases each character as we backspace)
            for _ in 0..current_len {
                crate::print!("\x08");
            }

            // Set buffer to historical command and display it
            buffer.set_from_bytes(cmd_bytes);
            if let Ok(cmd_str) = core::str::from_utf8(cmd_bytes) {
                crate::print!("{}", cmd_str);
            }
        }
    }
}

/// Handle down arrow - navigate to next command in history
pub fn handle_arrow_down() {
    unsafe {
        if !BUFFER_INITIALIZED || !HISTORY_INITIALIZED {
            return;
        }

        let mut history = get_history().lock();
        let mut buffer = get_buffer().lock();
        let current_len = buffer.as_str().len();

        // Erase current line (VGA driver erases each character as we backspace)
        for _ in 0..current_len {
            crate::print!("\x08");
        }

        if let Some(cmd_bytes) = history.next() {
            // Set buffer to next historical command and display it
            buffer.set_from_bytes(cmd_bytes);
            if let Ok(cmd_str) = core::str::from_utf8(cmd_bytes) {
                crate::print!("{}", cmd_str);
            }
        } else {
            // At the end of history, clear buffer
            buffer.clear();
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

            // Execute command and save to history
            if cmd_len > 0 {
                if let Ok(cmd_str) = core::str::from_utf8(&cmd_copy[..cmd_len]) {
                    // Save to history before executing
                    if HISTORY_INITIALIZED {
                        let mut history = get_history().lock();
                        history.add(cmd_str);
                    }

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
        "mana-flow" => cmd_mana_flow(),
        "soothe" => cmd_soothe(args),
        "release" => cmd_release(args),
        "observe-weave" => cmd_observe_weave(),
        "weave-new" => cmd_weave_new(args),
        "rest" => cmd_rest(args),
        "echo" => cmd_echo(args),
        "clear" => cmd_clear(),
        "help" => cmd_help(),
        "preempt" => cmd_preempt(args),
        "uptime" => cmd_uptime(),
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
    crate::println!("◈ Eldarin Shell - The Voice of Symbiotic Communion");
    crate::println!();
    crate::println!("System Observation:");
    crate::println!("  harmony            - Display system harmony and thread statistics");
    crate::println!("  mana-flow          - Visualize memory (Mana Pool) usage");
    crate::println!("  observe-weave      - Real-time view of the Loom's activity");
    crate::println!("  uptime             - Show how long the realm has been awake");
    crate::println!();
    crate::println!("Thread Management:");
    crate::println!("  weave-new [name]   - Spawn a new thread into the Loom");
    crate::println!("  soothe [id]        - Lower a thread's priority (more harmonious)");
    crate::println!("  release [id]       - Gracefully release a thread's resources");
    crate::println!("  rest [ms]          - Rest for a duration (sleep)");
    crate::println!();
    crate::println!("System Control:");
    crate::println!("  preempt [cmd]      - Control preemptive multitasking");
    crate::println!("                       'status'  - Show preemption state");
    crate::println!("                       'enable'  - Enable (100ms quantum)");
    crate::println!("                       'disable' - Return to cooperative mode");
    crate::println!("  clear              - Clear the screen");
    crate::println!();
    crate::println!("Utilities:");
    crate::println!("  echo [text]        - Repeat text (test the shell)");
    crate::println!("  help               - Show this message");
    crate::println!();
    crate::println!("The shell listens to your intentions and translates them into action.");
}

/// The Preempt Spell - Control preemptive multitasking
fn cmd_preempt(args: &str) {
    match args.trim() {
        "status" | "" => {
            // Show current status
            crate::println!("◈ Preemption Status");
            crate::println!();

            let enabled = crate::loom_of_fate::is_preemption_enabled();
            let quantum = crate::loom_of_fate::get_time_quantum();

            crate::println!("  Mode: {}", if enabled { "PREEMPTIVE" } else { "COOPERATIVE" });
            crate::println!("  Time Quantum: {}ms", quantum);
            crate::println!();

            if enabled {
                crate::println!("  ⚠ Preemption is ENABLED");
                crate::println!("  Threads will be interrupted after {}ms", quantum);
                crate::println!("  Note: Timer interrupt integration not yet active");
            } else {
                crate::println!("  ✓ Cooperative mode (threads yield voluntarily)");
            }
        }
        "enable" => {
            crate::println!("◈ Enabling preemptive multitasking...");
            crate::loom_of_fate::enable_preemption(100); // 100ms quantum (conservative)
            crate::println!("  ✓ Preemption enabled with 100ms quantum");
            crate::println!("  ⚠ ACTIVE: Timer will now trigger context switches!");
            crate::println!("  Use 'preempt disable' if system becomes unstable");
        }
        "disable" => {
            crate::println!("◈ Disabling preemptive multitasking...");
            crate::loom_of_fate::disable_preemption();
            crate::println!("  ✓ Returned to cooperative mode");
        }
        _ => {
            crate::println!("Unknown preempt command: '{}'", args);
            crate::println!("Usage: preempt [status|enable|disable]");
        }
    }
}

/// The Mana-Flow Spell - Visualize memory usage with elegant bars
fn cmd_mana_flow() {
    crate::println!("◈ Mana Pool - The Flow of Essence");
    crate::println!();

    let stats = crate::mana_pool::stats();

    let sanctuary_total_kb = stats.sanctuary_total / 1024;
    let sanctuary_used_kb = stats.sanctuary_used / 1024;
    let sanctuary_free_kb = (stats.sanctuary_total - stats.sanctuary_used) / 1024;

    let ephemeral_total_kb = stats.ephemeral_total / 1024;
    let ephemeral_used_kb = stats.ephemeral_used / 1024;
    let ephemeral_free_kb = (stats.ephemeral_total - stats.ephemeral_used) / 1024;

    let total_bytes = stats.sanctuary_total + stats.ephemeral_total;
    let used_bytes = stats.sanctuary_used + stats.ephemeral_used;
    let total_kb = total_bytes / 1024;
    let used_kb = used_bytes / 1024;
    let free_kb = (total_bytes - used_bytes) / 1024;

    let used_percent = if total_bytes > 0 {
        (used_bytes * 100) / total_bytes
    } else {
        0
    };

    // Draw a beautiful progress bar
    crate::println!("  Total Essence: {} KB", total_kb);
    crate::println!();
    crate::print!("  [");

    let bar_width = 50;
    let filled = (used_percent as usize * bar_width) / 100;
    for i in 0..bar_width {
        if i < filled {
            crate::print!("█");
        } else {
            crate::print!("░");
        }
    }
    crate::println!("] {}%", used_percent);
    crate::println!();

    crate::println!("  Sustenance (Used):     {} KB", used_kb);
    crate::println!("  Free Flow (Available): {} KB", free_kb);
    crate::println!();

    crate::println!("  Sanctuary (Long-lived):");
    crate::println!("    Used: {} KB / {} KB", sanctuary_used_kb, sanctuary_total_kb);
    crate::println!();

    crate::println!("  Ephemeral Mist (Short-lived):");
    crate::println!("    Used: {} KB / {} KB", ephemeral_used_kb, ephemeral_total_kb);
    crate::println!();

    crate::println!("  Total Objects: {}", stats.total_objects);

    if used_bytes == 0 {
        crate::println!();
        crate::println!("  Status: ◈ The Mana Pool flows freely, untouched");
    } else if used_percent < 50 {
        crate::println!();
        crate::println!("  Status: ✓ Abundant mana available");
    } else if used_percent < 80 {
        crate::println!();
        crate::println!("  Status: ◈ Mana flow is balanced");
    } else {
        crate::println!();
        crate::println!("  Status: ⚠ Mana reserves running low");
    }
}

/// The Uptime Spell - Show how long the system has been running
fn cmd_uptime() {
    let ticks = crate::attunement::timer::ticks();
    let seconds = ticks / 1000;  // Assuming 1ms ticks
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    crate::println!("◈ Time Since Awakening");
    crate::println!();

    if days > 0 {
        crate::println!("  {} days, {} hours, {} minutes, {} seconds",
            days, hours % 24, minutes % 60, seconds % 60);
    } else if hours > 0 {
        crate::println!("  {} hours, {} minutes, {} seconds",
            hours, minutes % 60, seconds % 60);
    } else if minutes > 0 {
        crate::println!("  {} minutes, {} seconds", minutes, seconds % 60);
    } else {
        crate::println!("  {} seconds", seconds);
    }

    crate::println!("  ({} timer ticks)", ticks);
}

/// The Soothe Spell - Lower a thread's priority
fn cmd_soothe(args: &str) {
    crate::println!("◈ Soothing a Thread");
    crate::println!();
    crate::println!("  Feature not yet implemented.");
    crate::println!("  This will lower the priority of thread: {}", args);
    crate::println!();
    crate::println!("  \"Be gentle, for every thread serves the realm.\"");
}

/// The Release Spell - Gracefully terminate a thread
fn cmd_release(args: &str) {
    crate::println!("◈ Releasing a Thread");
    crate::println!();
    crate::println!("  Feature not yet implemented.");
    crate::println!("  This will gracefully release thread: {}", args);
    crate::println!();
    crate::println!("  \"A thread's resources return to the Mana Pool,\"");
    crate::println!("  \"its fate gently unwoven from the Loom.\"");
}

/// The Observe-Weave Spell - Real-time system monitor
fn cmd_observe_weave() {
    crate::println!("◈ Observing the Loom's Weave");
    crate::println!();
    crate::println!("  Feature not yet fully implemented.");
    crate::println!();
    crate::println!("  This command will show a real-time, continuously");
    crate::println!("  updating view of the Loom's activity:");
    crate::println!();
    crate::println!("  • Threads and their states (Weaving/Resting)");
    crate::println!("  • Harmony metrics in real-time");
    crate::println!("  • Memory flow visualization");
    crate::println!("  • Context switch rate");
    crate::println!();
    crate::println!("  Press 'q' to return to the shell (future feature)");
    crate::println!();
    crate::println!("  \"Watch the threads dance in their eternal weave...\"");
}

/// The Weave-New Spell - Spawn a new thread
fn cmd_weave_new(args: &str) {
    crate::println!("◈ Weaving a New Thread");
    crate::println!();
    crate::println!("  Feature not yet implemented.");
    crate::println!("  This will create a new thread named: {}", args);
    crate::println!();
    crate::println!("  \"A new thread shall join the grand tapestry.\"");
}

/// The Rest Spell - Sleep for a duration
fn cmd_rest(args: &str) {
    let ms: u64 = args.trim().parse().unwrap_or(1000);

    crate::println!("◈ Entering a State of Rest");
    crate::println!();
    crate::println!("  Resting for {} milliseconds...", ms);

    // Simple busy-wait (in a real implementation, this would yield to other threads)
    let start = crate::attunement::timer::ticks();
    while crate::attunement::timer::ticks() - start < ms {
        // Busy wait
    }

    crate::println!("  ✓ Awakened");
}
