# CLAUDE.md - AI Assistant Guide to AethelOS Development

> **Quick Reference:** This document is designed for AI assistants (Claude, GPT, etc.) working on AethelOS. It provides build commands, design philosophy, coding standards, and pointers to architectural documentation.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Design Philosophy](#design-philosophy)
3. [Architecture Overview](#architecture-overview)
4. [Coding Standards & Quality Controls](#coding-standards--quality-controls)
5. [Documentation Index](#documentation-index)
6. [Current Status & Roadmap](#current-status--roadmap)
7. [Common Tasks](#common-tasks)

---

## Quick Start

### Build Commands

**From project root (`aethelos-source/`):**

```bash
# 1. Build the kernel
cd heartwood
cargo build --target x86_64-aethelos.json

# 2. Create bootable ISO (requires WSL/Linux with GRUB)
cd ..
wsl bash -c "cp target/x86_64-aethelos/debug/heartwood isodir/boot/aethelos/heartwood.bin && grub-mkrescue -o aethelos.iso isodir"

# 3. Run in QEMU
"C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom aethelos.iso -serial file:serial.log -m 256M -display gtk -no-reboot -no-shutdown

# OR: Use the boot script (Windows)
BOOT_AETHELOS.bat
```

**Quick rebuild workflow (one-liner with error checking):**
```bash
cd /f/OS/aethelos-source/heartwood && cargo build --bin heartwood --target x86_64-aethelos.json 2>&1 | grep -E "(Finished|error)" && cd .. && wsl bash -c "cp target/x86_64-aethelos/debug/heartwood isodir/boot/aethelos/heartwood.bin && grub-mkrescue -o aethelos.iso isodir 2>&1 | grep success"
```

**Note:** This filters output to show only build status and ISO creation success, making it faster to spot issues.

### Prerequisites

- **Rust:** Nightly toolchain (configured in `rust-toolchain.toml`)
  - Components: `rust-src`, `llvm-tools-preview`
- **GRUB:** `grub-mkrescue` (via WSL on Windows, native on Linux)
- **QEMU:** `qemu-system-x86_64` for testing
- **Target:** Custom `x86_64-aethelos.json` target spec

---

## Design Philosophy

### Core Principle: Symbiotic Computing

> *"The code does not command the silicon. The silicon does not serve the code. They dance together, and in that dance, life emerges."*

AethelOS is **not** a clone of Unix, Linux, or Windows. It is a ground-up rethinking of operating system design based on these principles:

### 1. **Harmony Over Force**

- **NOT:** Preemptive scheduling that interrupts processes arbitrarily
- **BUT:** Cooperative negotiation where processes yield willingly
- **METAPHOR:** The Loom of Fate weaves threads together harmoniously

**Implementation:**
- Threads have harmony scores based on CPU usage, yield frequency, and cooperation
- "Parasitic" threads are throttled (slowed), not killed
- System-wide harmony metrics guide scheduling decisions

### 2. **Memory Over Forgetting**

- **NOT:** Files that are overwritten and lost
- **BUT:** Git-like versioning built into the filesystem
- **METAPHOR:** The World-Tree remembers all versions like tree rings

**Implementation:**
- Content-addressable storage (SHA-256)
- Global commit graph tracking all changes
- Query-based file discovery: `seek scrolls where essence is "Scroll" and creator is "Elara"`
- Intelligent pruning prevents unbounded growth

### 3. **Beauty as Necessity**

- **NOT:** Aesthetics as afterthought
- **BUT:** Visual design reveals system state intuitively
- **METAPHOR:** The system's appearance is its truth

**Implementation:**
- Poetic naming (Loom of Fate, Mana Pool, World-Tree)
- Unicode symbols (◈ for emphasis, tree metaphors)
- VGA graphics mode planned for vector-based GUI
- Color-coded thread states, visual harmony indicators

### 4. **Security Through Nature**

- **NOT:** Access control lists and permissions bits
- **BUT:** Unforgeable capability tokens
- **METAPHOR:** Natural boundaries, not artificial walls

**Implementation:**
- Capability-based security (no raw pointers in userspace)
- Capabilities grant specific rights (read, write, execute, delegate)
- Capabilities can be attenuated (reduced permissions) but never amplified
- Hardware MMU enforces capability boundaries

---

## Architecture Overview

### High-Level Structure

```
┌─────────────────────────────────────────────────────────┐
│                    User Space                           │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ Eldarin     │  │ User Apps    │  │ System Utils  │  │
│  │ Shell       │  │ (Future)     │  │               │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────┘
                          ↕ (Capability-based syscalls)
┌─────────────────────────────────────────────────────────┐
│                    Groves (Services)                     │
│  ┌──────────────┐ ┌──────────────┐ ┌────────────────┐  │
│  │ World-Tree   │ │ The Weave    │ │ Network Sprite │  │
│  │ (Filesystem) │ │ (Compositor) │ │ (Networking)   │  │
│  └──────────────┘ └──────────────┘ └────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          ↕ (IPC via Nexus)
┌─────────────────────────────────────────────────────────┐
│              Heartwood (Kernel)                         │
│  ┌──────────────┐ ┌──────────────┐ ┌────────────────┐  │
│  │ Loom of Fate │ │ Mana Pool    │ │ Attunement     │  │
│  │ (Scheduler)  │ │ (Memory)     │ │ (Hardware)     │  │
│  └──────────────┘ └──────────────┘ └────────────────┘  │
│  ┌──────────────┐ ┌──────────────┐                     │
│  │ Nexus        │ │ VGA Buffer   │                     │
│  │ (IPC)        │ │ (Display)    │                     │
│  └──────────────┘ └──────────────┘                     │
└─────────────────────────────────────────────────────────┘
                          ↕
┌─────────────────────────────────────────────────────────┐
│                    Hardware                              │
│  CPU (x86-64) | Memory | VGA | Keyboard | Timer | etc.  │
└─────────────────────────────────────────────────────────┘
```

### Heartwood (Kernel) Components

| Module | Purpose | Status | Location |
|--------|---------|--------|----------|
| **Loom of Fate** | Cooperative thread scheduler with harmony metrics | ✅ Complete | `heartwood/src/loom_of_fate/` |
| **Mana Pool** | Memory allocator (currently buddy allocator) | ✅ Complete | `heartwood/src/mana_pool/` |
| **Attunement** | Hardware abstraction (IDT, PIC, keyboard, timer) | ✅ Complete | `heartwood/src/attunement/` |
| **VGA Buffer** | Text mode display driver | ✅ Complete | `heartwood/src/vga_buffer.rs` |
| **Eldarin** | Interactive shell | ✅ Complete | `heartwood/src/eldarin.rs` |
| **Nexus** | Inter-process communication | ⚪ Planned | Not started |

### Groves (Service Layer)

| Service | Purpose | Status | Location |
|---------|---------|--------|----------|
| **World-Tree** | Query-based filesystem with versioning | 🟡 Designed | `groves/world-tree_grove/` |
| **The Weave** | Vector scene graph compositor | 🟡 Designed | `groves/the-weave_grove/` |
| **Lanthir** | Window manager | 🟡 Designed | `groves/lanthir_grove/` |
| **Network Sprite** | Network stack | 🟡 Designed | `groves/network_sprite/` |

### Ancient Runes (Standard Library)

| Library | Purpose | Status | Location |
|---------|---------|--------|----------|
| **Corelib** | Collections, math, error types | 🟡 Skeletal | `ancient-runes/corelib/` |
| **Weaving** | Widget toolkit (buttons, labels, containers) | 🟡 Skeletal | `ancient-runes/weaving/` |
| **Script** | Shell scripting API for Glimmer-Weave | 🟡 Skeletal | `ancient-runes/script/` |

**Status Legend:**
- ✅ **Complete:** Working and integrated
- 🟡 **Designed:** Architecture documented, skeleton code exists
- ⚪ **Planned:** Design phase

---

## Coding Standards & Quality Controls

### 🔴 CRITICAL RULES - MUST FOLLOW

#### 1. **No Unsafe Without Justification**

```rust
// ❌ WRONG - Unjustified unsafe
unsafe {
    core::ptr::write_volatile(addr, value);
}

// ✅ CORRECT - Documented safety invariant
/// SAFETY: VGA buffer at 0xb8000 is a valid MMIO region
/// guaranteed by PC architecture. No other code writes here.
unsafe {
    core::ptr::write_volatile(addr, value);
}
```

**Rule:** Every `unsafe` block MUST have a `/// SAFETY:` comment explaining why it's safe.

#### 2. **Interrupt Safety for Shared State**

```rust
// ❌ WRONG - Spinlock can deadlock with interrupts
static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

fn timer_interrupt() {
    SCHEDULER.lock().schedule_next();  // Deadlock if lock held!
}

// ✅ CORRECT - Disable interrupts while holding lock
use crate::attunement::interrupts::without_interrupts;

fn schedule() {
    without_interrupts(|| {
        SCHEDULER.lock().schedule_next();
    });
}
```

**Rule:** Any data structure accessed from interrupt handlers MUST use `without_interrupts()` when locking.

#### 3. **Stack Alignment for x86-64**

```rust
// ❌ WRONG - Stack not 16-byte aligned
const STACK_SIZE: usize = 4096;

// ✅ CORRECT - Stack is 16n-8 aligned (RSP % 16 == 8 before call)
#[repr(C, align(16))]
struct ThreadStack {
    data: [u8; STACK_SIZE],
}
```

**Rule:** All thread stacks MUST be 16-byte aligned. Stack pointer MUST be `16n-8` before `call` instructions.

#### 4. **No Panics in Critical Sections**

```rust
// ❌ WRONG - Panic in interrupt handler
fn keyboard_interrupt() {
    let key = read_scancode();
    assert!(key != 0);  // Could panic!
}

// ✅ CORRECT - Graceful error handling
fn keyboard_interrupt() {
    let key = read_scancode();
    if key == 0 {
        log::warn!("Invalid scancode received");
        return;
    }
}
```

**Rule:** Interrupt handlers and critical sections MUST NOT panic. Use `log::warn!()` or `log::error!()` instead.

#### 5. **Preserve AethelOS Naming Conventions**

```rust
// ❌ WRONG - Generic/boring names
struct MemoryAllocator { ... }
fn schedule_thread() { ... }

// ✅ CORRECT - Poetic, metaphorical names
struct ManaPool { ... }
fn weave_thread_into_loom() { ... }
```

**Rule:** Use thematic names that match AethelOS philosophy:
- **Memory:** Mana Pool, Sanctuary (persistent), Ephemeral Mist (temporary)
- **Threading:** Loom of Fate, Weaving (running), Resting (blocked), Tangled (deadlock), Fading (terminating)
- **Files:** Scrolls (text), Tomes (binaries), Runes (config), Tapestries (images)
- **Time:** Moments, Heartbeats (ticks), Chronurgy (versioning)

### 🟡 IMPORTANT GUIDELINES

#### Error Handling

```rust
// Prefer Result<T, E> over panicking
pub fn create_thread(entry: fn()) -> Result<ThreadId, ThreadError> {
    if threads.len() >= MAX_THREADS {
        return Err(ThreadError::TooManyThreads);
    }
    // ...
}
```

#### Documentation

```rust
/// Creates a new thread in the Loom of Fate.
///
/// # Arguments
/// * `entry` - The function to execute in the new thread
/// * `priority` - Thread priority (High, Normal, Low, Idle)
///
/// # Returns
/// * `Ok(ThreadId)` - The ID of the newly created thread
/// * `Err(ThreadError)` - If thread creation fails
///
/// # Example
/// ```
/// let id = loom.weave(worker_function, Priority::Normal)?;
/// ```
pub fn weave(&mut self, entry: fn(), priority: Priority) -> Result<ThreadId, ThreadError>
```

**Rule:** Public APIs MUST have doc comments with Arguments, Returns, and Examples.

#### Performance Comments

```rust
// PERF: Using spin loop instead of hlt because context switches
// are expensive (~1000 cycles) vs spin check (~10 cycles)
loop {
    if self.should_yield() {
        break;
    }
}
```

**Rule:** Non-obvious performance decisions MUST be documented with `// PERF:` comments.

#### TODO Markers

```rust
// TODO(phase-2): Implement priority-based preemption
// See: docs/PREEMPTIVE_MULTITASKING_PLAN.md

// TODO(optimization): Cache harmony scores to avoid recalculation
```

**Rule:** TODOs MUST reference:
- Which implementation phase they belong to
- Related documentation if applicable

### 🟢 BEST PRACTICES

#### Const Over Mut

```rust
// Prefer const/immutable by default
const MAX_THREADS: usize = 256;
const STACK_SIZE: usize = 16384;

// Only use mut when mutation is necessary
let mut harmony_score = 0.0;
```

#### Type Aliases for Clarity

```rust
type ThreadId = usize;
type Timestamp = u64;  // Ticks since boot
type HarmonyScore = f32;  // 0.0 = parasitic, 1.0 = perfect harmony
```

#### Bitflags for State

```rust
use bitflags::bitflags;

bitflags! {
    pub struct ThreadFlags: u32 {
        const WEAVING = 0b00000001;  // Running
        const RESTING = 0b00000010;  // Blocked
        const FADING  = 0b00000100;  // Terminating
    }
}
```

### 🔍 Code Review Checklist

Before committing code, verify:

- [ ] All `unsafe` blocks have `/// SAFETY:` comments
- [ ] Interrupt-accessed data uses `without_interrupts()`
- [ ] No panics in interrupt handlers or critical sections
- [ ] Public functions have doc comments
- [ ] Naming follows AethelOS conventions
- [ ] No raw `println!()` in kernel (use `crate::println!()` macro)
- [ ] Stack allocations are properly aligned
- [ ] Error paths return `Result`, not panic
- [ ] TODOs reference implementation phases
- [ ] Code compiles without warnings: `cargo build --target x86_64-aethelos.json`

---

## Documentation Index

### Core Architecture Documents

| Document | Purpose | Status |
|----------|---------|--------|
| **[README.md](README.md)** | Project overview, build instructions | Current |
| **[DESIGN.md](DESIGN.md)** | High-level design philosophy | Current |
| **[GENESIS.scroll](GENESIS.scroll)** | Philosophical foundation (if exists) | Check if exists |

### Implementation Plans

| Plan | Scope | Status |
|------|-------|--------|
| **[PREEMPTIVE_MULTITASKING_PLAN.md](docs/PREEMPTIVE_MULTITASKING_PLAN.md)** | Timer-based preemption, interrupt-safe locks | ✅ Implemented |
| **[VGA_GRAPHICS_MODE_PLAN.md](docs/VGA_GRAPHICS_MODE_PLAN.md)** | Graphics mode, PSF fonts, Unicode support | 🟡 Designed |
| **[WORLD_TREE_PLAN.md](docs/WORLD_TREE_PLAN.md)** | Query-based filesystem, versioning, pruning | 🟡 Designed |
| **[GLIMMER_FORGE_PLAN.md](docs/GLIMMER_FORGE_PLAN.md)** | Scripting language + Rust compiler | 🟡 Designed |
| **[PRODUCTION_READINESS_PLAN.md](docs/PRODUCTION_READINESS_PLAN.md)** | Kernel hardening tasks | 🟡 In progress |

### Reading Order for New Contributors

1. **Start:** `README.md` - Get the big picture
2. **Philosophy:** `DESIGN.md` - Understand the "why"
3. **Current Code:** Browse `heartwood/src/` - See what's implemented
4. **Next Steps:** `docs/VGA_GRAPHICS_MODE_PLAN.md` or `docs/WORLD_TREE_PLAN.md` - Understand what's planned

---

## Current Status & Roadmap

### ✅ Completed (v0.1.0-alpha)

**January 2025:**
- Multiboot2 bootloader integration
- VGA text mode (80×25, Code Page 437)
- Preemptive multitasking with priority scheduling
- Buddy allocator (64B - 64KB blocks)
- Interrupt handling (IDT, PIC, timer, keyboard)
- Interactive shell (Eldarin) with command history, backspace
- Thread management (create, yield, terminate)
- Serial port logging
- Thematic shell commands: `mana-flow`, `uptime`, `rest`

### 🚧 In Progress

- VGA Graphics Mode (Phase 1: Infrastructure)
- World-Tree filesystem integration
- Shell command expansion

### 🎯 Next Priorities (v0.2.0)

**Q1 2025:**
1. **VGA Graphics Mode** (1-2 weeks)
   - Phase 1-2: Basic graphics + font rendering
   - Enable proper Unicode display (◈ symbol!)

2. **World-Tree Grove** (3-4 weeks)
   - Phase 1-3: Core storage, query language, versioning
   - Integration with Eldarin shell

3. **The Weave Compositor** (4-6 weeks)
   - Depends on graphics mode
   - Scene graph rendering
   - Window management basics

### 🔮 Long-Term Vision (v1.0+)

**2025-2026:**
- Glimmer-Weave scripting language
- Runic Forge compiler
- Network Sprite
- Self-hosting capability
- Package ecosystem

---

## Common Tasks

### Adding a New Shell Command

1. **Edit `heartwood/src/eldarin.rs`:**

```rust
// In process_command() function
match parts[0] {
    "help" => cmd_help(),
    "uptime" => cmd_uptime(),
    "my-command" => cmd_my_command(args),  // Add here
    // ...
}

// Add implementation
fn cmd_my_command(args: &[&str]) {
    if args.is_empty() {
        crate::println!("Usage: my-command <arg>");
        return;
    }

    crate::println!("◈ My Command Output");
    // Your logic here
}
```

2. **Rebuild and test:**
```bash
cd heartwood && cargo build --target x86_64-aethelos.json
```

### Adding a New Thread

1. **Create thread function:**

```rust
fn my_worker_thread() -> ! {
    loop {
        // Do work

        crate::loom_of_fate::yield_now();  // Cooperative yield
    }
}
```

2. **Spawn in `heartwood/src/main.rs`:**

```rust
loom.weave(
    my_worker_thread,
    Priority::Normal,
    "My Worker"
)?;
```

### Debugging

**Serial output:**
```rust
use crate::serial_println;

serial_println!("Debug: value = {}", my_value);
```

**Check logs:**
```bash
# After running QEMU, check serial.log
cat serial.log
```

**QEMU debugging flags:**
```bash
qemu-system-x86_64 \
  -cdrom aethelos.iso \
  -serial file:serial.log \
  -d int,cpu_reset,guest_errors \  # Enable debug output
  -D qemu-debug.log \               # QEMU internal log
  -no-reboot -no-shutdown           # Halt on panic
```

### Testing Changes

1. **Build:** `cd heartwood && cargo build --target x86_64-aethelos.json`
2. **Check warnings:** Should build cleanly with no warnings
3. **Create ISO:** `cd .. && wsl bash -c "cp target/x86_64-aethelos/debug/heartwood isodir/boot/aethelos/heartwood.bin && grub-mkrescue -o aethelos.iso isodir"`
4. **Test in QEMU:** `BOOT_AETHELOS.bat` (or direct QEMU command)
5. **Verify:** Check that changes work as expected
6. **Check logs:** `cat serial.log` for any errors

---

## Working with Claude/AI Assistants

### When to Ask for Clarification

- **Ambiguous requirements:** "Should this be cooperative or preemptive?"
- **Multiple valid approaches:** "Use spinlock or mutex here?"
- **Breaking changes:** "This changes the API - is that okay?"
- **Naming conventions:** "What's the AethelOS name for X?"

### What to Preserve

- **Naming style:** Poetic, metaphorical (Loom, Mana, Weave)
- **Architecture:** Capability-based, cooperative, harmony-focused
- **Code safety:** All safety invariants must be preserved
- **Documentation:** Plans in `docs/` are authoritative

### What Can Be Modified

- **Implementation details:** How something works internally
- **Performance:** Optimizations that maintain correctness
- **Error messages:** Make them clearer/more helpful
- **Code organization:** Refactoring for clarity

---

## Philosophy Reminders

### When Stuck, Ask:

1. **"What would be harmonious?"** - Favor cooperation over force
2. **"What would preserve memory?"** - Keep history, enable rollback
3. **"What would be beautiful?"** - Aesthetics reveal truth
4. **"What would be natural?"** - Security through inherent properties, not bolted-on rules

### Naming Guidelines:

- **Memory:** Mana, Sanctuary, Ephemeral Mist
- **Threading:** Loom, Weaving, Resting, Tangled, Fading
- **Files:** Scrolls, Tomes, Runes, Tapestries, Chronicles
- **Time:** Moments, Heartbeats, Chronurgy
- **UI:** The Weave, Glyphs (shaders), Lanthir (window manager)
- **Networking:** Sprite (daemon), Realms (remote systems)

**Bad:** `MemoryAllocator`, `TaskScheduler`, `FileSystem`
**Good:** `ManaPool`, `LoomOfFate`, `WorldTree`

---

## Emergency Contacts

### Critical Files - DO NOT DELETE

- `heartwood/x86_64-aethelos.json` - Target specification
- `rust-toolchain.toml` - Rust version config
- `isodir/boot/grub/grub.cfg` - Bootloader config
- `docs/*.md` - Architecture plans

### If Build Breaks

1. Check `cargo --version` - Should be nightly
2. Check `rustup component list` - Should have `rust-src`, `llvm-tools-preview`
3. Check target file exists: `heartwood/x86_64-aethelos.json`
4. Clean build: `cargo clean && cargo build --target x86_64-aethelos.json`
5. Check WSL: `wsl --status` (for ISO creation)

---

> **Remember:** AethelOS is an exploration, not a product. Prioritize learning, beauty, and principled design over feature velocity.

*Last updated: January 2025*
