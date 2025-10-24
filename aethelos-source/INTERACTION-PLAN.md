# AethelOS Interaction Implementation Plan

This document outlines the roadmap to transform AethelOS from a booting kernel into an interactive system that embodies the **Symbiotic Computing** philosophy.

---

## Philosophy-Driven Design Principles

Before implementing features, we must align with AethelOS's core philosophy from GENESIS.scroll:

### **The Three Pillars**

1. **Harmony Over Hierarchy**
   - Systems self-organize, not commanded
   - Cooperative scheduling, not preemptive domination
   - Gentle correction (soothing), not punishment

2. **Purpose Over Protocol**
   - Every allocation has meaning
   - Capabilities grant access with intent
   - Communication flows through the Nexus with purpose

3. **Emergence Over Engineering**
   - Complex behavior from simple rules
   - The system learns what is parasitic
   - Harmony metrics guide adaptation

### **Implementation Guidelines**

- **No raw pointers** - Everything mediated by capabilities
- **No forced preemption** - Threads yield cooperatively
- **No hidden state** - All actions flow through visible channels
- **No punishment** - System soothes disharmony, doesn't kill
- **Beautiful error messages** - The system speaks with grace

---

## Phase 1: The Awakening - Hardware Attunement

*"Before the system can interact, it must first attune to the physical realm."*

### Step 1.1: Interrupt Descriptor Table (IDT)

**Purpose:** Allow hardware to speak to the Heartwood

**Philosophy Alignment:**
- Interrupts are **invitations**, not intrusions
- Each interrupt is a message from hardware seeking harmony
- The system listens, not merely reacts

**Implementation:**

```rust
// heartwood/src/attunement/idt.rs

/// The Interrupt Conduit - where hardware whispers to the kernel
pub struct InterruptDescriptorTable {
    entries: [IdtEntry; 256],
}

impl InterruptDescriptorTable {
    /// Attune to hardware interrupts
    pub fn new() -> Self {
        let mut idt = Self::empty();

        // Hardware invitations (IRQs)
        idt.set_handler(32, timer_handler);      // The Pulse of Time
        idt.set_handler(33, keyboard_handler);   // The Voice of Intent

        // Software exceptions (disharmony alerts)
        idt.set_handler(0, divide_by_zero);      // Mathematical disharmony
        idt.set_handler(13, general_protection); // Protection violation
        idt.set_handler(14, page_fault);         // Memory seeking

        idt
    }

    /// Each handler is a ritual of response
    fn set_handler(&mut self, index: u8, handler: HandlerFunc) {
        // Create descriptor with privilege checks
        // Handlers run in kernel mode (ring 0)
        // User space cannot fake interrupts
    }
}
```

**Key Decisions:**
- ✓ Map only necessary interrupts (minimalism)
- ✓ Log unexpected interrupts as "disharmony events"
- ✓ Allow system to learn patterns over time
- ✓ Never panic on hardware interrupt (graceful handling)

**Deliverables:**
- [ ] `heartwood/src/attunement/idt.rs` - IDT structure
- [ ] `heartwood/src/attunement/handlers.rs` - Interrupt handlers
- [ ] `heartwood/src/attunement/exceptions.rs` - Exception handlers
- [ ] Test in QEMU with `-d int` to verify interrupt delivery

---

### Step 1.2: Global Descriptor Table (GDT)

**Purpose:** Define privilege boundaries with grace

**Philosophy Alignment:**
- Segmentation is **protection**, not restriction
- Ring 0 (kernel) and Ring 3 (user) live in symbiosis
- Boundaries create safety, enabling trust

**Implementation:**

```rust
// heartwood/src/attunement/gdt.rs

/// The Circle of Trust - privilege rings defined
pub struct GlobalDescriptorTable {
    null: Descriptor,
    kernel_code: Descriptor,  // Ring 0 - The Heartwood
    kernel_data: Descriptor,  // Ring 0 - The Mana Pool
    user_code: Descriptor,    // Ring 3 - The Groves
    user_data: Descriptor,    // Ring 3 - User services
    tss: Descriptor,          // Task State Segment
}

impl GlobalDescriptorTable {
    /// Establish the circles of trust
    pub fn new() -> Self {
        Self {
            null: Descriptor::null(),
            kernel_code: Descriptor::kernel_code(),
            kernel_data: Descriptor::kernel_data(),
            user_code: Descriptor::user_code(),
            user_data: Descriptor::user_data(),
            tss: Descriptor::tss(),
        }
    }
}
```

