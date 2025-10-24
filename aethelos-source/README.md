# AethelOS - A Symbiotic Operating System

![Version](https://img.shields.io/badge/version-0.1.0--alpha-blue)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)
![Language](https://img.shields.io/badge/language-Rust-orange)

> *"The code does not command the silicon. The silicon does not serve the code. They dance together, and in that dance, life emerges."*

---

## Overview

AethelOS is a radical reimagining of what an operating system can be. It is not Linux, not Windows, not macOS - it is a self-contained digital realm built on the principles of **symbiotic computing**, where the OS, hardware, and user exist in harmonious equilibrium.

## Core Philosophy

### Symbiotic Computing

- **Harmony Over Force**: The system negotiates, not preempts
- **Memory Over Forgetting**: Every file carries the memory-rings of its history
- **Beauty as Necessity**: Aesthetics reveal intuitive system state
- **Security Through Nature**: Safety through natural capability boundaries

## Architecture

### The Heartwood (Kernel)

A hybrid microkernel containing only the most sacred responsibilities:

- **The Loom of Fate**: Harmony-based cooperative scheduler
  - Thread states: Weaving, Resting, Tangled, Fading
  - Resource negotiation based on system-wide harmony
  - Parasite detection and throttling (not killing)

- **The Mana Pool**: Capability-based memory management
  - No raw pointers in user space
  - Purpose-driven allocation (Sanctuary vs Ephemeral Mist)
  - Automatic reclamation via reference counting

- **The Nexus**: High-speed asynchronous message passing (IPC)
  - Priority-aware message delivery
  - Capability-based addressing
  - Zero-copy where possible

- **Attunement Layer**: Hardware abstraction interface
  - CPU feature detection and management
  - Interrupt handling
  - Timer management

### The Groves (User-Space Services)

Isolated processes that grow from the Heartwood:

- **World-Tree Grove**: Relational database filesystem
  - Query-based file access (not path-based)
  - Built-in versioning (Chronurgy)
  - Rich metadata (Creator, Genesis Time, Essence, Connections)

- **The Weave Grove**: Vector-based scene graph compositor
  - Resolution-independent rendering
  - First-class shader support (Glyphs)
  - Fluid transformations and effects

- **Lanthir Grove**: Window management service
  - Harmonic window arrangement
  - Non-rectangular window shapes

- **Network Sprite**: Network daemon
  - Connection-oriented architecture
  - Natural data flow

### Ancient Runes (Core Libraries)

APIs for developers:

- **Corelib**: Standard data structures and utilities
- **Weaving API**: Toolkit for graphical applications
- **Eldarin Script**: Shell interaction library

## Project Structure

```
aethelos-source/
├── GENESIS.scroll           # Philosophical and architectural overview
├── Cargo.toml               # Workspace configuration
│
├── awakening/               # Bootloader
│   ├── boot.asm             # First stage (assembly)
│   └── heartwood_loader/    # Second stage (Rust)
│
├── heartwood/               # The Kernel
│   ├── nexus/               # IPC system
│   ├── loom-of-fate/        # Scheduler
│   ├── mana-pool/           # Memory management
│   └── attunement/          # Hardware abstraction
│
├── groves/                  # User-space services
│   ├── world-tree_grove/    # Filesystem
│   ├── the-weave_grove/     # Compositor
│   ├── lanthir_grove/       # Window manager
│   └── network_sprite/      # Network daemon
│
└── ancient-runes/           # Core libraries
    ├── corelib/             # Standard library
    ├── weaving/             # GUI toolkit
    └── script/              # Shell API
```

## Building

### Prerequisites

- Rust 1.75 or later
- NASM (for boot.asm)
- QEMU (for testing)

### Build Commands

```bash
# Build the entire workspace
cargo build --release

# Build just the kernel
cd heartwood
cargo build --release

# Build a specific Grove
cd groves/world-tree_grove
cargo build --release
```

### Running in QEMU

```bash
# Assemble the bootloader
nasm -f bin awakening/boot.asm -o boot.bin

# Create a bootable disk image
# (Additional steps would be needed for a complete bootable image)

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=boot.bin
```

## Key Innovations

### 1. Harmony-Based Scheduling

Instead of preemptive scheduling with fixed time slices, the Loom of Fate:
- Analyzes system-wide harmony metrics
- Detects parasitic behavior through resource usage patterns
- Throttles (soothes) greedy processes instead of killing them
- Rewards cooperative yielding behavior

### 2. Capability-Based Memory

User-space processes never see raw memory addresses:
- All memory access is through opaque handles
- Capabilities grant specific rights (read, write, execute, transfer)
- MMU enforces boundaries at hardware level
- Automatic deallocation when last handle is released

### 3. Query-Based Filesystem

Files are database objects, not paths:
```rust
// Instead of: /home/user/documents/poem.txt
// You query:
Seek {
    Essence: "Scroll",
    Creator: "Elara",
    Name: "Poem"
}
```

Built-in versioning means you can access any historical state:
```rust
// Read the file as it existed 3 days ago
Seek {
    Essence: "Scroll",
    Name: "Config",
    Timestamp: now() - days(3)
}
```

### 4. Vector-Based Graphics

The Weave renders everything mathematically:
- Windows defined as Bézier curves
- Infinite resolution independence
- Shaders (Glyphs) as first-class primitives
- Fluid animations through transform modifications

## Current Status

**Version 0.1.0-alpha** - "The First Breath"

This is a foundational implementation demonstrating the core architecture and philosophy. Currently implemented:

✅ Complete architectural design
✅ Heartwood kernel structure (Nexus, Loom of Fate, Mana Pool, Attunement)
✅ All Grove services (stubs)
✅ Ancient Runes libraries (basic implementations)
✅ Boot sequence design

### Not Yet Implemented

- Actual hardware initialization
- Real memory allocator (currently using placeholder bump allocator)
- Complete interrupt handling
- Physical device drivers
- Full filesystem implementation
- Graphics rendering pipeline
- Network stack

## Why AethelOS?

AethelOS is not meant to replace existing operating systems. It's an exploration of what's possible when we:

1. **Question Assumptions**: Why must files be paths? Why must scheduling be preemptive?
2. **Prioritize Beauty**: Can an OS be art as well as utility?
3. **Embrace Metaphor**: Can naming and design reflect a coherent philosophy?
4. **Value Longevity**: What if we designed for 100-year timescales?

## Contributing

This is currently an experimental, educational project. Contributions are welcome, especially for:

- Completing hardware initialization
- Implementing real device drivers
- Building out the graphics pipeline
- Creating example applications using Ancient Runes

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Inspired by:
- The microkernel philosophy (Minix, L4, seL4)
- Capability-based security (KeyKOS, EROS)
- Plan 9's everything-is-a-file taken further
- The aesthetic vision of Elven computing

---

> *"This is not an OS for everyone. It is an OS for those who believe computing can be more than utility—that it can be art, philosophy, and symbiosis."*

*For more details, see [GENESIS.scroll](GENESIS.scroll)*
