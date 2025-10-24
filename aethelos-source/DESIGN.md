# AethelOS Design Document

## Table of Contents

1. [Introduction](#introduction)
2. [Core Philosophy](#core-philosophy)
3. [Architectural Decisions](#architectural-decisions)
4. [The Heartwood (Kernel)](#the-heartwood-kernel)
5. [The Groves (User Space)](#the-groves-user-space)
6. [Ancient Runes (Libraries)](#ancient-runes-libraries)
7. [Implementation Details](#implementation-details)
8. [Security Model](#security-model)
9. [Future Work](#future-work)

---

## Introduction

AethelOS is an experimental operating system designed to explore radical alternatives to conventional OS design. Rather than optimizing purely for performance or compatibility, AethelOS prioritizes:

- **Harmony**: System-wide equilibrium over individual process optimization
- **Longevity**: 100-year design timescales
- **Beauty**: Aesthetics as a first-class design principle
- **Symbiosis**: Cooperation between user, OS, and hardware

This document explains the technical decisions behind the philosophy.

---

## Core Philosophy

### Symbiotic Computing

Traditional operating systems establish a hierarchy:
```
User → OS → Hardware
```

AethelOS establishes a triangle of mutual dependence:
```
       User
      /    \
     /      \
   OS ←→ Hardware
```

Each component:
- **Respects** the limitations of the others
- **Negotiates** rather than demands
- **Adapts** to changing conditions

### Implications

This philosophy manifests in concrete design choices:

1. **Cooperative Scheduling**: Threads yield voluntarily, guided by harmony metrics
2. **Resource Negotiation**: Processes request resources and can be denied
3. **Graceful Degradation**: No component ever crashes the entire system
4. **Transparent State**: The UI reveals system state through living metaphors

---

## Architectural Decisions

### Why a Hybrid Microkernel?

**Decision**: Use a minimal kernel (microkernel philosophy) with some performance-critical services in kernel space (hybrid approach).

**Rationale**:
- **Security**: Small trusted computing base (TCB)
- **Resilience**: Driver crashes don't kill the kernel
- **Flexibility**: Services can be replaced without rebooting
- **Performance**: Critical paths (IPC, memory) optimized in kernel

**Comparison**:
- **vs Monolithic** (Linux): Better isolation, worse performance
- **vs Pure Microkernel** (L4): Better performance, slightly larger TCB
- **vs Exokernel**: More abstraction, easier to program

### Why Capability-Based Security?

**Decision**: All resources accessed through unforgeable capabilities, not addresses or file paths.

**Rationale**:
- **Prevents** confused deputy attacks
- **Enables** fine-grained access control
- **Eliminates** ambient authority
- **Simplifies** security reasoning (no ACLs needed)

**Example**:
```rust
// Instead of:
let ptr = 0x1000 as *mut u8;  // Can access any memory!
unsafe { *ptr = 42; }

// AethelOS uses:
let handle = mana_pool::animate(size, purpose)?;
// Handle can only access the allocated region
```

### Why a Relational Filesystem?

**Decision**: Files are database objects with metadata, not paths in a hierarchy.

**Rationale**:
- **Discovery**: Query is more powerful than path navigation
- **Metadata**: Rich context (creator, time, relationships) is mandatory
- **Versioning**: Built-in time-travel without extra tools
- **Flexibility**: No need to reorganize when mental models change

**Example**:
```rust
// Instead of: /home/user/projects/rust/aethelos/src/main.rs
// Which forces a single hierarchy

// Query:
Seek {
    Essence: "Rune",  // Executable
    Creator: "Elara",
    Project: "AethelOS",
    Language: "Rust",
}
// Multiple ways to find the same file
```

### Why Vector-Based Graphics?

**Decision**: Render everything as mathematical primitives (Bézier curves, gradients).

**Rationale**:
- **Resolution Independence**: Perfect rendering at any DPI
- **Animations**: Transform scene graph nodes, not pixels
- **Aesthetics**: Smooth curves and gradients are natural
- **GPU Friendly**: Modern GPUs excel at vector rendering

---

## The Heartwood (Kernel)

### Component: The Nexus (IPC)

**Purpose**: All inter-process communication flows through the Nexus.

**Architecture**:
```
Process A                    Process B
    |                            |
    | (capability to channel)    |
    v                            v
  Channel ← Nexus Core → Channel
```

**Key Features**:
- **Asynchronous**: Non-blocking message passing
- **Priority-Aware**: Critical messages delivered first
- **Capability-Based**: Processes hold channel capabilities, not IDs
- **Zero-Copy**: Messages live in shared memory when possible

**Message Structure**:
```rust
pub struct Message {
    message_type: MessageType,  // What kind of message
    priority: MessagePriority,  // How urgent
    reply_to: Option<u64>,      // For request-response
}
```

**Performance**:
- **Target**: < 1μs for local message passing
- **Comparison**: Similar to L4 IPC, faster than UNIX pipes

### Component: The Loom of Fate (Scheduler)

**Purpose**: Maintain system-wide harmony while ensuring progress.

**Architecture**:
```
Threads → Harmony Analyzer → Scheduler → CPU
             ↓
      (harmony scores)
```

**Thread States**:
- **Weaving**: Actively running
- **Resting**: Idle, ready to run
- **Tangled**: Blocked on I/O or error
- **Fading**: Exiting gracefully

**Harmony Calculation**:
```rust
harmony_score =
    0.7 * thread_cooperation +
    0.2 * resource_efficiency +
    0.1 * yield_frequency
```

**Parasite Detection**:
A thread is considered parasitic if:
- Harmony score < 0.3
- Excessive CPU usage without yielding
- Memory hoarding

**Response to Parasites**:
1. **First**: Throttle (reduce CPU allocation)
2. **Second**: Send disharmony signal to parent
3. **Last Resort**: Terminate (only if explicitly requested by user)

**Why Not Preemptive?**:
- Preemption is *reactive* (respond after harm)
- Cooperation is *proactive* (prevent harm)
- Yields provide natural scheduling points
- Better for cache locality and battery life

### Component: The Mana Pool (Memory)

**Purpose**: Manage memory as abstract objects, not raw bytes.

**Architecture**:
```
User Process
    ↓
 (Handle)
    ↓
Object Manager
    ↓
Sanctuary / Ephemeral Mist
    ↓
Physical Memory
```

**Allocation Types**:

| Purpose | Region | Strategy |
|---------|--------|----------|
| Long-lived | Sanctuary | Conservative, stable addresses |
| Short-lived | Ephemeral Mist | Aggressive reclamation |
| Static | Sanctuary | Never freed |
| Ephemeral | Ephemeral Mist | Immediate reclamation |

**Capability Model**:
```rust
pub struct Capability {
    handle: ObjectHandle,
    rights: CapabilityRights,  // READ, WRITE, EXECUTE, TRANSFER
}
```

**Memory Safety**:
- User space: No raw pointers
- Kernel space: Rust's borrow checker
- Hardware: MMU enforces capability boundaries

### Component: Attunement Layer

**Purpose**: Abstract hardware details while maintaining performance.

**Responsibilities**:
- CPU feature detection and management
- Interrupt handling and routing
- Timer management
- Device discovery

**Design Principle**: "Attuning" not "Controlling"
- Discover what hardware *can* do
- Adapt OS behavior to hardware capabilities
- Never assume specific hardware features

---

## The Groves (User Space)

### World-Tree Grove (Filesystem)

**File as Object**:
```rust
pub struct FileObject {
    id: FileId,
    creator: String,
    genesis_time: u64,
    essence: FileEssence,      // Type
    connections: Vec<FileId>,  // Relationships
    data: Vec<u8>,
    versions: Vec<FileVersion>, // History
}
```

**Query Language**:
```rust
pub struct FileQuery {
    essence: Option<FileEssence>,
    creator: Option<String>,
    name_pattern: Option<String>,
    time_range: Option<(u64, u64)>,
    connected_to: Option<FileId>,
}
```

**Versioning (Chronurgy)**:
- Every write creates a new version
- Old versions automatically retained
- Query by timestamp to access history
- Garbage collection based on policy

**Example Queries**:
```rust
// All images created by Elara
Seek { essence: Some(Tapestry), creator: Some("Elara") }

// Config files modified in the last day
Seek { essence: Some(Scroll), time_range: Some((now() - 1day, now())) }

// All files connected to README
Seek { connected_to: Some(readme_id) }
```

### The Weave Grove (Compositor)

**Scene Graph**:
```
Root
├── Window 1
│   ├── Title Bar
│   │   └── Text
│   └── Content
│       ├── Button
│       └── Label
└── Window 2
    └── ...
```

**Rendering Pipeline**:
```
Scene Graph → Transform Application → Rasterization → Glyph (Shader) → Framebuffer
```

**Window Shapes**:
- Not limited to rectangles
- Defined by Bézier curves
- Natural organic shapes
- Animated transformations

**Glyphs (Shaders)**:
```rust
pub enum GlyphType {
    Shimmer,       // Subtle glow animation
    Glow,          // Radiant aura
    Trail,         // Motion blur effect
    Transparency,  // Alpha blending
    Distortion,    // Ripple/wave
}
```

### Lanthir Grove (Window Manager)

**Responsibilities**:
- Window placement and z-ordering
- Focus management
- Workspace organization
- Harmony-based window arrangement

**Design Principle**: Windows are arranged to minimize cognitive load:
- Related windows grouped spatially
- Size proportional to importance
- Smooth transitions, never jarring

### Network Sprite (Network)

**Connection States**:
```
Establishing → Connected ←→ Flowing ←→ Resting → Fading
```

**Design Principle**: Network as living connections
- Connections have state and lifecycle
- Data flows naturally, not pushed
- Backpressure handled gracefully

---

## Ancient Runes (Libraries)

### Corelib

Standard data structures and utilities:
- Collections (Vec, BTreeMap, VecDeque)
- String manipulation
- Math utilities (clamp, lerp, smoothstep)
- Error handling

### Weaving API

GUI toolkit for applications:
```rust
pub trait Widget {
    fn natural_size(&self) -> (f32, f32);
    fn render(&self) -> WidgetNode;
    fn handle_event(&mut self, event: Event);
}
```

**Widgets**:
- Button, Label, TextBox
- VBox, HBox (layout containers)
- Canvas (custom drawing)

### Eldarin Script

Shell interaction API:
```rust
// Execute commands
let result = script::execute("list scrolls")?;

// Interactive prompts
let name = script::prompt("What is your name?")?;
```

---

## Implementation Details

### Boot Sequence

1. **BIOS/UEFI** loads boot sector (boot.asm)
2. **Boot sector** loads second stage (heartwood_loader)
3. **Heartwood loader**:
   - Sets up paging (4-level page tables)
   - Maps kernel to higher half
   - Allocates initial heap
   - Loads kernel ELF
   - Jumps to kernel entry
4. **Heartwood** (`_start`):
   - Initializes VGA buffer
   - Initializes Mana Pool
   - Initializes Nexus
   - Initializes Loom of Fate
   - Initializes Attunement Layer
   - Spawns initial Groves
5. **Groves** start providing services

### Memory Layout

```
Virtual Memory:
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF : User space
0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel space

Kernel Space:
0xFFFF_8000_0000_0000 : Kernel code
0xFFFF_9000_0000_0000 : Kernel heap (Mana Pool)
0xFFFF_A000_0000_0000 : Device memory
```

### Context Switching

1. Timer interrupt or explicit yield
2. Save current thread context
3. Harmony analyzer updates scores
4. Scheduler selects next thread
5. Restore selected thread context
6. Return to user space

---

## Security Model

### Capabilities

Every resource is accessed through a capability:
```rust
pub struct Capability {
    handle: ObjectHandle,     // Unforgeable reference
    rights: CapabilityRights, // What you can do
}
```

**Properties**:
- **Unforgeable**: Cannot be guessed or constructed
- **Transferable**: Can be passed between processes (if TRANSFER right)
- **Revocable**: Can be invalidated by creator
- **Delegatable**: Can create derived capabilities with fewer rights

### Isolation

- **Processes**: Separate address spaces
- **Groves**: Run in user space, can crash independently
- **Heartwood**: Minimal TCB, protected by hardware

### Trust Model

```
User
  ↓
Applications (untrusted)
  ↓
Groves (partially trusted)
  ↓
Heartwood (fully trusted)
  ↓
Hardware (trusted)
```

---

## Future Work

### Near-Term (3-6 months)

- [ ] Complete hardware initialization
- [ ] Real page allocator
- [ ] Interrupt handling (IDT setup)
- [ ] Basic device drivers (keyboard, disk)
- [ ] VFS for World-Tree Grove

### Mid-Term (6-12 months)

- [ ] Graphics rendering pipeline
- [ ] Network stack (TCP/IP)
- [ ] Shell implementation
- [ ] Example applications

### Long-Term (1-2 years)

- [ ] Multi-core support
- [ ] GPU acceleration for Weave
- [ ] Package manager
- [ ] Developer tools (debugger, profiler)

### Research Questions

1. Can harmony-based scheduling compete with preemptive schedulers?
2. How much overhead does capability-based memory add?
3. Is query-based filesystem intuitive for users?
4. Can vector graphics match raster performance?

---

## Conclusion

AethelOS is an experiment in radical OS design. It asks: "What if we started from first principles, without the baggage of UNIX or Windows?"

The result is a system that prioritizes:
- **Harmony** over raw performance
- **Beauty** over familiarity
- **Longevity** over expedience
- **Symbiosis** over control

Whether these principles lead to a practical operating system remains to be seen. But the journey of exploration is valuable in itself.

---

*Last Updated: 2025-10-24*
*Version: 0.1.0-alpha*