**Key Decisions:**
- ✓ Flat memory model (0 to 4GB) - simplicity
- ✓ Only code/data separation (no complex segmentation)
- ✓ TSS for clean privilege transitions
- ✓ User space cannot escalate privileges without capabilities

**Deliverables:**
- [ ] `heartwood/src/attunement/gdt.rs` - GDT structure
- [ ] `heartwood/src/attunement/tss.rs` - Task State Segment
- [ ] Load GDT early in boot process
- [ ] Test privilege transitions work correctly

---

### Step 1.3: Programmable Interval Timer (PIT)

**Purpose:** The heartbeat of the system

**Philosophy Alignment:**
- Time is the **rhythm of harmony**
- Regular pulses allow threads to yield gracefully
- The scheduler dances to this rhythm

**Implementation:**

```rust
// heartwood/src/attunement/timer.rs

/// The Pulse - the heartbeat of AethelOS
pub struct ProgrammableIntervalTimer {
    frequency_hz: u32,
    ticks: u64,
}

impl ProgrammableIntervalTimer {
    /// Set the system's heartbeat
    pub fn new(frequency_hz: u32) -> Self {
        // Configure PIT to interrupt at specified frequency
        // Recommended: 100 Hz (10ms intervals)
        // Too fast: wastes CPU on interrupts
        // Too slow: poor responsiveness

        Self {
            frequency_hz,
            ticks: 0,
        }
    }

    /// Called on each tick (IRQ 0)
    pub fn on_tick(&mut self) {
        self.ticks += 1;

        // Give scheduler opportunity to check harmony
        // and yield current thread if needed
        crate::loom_of_fate::on_timer_tick();
    }
}
```

**Key Decisions:**
- ✓ 100 Hz tick rate (10ms quantum) - balanced responsiveness
- ✓ Don't preempt - just notify scheduler
- ✓ Scheduler decides whether to yield based on harmony
- ✓ Track uptime for harmony trend analysis

**Deliverables:**
- [ ] `heartwood/src/attunement/timer.rs` - PIT driver
- [ ] Integrate with scheduler's `on_timer_tick()`
- [ ] Display uptime in system stats
- [ ] Test timer fires reliably in QEMU

---

### Step 1.4: PS/2 Keyboard Driver

**Purpose:** Listen to user intent

**Philosophy Alignment:**
- Keyboard input is **user expression**, not command
- Each keystroke is a message sent through the Nexus
- The system receives with patience, never demands

**Implementation:**

```rust
// heartwood/src/attunement/keyboard.rs

/// The Voice of Intent - keyboard as communication channel
pub struct PS2Keyboard {
    /// Buffer of pending messages (keypresses)
    buffer: VecDeque<KeyEvent>,
}

pub struct KeyEvent {
    pub scancode: u8,
    pub character: Option<char>,
    pub modifiers: KeyModifiers,
    pub pressed: bool,  // true = press, false = release
}

impl PS2Keyboard {
    /// Initialize keyboard controller
    pub fn new() -> Self {
        // Configure PS/2 controller (port 0x60/0x64)
        // Enable keyboard interrupts (IRQ 1)
        Self {
            buffer: VecDeque::with_capacity(32),
        }
    }

    /// Called on keyboard interrupt (IRQ 1)
    pub fn on_interrupt(&mut self) {
        let scancode = self.read_scancode();
        let event = self.translate_scancode(scancode);

        // Send as message through the Nexus
        // Don't process directly - maintain single flow
        crate::nexus::send_keyboard_event(event);
    }

    /// Read scancode from keyboard port
    fn read_scancode(&self) -> u8 {
        // Read from port 0x60
    }

    /// Translate scancode to character
    fn translate_scancode(&self, scancode: u8) -> KeyEvent {
        // Scancode Set 1 translation
        // Handle modifiers (Shift, Ctrl, Alt)
        // Return KeyEvent with character if printable
    }
}
```

