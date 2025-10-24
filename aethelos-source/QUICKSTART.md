# AethelOS Quick Start Guide

Welcome to AethelOS! This guide will help you understand, build, and explore the system.

## Prerequisites

### Required Tools

1. **Rust Toolchain** (1.75 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default stable
   ```

2. **NASM** (Netwide Assembler)
   ```bash
   # Ubuntu/Debian
   sudo apt install nasm

   # macOS
   brew install nasm

   # Windows
   # Download from https://www.nasm.us/
   ```

3. **QEMU** (for testing)
   ```bash
   # Ubuntu/Debian
   sudo apt install qemu-system-x86

   # macOS
   brew install qemu

   # Windows
   # Download from https://www.qemu.org/download/
   ```

4. **Make** (optional but recommended)
   ```bash
   # Ubuntu/Debian
   sudo apt install build-essential

   # macOS
   xcode-select --install
   ```

## Understanding the Codebase

### Architecture Overview

```
The Heartwood (Kernel)
├── Nexus: IPC system
├── Loom of Fate: Harmony-based scheduler
├── Mana Pool: Capability-based memory
└── Attunement: Hardware abstraction

The Groves (User Space)
├── World-Tree: Relational filesystem
├── The Weave: Vector compositor
├── Lanthir: Window manager
└── Network Sprite: Network daemon

Ancient Runes (Libraries)
├── Corelib: Standard library
├── Weaving: GUI toolkit
└── Script: Shell API
```

### Key Files to Read

1. **[GENESIS.scroll](GENESIS.scroll)** - The philosophical foundation
2. **[DESIGN.md](DESIGN.md)** - Technical deep dive
3. **[heartwood/src/main.rs](heartwood/src/main.rs)** - Kernel entry point
4. **[heartwood/src/nexus/mod.rs](heartwood/src/nexus/mod.rs)** - IPC system
5. **[heartwood/src/loom_of_fate/scheduler.rs](heartwood/src/loom_of_fate/scheduler.rs)** - Scheduler

## Building AethelOS

### Using Make (Recommended)

```bash
# Show available targets
make info

# Build everything
make all

# Build individual components
make bootloader
make kernel
make groves
make runes

# Run tests
make test

# Clean build artifacts
make clean
```

### Using Cargo Directly

```bash
# Build the entire workspace
cargo build --release

# Build just the kernel
cd heartwood
cargo build --release

# Build a specific Grove
cd groves/world-tree_grove
cargo build --release

# Run tests
cargo test --workspace
```

### Build Output

After building, you'll find:
- **Kernel**: `target/release/heartwood`
- **Groves**: `target/release/world-tree_grove`, etc.
- **Libraries**: In each component's `target/release/` directory

## Running AethelOS

### Current Status

**Note**: The current version (0.1.0-alpha) is a foundational implementation. The boot sequence and hardware initialization are not yet complete, so you cannot boot AethelOS on real hardware or in QEMU yet.

What you *can* do:
1. Explore the code architecture
2. Run unit tests
3. Use the libraries in Rust projects
4. Experiment with the design

### When Bootable (Future)

```bash
# Assemble bootloader
nasm -f bin awakening/boot.asm -o build/boot.bin

# Create bootable image
# (Additional tools would be needed)

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=aethelos.img
```

## Exploring the Code

### Example 1: Understanding the Nexus (IPC)

The Nexus is the communication backbone of AethelOS. Here's how it works:

```rust
// Create a channel (in heartwood/src/nexus/mod.rs)
let (cap_a, cap_b) = nexus::create_channel()?;

// Send a message
let message = Message::new(
    MessageType::ResourceRequest {
        resource_type: ResourceType::Memory,
        amount: 4096,
    },
    MessagePriority::Normal,
);
nexus::send(cap_a.channel_id, message)?;

// Receive the message
let received = nexus::try_receive(cap_b.channel_id)?;
```

**Key Points**:
- Communication is through capabilities (cap_a, cap_b), not raw IDs
- Messages are prioritized (Critical, High, Normal, Low, Idle)
- Non-blocking by default

### Example 2: Understanding the Loom of Fate (Scheduler)

The scheduler maintains system harmony:

```rust
// Spawn a thread (in heartwood/src/loom_of_fate/mod.rs)
fn my_thread() -> ! {
    loop {
        // Do work
        perform_task();

        // Yield cooperatively
        loom_of_fate::yield_now();
    }
}

let thread_id = loom_of_fate::spawn(my_thread, ThreadPriority::Normal)?;

// Get scheduler stats
let stats = loom_of_fate::stats();
println!("System harmony: {}", stats.system_harmony);
println!("Parasites detected: {}", stats.parasite_count);
```

**Key Points**:
- Threads yield voluntarily (cooperative scheduling)
- Harmony scores track resource usage
- Parasitic threads are throttled, not killed

### Example 3: Understanding the Mana Pool (Memory)

Memory is managed as abstract objects:

```rust
// Allocate memory (in heartwood/src/mana_pool/mod.rs)
let handle = mana_pool::animate(
    4096,  // Size in bytes
    AllocationPurpose::LongLived,  // Goes to Sanctuary
)?;

// Memory is accessed through the handle (not raw pointers)
// When handle is dropped, memory is automatically freed

// Get pool statistics
let stats = mana_pool::stats();
println!("Sanctuary used: {} / {}",
    stats.sanctuary_used,
    stats.sanctuary_total
);
```

**Key Points**:
- No raw pointers in user space
- Purpose-driven allocation (Sanctuary vs Ephemeral Mist)
- Automatic reclamation when handle is dropped

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific component
cd heartwood
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_nexus_create_channel
```

## Code Style and Conventions

### Naming

AethelOS uses evocative, metaphorical names:

| Concept | AethelOS Name | Reason |
|---------|---------------|--------|
| Kernel | Heartwood | Core of a tree |
| Scheduler | Loom of Fate | Weaving threads |
| Memory Manager | Mana Pool | Lifeblood of the system |
| IPC | Nexus | Connection point |
| User Services | Groves | Growing from the Heartwood |
| Libraries | Ancient Runes | Inscribed knowledge |

### Code Organization

```rust
// Module structure
mod nexus;
mod loom_of_fate;
mod mana_pool;

// Each module has:
// - mod.rs: Public API and exports
// - {component}.rs: Implementation details
// - tests.rs: Unit tests (optional)
```

### Documentation

Every public item should have documentation:

```rust
/// Brief description of what this does
///
/// ## Arguments
/// * `param` - What this parameter means
///
/// ## Returns
/// What this function returns
///
/// ## Errors
/// When this might fail
///
/// ## Example
/// ```rust
/// let result = function(42)?;
/// ```
pub fn function(param: i32) -> Result<(), Error> {
    // ...
}
```

## Development Workflow

### 1. Make Changes

Edit files in your editor of choice.

### 2. Format Code

```bash
cargo fmt --all
```

### 3. Check for Errors

```bash
cargo check --workspace
```

### 4. Run Lints

```bash
cargo clippy --all-targets --all-features
```

### 5. Run Tests

```bash
cargo test --workspace
```

### 6. Build

```bash
make all
# or
cargo build --release --workspace
```

## Common Tasks

### Adding a New Grove Service

1. Create directory: `groves/my_grove/`
2. Add Cargo.toml:
   ```toml
   [package]
   name = "my_grove"
   version.workspace = true
   authors.workspace = true
   edition.workspace = true
   ```
3. Add to workspace in root Cargo.toml:
   ```toml
   members = [
       # ...
       "groves/my_grove",
   ]
   ```
4. Implement in `groves/my_grove/src/lib.rs`

### Adding a System Call

1. Define message type in `heartwood/src/nexus/message.rs`:
   ```rust
   pub enum MessageType {
       // ...
       MyNewRequest { data: u64 },
   }
   ```

2. Handle in appropriate kernel component

3. Export through public API

### Adding a Widget to Weaving

1. Edit `ancient-runes/weaving/src/lib.rs`
2. Implement the `Widget` trait:
   ```rust
   pub struct MyWidget {
       // fields
   }

   impl Widget for MyWidget {
       fn natural_size(&self) -> (f32, f32) { /* ... */ }
       fn render(&self) -> WidgetNode { /* ... */ }
       fn handle_event(&mut self, event: Event) { /* ... */ }
   }
   ```

## Troubleshooting

### Build Errors

**Problem**: Missing dependencies

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

**Problem**: Linker errors

Check that you have the correct toolchain:
```bash
rustc --version
cargo --version
```

### Test Failures

```bash
# Run with verbose output
cargo test -- --nocapture --test-threads=1

# Run specific failing test
cargo test failing_test_name -- --exact
```

## Next Steps

1. **Read the design document**: [DESIGN.md](DESIGN.md)
2. **Explore the kernel**: Start with [heartwood/src/main.rs](heartwood/src/main.rs)
3. **Try the examples**: See the examples/ directory (when created)
4. **Join development**: See CONTRIBUTING.md (when created)

## Resources

- **Official Docs**: See [DESIGN.md](DESIGN.md) and [GENESIS.scroll](GENESIS.scroll)
- **Rust Book**: https://doc.rust-lang.org/book/
- **OS Dev Wiki**: https://wiki.osdev.org/
- **Rust OS Tutorial**: https://os.phil-opp.com/

## Getting Help

Since this is an experimental project, the best way to learn is by reading the code and documentation. Key files to understand:

1. **GENESIS.scroll** - Philosophy
2. **DESIGN.md** - Architecture
3. **README.md** - Overview
4. **This file** - Practical guide

## Philosophy

Remember: AethelOS is not about building a production OS. It's about exploring *what's possible* when we question our assumptions about how operating systems should work.

The code is the documentation. The architecture is the argument. The system is the proof.

---

*"The code does not command the silicon. The silicon does not serve the code. They dance together, and in that dance, life emerges."*

Happy exploring! 🌳