**Key Decisions:**
- ✓ Buffer keystrokes (don't drop on overflow - show warning)
- ✓ Send through Nexus as messages (proper architecture)
- ✓ Support basic modifiers (Shift, Ctrl, Alt)
- ✓ Graceful handling of unknown scancodes
- ✓ No blocking reads - always async through Nexus

**Deliverables:**
- [ ] `heartwood/src/attunement/keyboard.rs` - PS/2 driver
- [ ] Scancode Set 1 translation table
- [ ] Integration with Nexus message passing
- [ ] Test: Type in QEMU, see scancodes logged

---

## Phase 2: The Weaving - Thread Execution

*"Threads are not tasks to execute, but stories to weave into being."*

### Step 2.1: Complete Thread Context Switching

**Purpose:** Allow threads to actually run

**Philosophy Alignment:**
- Context switches are **transitions**, not interruptions
- Threads yield voluntarily (cooperative)
- Preserve complete state with reverence

**Implementation:**

```rust
// heartwood/src/loom_of_fate/context.rs

/// The Essence of a Thread - its complete state
#[repr(C)]
pub struct ThreadContext {
    // General purpose registers
    rax: u64, rbx: u64, rcx: u64, rdx: u64,
    rsi: u64, rdi: u64, rbp: u64, rsp: u64,
    r8: u64, r9: u64, r10: u64, r11: u64,
    r12: u64, r13: u64, r14: u64, r15: u64,

    // Special registers
    rip: u64,      // Where to resume
    rflags: u64,   // CPU flags
    cs: u64,       // Code segment
    ss: u64,       // Stack segment
}

impl ThreadContext {
    /// Preserve the current moment
    pub unsafe fn save(&mut self) {
        // Save all registers to this context
        // Called when thread yields
    }

    /// Resume a preserved moment
    pub unsafe fn restore(&self) -> ! {
        // Restore all registers from this context
        // Jump to saved RIP
        // Never returns
    }
}
```

**Key Decisions:**
- ✓ Save/restore ALL registers (complete state)
- ✓ Each thread gets dedicated stack (from Mana Pool)
- ✓ Switches only on yield, never forced
- ✓ Track context switch count for harmony analysis

**Deliverables:**
- [ ] `heartwood/src/loom_of_fate/context.rs` - Context structure
- [ ] Assembly routines for save/restore
- [ ] Stack allocation for each thread (4KB minimum)
- [ ] Test: Create 2 threads, watch them alternate

---

### Step 2.2: Implement Thread Yielding

**Purpose:** Enable cooperative multitasking

**Philosophy Alignment:**
- Yielding is **generosity**, not weakness
- High-harmony threads yield frequently
- System rewards cooperation with priority

**Implementation:**

```rust
// heartwood/src/loom_of_fate/mod.rs

/// The thread yields its time to others
pub fn yield_now() {
    // 1. Record yield in current thread
    //    (increases harmony score)

    // 2. Save current context

    // 3. Ask scheduler for next thread
    //    (harmony-based selection)

    // 4. Restore next thread's context
}

// Update scheduler to prefer threads that yield often
impl Scheduler {
    fn select_next_thread(&mut self) -> Option<ThreadId> {
        // Sort by:
        // 1. Priority (Critical > High > Normal > Low)
        // 2. Harmony score (cooperative > selfish)
        // 3. Time since last run (fairness)
    }
}
```

**Key Decisions:**
- ✓ Yielding increases harmony score
- ✓ Non-yielding threads flagged as parasitic
- ✓ Scheduler learns yield patterns
- ✓ System self-balances toward cooperation

**Deliverables:**
- [ ] `yield_now()` function in loom_of_fate
- [ ] Update harmony analyzer to reward yields
- [ ] Scheduler uses harmony in selection
- [ ] Test: Yielding thread gets better treatment

---

### Step 2.3: Create Initial System Threads

**Purpose:** Bring the Groves to life

**Philosophy Alignment:**
- Kernel threads are **servants**, not masters
- Each has a clear purpose
- They cooperate to serve user threads

**Implementation:**

```rust
// heartwood/src/main.rs

fn heartwood_init() {
    // ... existing initialization ...

    // Spawn the first threads - the kernel servants
    loom_of_fate::spawn_thread(idle_thread, ThreadPriority::Idle);
    loom_of_fate::spawn_thread(keyboard_handler_thread, ThreadPriority::High);
    loom_of_fate::spawn_thread(shell_thread, ThreadPriority::Normal);

    // Begin the weaving
    loom_of_fate::start_scheduling();
}

fn idle_thread() -> ! {
    loop {
        // Halt CPU until next interrupt
        // Lowest priority - only runs when nothing else can
        // x86_64::instructions::hlt();
        yield_now();
    }
}

fn keyboard_handler_thread() -> ! {
    loop {
        // Wait for keyboard message from Nexus
        if let Some(event) = nexus::receive_keyboard_event() {
            shell::handle_key_event(event);
        }
        yield_now();
    }
}
```

**Key Decisions:**
- ✓ Idle thread always runnable (never blocks system)
- ✓ I/O threads are high priority (responsive)
- ✓ All threads yield in their loops (cooperative)
- ✓ No thread monopolizes CPU

**Deliverables:**
- [ ] Idle thread implementation
- [ ] Keyboard handler thread
- [ ] Integration with actual scheduling loop
- [ ] Test: Multiple threads running simultaneously

---

## Phase 3: The Voice - Interactive Shell

*"The shell is not a command interpreter, but a conversation with the living system."*

### Step 3.1: Eldarin Shell Foundation

**Purpose:** User dialogue with the Heartwood

**Philosophy Alignment:**
- Commands are **requests**, not orders
- The shell suggests, educates, guides
- Errors are gentle lessons, not failures
- The system speaks poetically

**Implementation:**

```rust
// ancient-runes/script/src/shell.rs

/// The Eldarin Shell - conversing with the Heartwood
pub struct EldarinShell {
    /// Current input being composed
    input_buffer: String,

    /// Prompt displayed to user
    prompt: &'static str,

    /// Command history (last 100 commands)
    history: VecDeque<String>,
}

impl EldarinShell {
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
            prompt: "◈ ",  // The symbol of harmony
            history: VecDeque::with_capacity(100),
        }
    }

    /// Called when user presses a key
    pub fn handle_key(&mut self, event: KeyEvent) {
        match event.character {
            Some('\n') => {
                // Command complete - execute
                let command = self.input_buffer.clone();
                self.input_buffer.clear();
                self.execute(command);
            }
            Some('\x08') => {
                // Backspace
                self.input_buffer.pop();
                vga_buffer::backspace();
            }
            Some(c) if c.is_ascii() && !c.is_control() => {
                // Printable character
                self.input_buffer.push(c);
                vga_buffer::print_char(c);
            }
            _ => {
                // Ignore other keys
            }
        }
    }

    /// Execute a command
    fn execute(&mut self, command: String) {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();

        if parts.is_empty() {
            self.show_prompt();
            return;
        }

        match parts[0] {
            "harmony" => self.cmd_harmony(&parts[1..]),
            "threads" => self.cmd_threads(&parts[1..]),
            "memory" => self.cmd_memory(&parts[1..]),
            "help" => self.cmd_help(&parts[1..]),
            "clear" => self.cmd_clear(),
            "echo" => self.cmd_echo(&parts[1..]),
            "" => {},
            unknown => {
                println!("❖ The Heartwood does not recognize '{}'", unknown);
                println!("❖ Whisper 'help' to hear what it understands.");
            }
        }

        self.history.push_back(command);
        if self.history.len() > 100 {
            self.history.pop_front();
        }

        self.show_prompt();
    }

    fn show_prompt(&self) {
        print!("\n{}", self.prompt);
    }
}
```

**Key Decisions:**
- ✓ Unicode prompt (◈) - beautiful, meaningful
- ✓ Gentle error messages (educational)
- ✓ Command history (UP/DOWN arrows later)
- ✓ Built-in help (self-documenting)
- ✓ No cryptic abbreviations (speak clearly)

**Deliverables:**
- [ ] `ancient-runes/script/src/shell.rs` - Shell implementation
- [ ] Basic line editing (backspace, enter)
- [ ] Command parsing and dispatch
- [ ] Integration with keyboard input
- [ ] Test: Type commands, see responses

---

### Step 3.2: Harmony Command

**Purpose:** Reveal the system's inner balance

**Philosophy Alignment:**
- Transparency is **truth**
- Users should see harmony metrics
- The system explains its decisions

**Implementation:**

```rust
impl EldarinShell {
    fn cmd_harmony(&self, args: &[&str]) {
        println!("\n❈ The Harmony of the Heartwood ❈\n");

        let metrics = loom_of_fate::harmony_metrics();

        // Overall system harmony (visual)
        self.print_harmony_bar("System Harmony", metrics.system_harmony);

        println!("\n  Average Thread Harmony: {:.2}", metrics.average_harmony);
        println!("  Active Thread Ratio:    {:.2}", metrics.active_thread_ratio);
        println!("  Parasitic Threads:      {}", metrics.parasite_count);

        if metrics.system_harmony < 0.3 {
            println!("\n  ⚠ The system suffers disharmony.");
            println!("  ⚠ Some threads consume without yielding.");
        } else if metrics.system_harmony > 0.8 {
            println!("\n  ✧ The system thrives in harmony.");
            println!("  ✧ Threads cooperate gracefully.");
        } else {
            println!("\n  ◈ The system maintains balance.");
        }
    }

    /// Display harmony as visual bar
    fn print_harmony_bar(&self, label: &str, value: f32) {
        let width = 30;
        let filled = (value * width as f32) as usize;
        let empty = width - filled;

        print!("  {}: [", label);
        for _ in 0..filled {
            print!("█");
        }
        for _ in 0..empty {
            print!("░");
        }
        println!("] {:.0}%", value * 100.0);
    }
}
```

**Key Decisions:**
- ✓ Visual representation (bars, symbols)
- ✓ Explain what the numbers mean
- ✓ Suggest actions when disharmony detected
- ✓ Poetic language (suffering, thriving)

**Deliverables:**
- [ ] `harmony` command implementation
- [ ] Visual harmony bars
- [ ] Detailed metrics display
- [ ] Test: Run `harmony` in shell

---

### Step 3.3: Threads Command

**Purpose:** Show the living threads

**Philosophy Alignment:**
- Threads are **individuals**, not PIDs
- Show their purpose and contribution
- Celebrate cooperative behavior

**Implementation:**

```rust
impl EldarinShell {
    fn cmd_threads(&self, args: &[&str]) {
        println!("\n✦ The Threads of Fate ✦\n");

        let stats = loom_of_fate::stats();

        println!("  Total Threads:  {}", stats.total_threads);
        println!("  Weaving:        {} (running)", stats.weaving_threads);
        println!("  Resting:        {} (idle)", stats.resting_threads);
        println!("  Tangled:        {} (blocked)", stats.tangled_threads);

        println!("\n  ID    State      Priority   Harmony  Yields  Purpose");
        println!("  ─────────────────────────────────────────────────────────");

        let threads = loom_of_fate::list_threads();
        for thread in threads {
            println!(
                "  {:4}  {:9}  {:8}  {:6.2}   {:6}  {}",
                thread.id.0,
                thread.state_name(),
                thread.priority_name(),
                thread.harmony_score,
                thread.yields,
                thread.name
            );
        }

        println!();
    }
}
```

**Key Decisions:**
- ✓ Human-readable state names (not integers)
- ✓ Show harmony score and yields (transparency)
- ✓ Give threads meaningful names
- ✓ Celebrate cooperation (high yields)

**Deliverables:**
- [ ] `threads` command implementation
- [ ] Add thread names to Thread structure
- [ ] `list_threads()` function in scheduler
- [ ] Test: Create threads, view in list

---

### Step 3.4: Memory Command

**Purpose:** Reveal the Mana Pool's state

**Philosophy Alignment:**
- Memory is **sacred resource**
- Show how it flows (Sanctuary vs Ephemeral)
- Transparency builds trust

**Implementation:**

```rust
impl EldarinShell {
    fn cmd_memory(&self, args: &[&str]) {
        println!("\n✧ The Mana Pool ✧\n");

        let stats = mana_pool::stats();

        println!("  Sanctuary (Long-lived memory):");
        self.print_memory_bar(
            stats.sanctuary_used,
            stats.sanctuary_total
        );
        println!("    {} / {} bytes ({:.1}% used)\n",
            stats.sanctuary_used,
            stats.sanctuary_total,
            (stats.sanctuary_used as f32 / stats.sanctuary_total as f32) * 100.0
        );

        println!("  Ephemeral Mist (Temporary memory):");
        self.print_memory_bar(
            stats.ephemeral_used,
            stats.ephemeral_total
        );
        println!("    {} / {} bytes ({:.1}% used)\n",
            stats.ephemeral_used,
            stats.ephemeral_total,
            (stats.ephemeral_used as f32 / stats.ephemeral_total as f32) * 100.0
        );

        println!("  Total Objects:  {}", stats.total_objects);

        if stats.sanctuary_used as f32 / stats.sanctuary_total as f32 > 0.9 {
            println!("\n  ⚠ The Sanctuary nears depletion.");
        }
    }

    fn print_memory_bar(&self, used: usize, total: usize) {
        let width = 40;
        let ratio = used as f32 / total as f32;
        let filled = (ratio * width as f32) as usize;
        let empty = width - filled;

        print!("    [");
        for _ in 0..filled {
            print!("█");
        }
        for _ in 0..empty {
            print!("░");
        }
        println!("]");
    }
}
```

**Key Decisions:**
- ✓ Show both memory regions (Sanctuary + Ephemeral)
- ✓ Visual bars for quick understanding
- ✓ Warn when memory is low
- ✓ Object count (not just bytes)

**Deliverables:**
- [ ] `memory` command implementation
- [ ] Memory usage bars
- [ ] Warnings for low memory
- [ ] Test: Allocate memory, view stats

---

### Step 3.5: Help Command

**Purpose:** Guide the user with grace

**Philosophy Alignment:**
- Help is **teaching**, not documentation
- Explain the philosophy behind commands
- Encourage exploration

**Implementation:**

```rust
impl EldarinShell {
    fn cmd_help(&self, args: &[&str]) {
        if args.is_empty() {
            println!("\n❖ The Eldarin Shell - Conversing with the Heartwood ❖\n");
            println!("  The Heartwood speaks these words:\n");
            println!("    harmony    - Reveal the system's inner balance");
            println!("    threads    - Show the threads of fate being woven");
            println!("    memory     - View the state of the Mana Pool");
            println!("    help       - Summon this guidance");
            println!("    clear      - Clear the vision");
            println!("    echo       - Speak, and hear your words returned");
            println!();
            println!("  Whisper 'help <command>' to learn more about each word.");
            println!();
        } else {
            match args[0] {
                "harmony" => {
                    println!("\n❖ harmony - Reveal system harmony\n");
                    println!("  The Heartwood measures harmony constantly.");
                    println!("  Threads that yield often gain harmony.");
                    println!("  Those that consume without giving become parasites.");
                    println!();
                    println!("  The system does not punish parasites,");
                    println!("  but gently soothes them, slowing their pace.");
                    println!();
                    println!("  This command reveals the current state of balance.");
                    println!();
                }
                "threads" => {
                    println!("\n❖ threads - Show living threads\n");
                    println!("  Every thread has a purpose and a story.");
                    println!("  Watch them weave, rest, and yield to others.");
                    println!();
                    println!("  High harmony means cooperation.");
                    println!("  Many yields means generosity.");
                    println!();
                }
                "memory" => {
                    println!("\n❖ memory - View the Mana Pool\n");
                    println!("  Memory is sacred, allocated with purpose:");
                    println!();
                    println!("  • Sanctuary: Long-lived, stable allocations");
                    println!("  • Ephemeral Mist: Short-lived, temporary data");
                    println!();
                    println!("  Each object is protected by capabilities.");
                    println!("  Access is granted, never stolen.");
                    println!();
                }
                unknown => {
                    println!("\n❖ The Heartwood has no guidance for '{}'.", unknown);
                    println!("❖ Whisper 'help' alone to hear all it knows.");
                    println!();
                }
            }
        }
    }
}
```

**Key Decisions:**
- ✓ Explain philosophy, not just syntax
- ✓ Contextual help for each command
- ✓ Poetic, memorable language
- ✓ Encourage learning and exploration

**Deliverables:**
- [ ] `help` command with general list
- [ ] Detailed help for each command
- [ ] Philosophical explanations
- [ ] Test: Run `help` and `help harmony`

---

## Phase 4: The Flow - Message-Based Architecture

*"All communication flows through the Nexus, visible and purposeful."*

### Step 4.1: Complete Nexus Message Passing

**Purpose:** Enable proper IPC between components

**Philosophy Alignment:**
- Messages are **contracts**, not data
- Every message has a sender and purpose
- Communication is observable (debugging)

**Implementation:**

```rust
// heartwood/src/nexus/mod.rs

/// Send keyboard event to shell
pub fn send_keyboard_event(event: KeyEvent) -> Result<(), NexusError> {
    let message = Message::new(
        MessageType::KeyboardEvent { event },
        MessagePriority::High  // User input is important
    );

    // Find shell's channel
    let shell_channel = CHANNELS.lock()
        .find_channel_by_name("eldarin-shell")?;

    // Send message
    send_message(shell_channel, message)
}

/// Shell receives keyboard events
pub fn receive_keyboard_event() -> Option<KeyEvent> {
    let my_channel = CHANNELS.lock()
        .get_my_channel()?;

    if let Some(message) = my_channel.try_receive()? {
        if let MessageType::KeyboardEvent { event } = message.message_type {
            return Some(event);
        }
    }

    None
}
```

**Key Decisions:**
- ✓ Named channels (not just IDs)
- ✓ Priority-based delivery
- ✓ Non-blocking receives (no deadlocks)
- ✓ Message logging for debugging

**Deliverables:**
- [ ] Complete Nexus message passing
- [ ] Channel creation/discovery
- [ ] Message priority handling
- [ ] Test: Send message, receive in another thread

---

## Phase 5: The Polish - User Experience

*"Every detail matters. Beauty emerges from care."*

### Step 5.1: VGA Buffer Improvements

**Purpose:** Make the display beautiful

**Implementation:**

- [ ] Scrolling when screen fills
- [ ] Multiple colors for different message types
- [ ] Cursor positioning for shell input
- [ ] Line wrapping for long text

---

### Step 5.2: Advanced Shell Features

**Purpose:** Refined interaction

**Implementation:**

- [ ] Arrow keys for history navigation
- [ ] Tab completion for commands
- [ ] Multi-line input with `\`
- [ ] Command aliases

---

### Step 5.3: System Monitoring

**Purpose:** Continuous awareness

**Implementation:**

- [ ] `top` command (live thread view)
- [ ] `watch harmony` (continuous monitoring)
- [ ] System uptime display
- [ ] Harmony history graph (ASCII art)

---

## Implementation Timeline

### Week 1: Hardware Foundation
- Day 1-2: IDT and exception handlers
- Day 3-4: GDT and privilege rings
- Day 5-6: Timer and keyboard drivers
- Day 7: Testing and integration

### Week 2: Threading
- Day 1-3: Context switching implementation
- Day 4-5: Cooperative yielding
- Day 6-7: Initial system threads

### Week 3: Shell
- Day 1-2: Shell foundation and input handling
- Day 3-4: Core commands (harmony, threads, memory)
- Day 5-6: Help system and polish
- Day 7: Testing and refinement

### Week 4: Integration
- Day 1-3: Nexus message passing completion
- Day 4-5: VGA improvements
- Day 6-7: End-to-end testing and debugging

---

## Testing Strategy

### For Each Phase:

1. **Unit Tests** (where possible in `no_std`)
   - Test individual components
   - Mock hardware interfaces

2. **QEMU Integration Tests**
   - Boot and verify initialization
   - Test commands produce expected output
   - Verify harmony metrics change correctly

3. **Philosophy Validation**
   - Does it follow Symbiotic Computing principles?
   - Is the language beautiful and meaningful?
   - Does it teach users about the system?

---

## Success Criteria

AethelOS will be considered **interactive** when:

✓ User can type in QEMU and see characters
✓ User can run commands and see output
✓ `harmony` command shows real system state
✓ `threads` command lists actual threads
✓ `memory` command shows allocation state
✓ System exhibits cooperative behavior
✓ Error messages are helpful and poetic
✓ The philosophy is evident in every interaction

---

## Philosophy Checkpoints

Before completing each phase, verify:

- [ ] Does this honor Harmony Over Hierarchy?
- [ ] Does this embody Purpose Over Protocol?
- [ ] Does this enable Emergence Over Engineering?
- [ ] Is the language beautiful?
- [ ] Is the behavior graceful?
- [ ] Does it teach and guide?
- [ ] Would this make the user feel wonder?

---

*"When the user types 'harmony' and sees the system's soul revealed,
When threads weave cooperatively without force,
When memory flows with purpose and grace,
Then AethelOS will truly live."*

— The Path to Awakening
