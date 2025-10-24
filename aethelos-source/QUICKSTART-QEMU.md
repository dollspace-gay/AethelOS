# Quick Start: Running AethelOS in QEMU

This guide gets you from zero to booting AethelOS in QEMU in under 5 minutes.

## TL;DR - Three Commands

```bash
# 1. Setup bootloader infrastructure
./setup-boot.sh      # Linux/macOS
.\setup-boot.ps1     # Windows

# 2. Build the kernel
./build.sh           # Linux/macOS
.\build.ps1          # Windows

# 3. Run in QEMU
./run-qemu.sh        # Linux/macOS
.\run-qemu.ps1       # Windows
```

## Prerequisites

### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Or visit: https://rustup.rs/

### 2. Install QEMU

**Windows:**
- Download: https://qemu.weilnetz.de/w64/
- Add to PATH or the scripts will find it automatically

**Linux (Ubuntu/Debian):**
```bash
sudo apt install qemu-system-x86
```

**macOS:**
```bash
brew install qemu
```

## Step-by-Step Setup

### Step 1: Run Setup Script

This creates the bootloader infrastructure (linker script, multiboot header, build config):

**Linux/macOS:**
```bash
chmod +x setup-boot.sh
./setup-boot.sh
```

**Windows PowerShell:**
```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\setup-boot.ps1
```

**What it does:**
- Creates `x86_64-aethelos.json` (custom target specification)
- Creates `heartwood/linker.ld` (linker script)
- Creates `heartwood/src/boot.rs` (Multiboot2 header)
- Creates `.cargo/config.toml` (build configuration)
- Creates `build.sh` and `run-qemu.sh` helper scripts

### Step 2: Build the Kernel

**Linux/macOS:**
```bash
./build.sh
```

**Windows:**
```powershell
.\build.ps1
```

This compiles AethelOS with:
- Nightly Rust (for `build-std`)
- Custom target (`x86_64-aethelos`)
- No standard library
- Multiboot2 bootable binary

**Output:** `target/x86_64-aethelos/debug/heartwood`

### Step 3: Run in QEMU

**Default (VGA display):**
```bash
./run-qemu.sh        # Linux/macOS
.\run-qemu.ps1       # Windows
```

**Other modes:**
```bash
# Serial console only (no GUI)
./run-qemu.sh serial

# Debug mode (shows interrupts, CPU resets)
./run-qemu.sh debug

# GDB debugging (pauses at start, waits for GDB)
./run-qemu.sh gdb
```

## What You'll See

When AethelOS boots successfully, you should see:

```
[] Awakening the Heartwood...
[] Kindling the Mana Pool...
[] Opening the Nexus...
[] Weaving the Loom of Fate...
[] Attuning to the hardware...
[] The Heartwood lives!
```

The system will then enter an idle loop (currently no interaction).

## QEMU Window Controls

- **Close Window** - Stops QEMU
- **Ctrl+Alt+G** - Release mouse capture
- **Ctrl+Alt+F** - Toggle fullscreen
- **Ctrl+Alt+1** - Switch to VGA display
- **Ctrl+Alt+2** - Switch to QEMU monitor

### QEMU Monitor Commands

Press `Ctrl+A, C` in serial mode to access the monitor:

- `info registers` - Show CPU state
- `info mem` - Show memory mappings
- `info mtree` - Show memory tree
- `q` or `quit` - Exit QEMU
- `c` - Continue execution

## Debugging

### Enable Debug Output

```bash
./run-qemu.sh debug
```

Shows:
- CPU resets
- Interrupts (INT, exceptions)
- Triple faults
- Doesn't auto-reboot on crash

### GDB Debugging

**Terminal 1 - Start QEMU:**
```bash
./run-qemu.sh gdb
# QEMU pauses, waiting for GDB
```

**Terminal 2 - Connect GDB:**
```bash
rust-gdb target/x86_64-aethelos/debug/heartwood

# In GDB:
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
(gdb) step
(gdb) info registers
```

### Check for Errors

**Triple Fault (reboot loop):**
```bash
# Run with debug mode to see what's happening
./run-qemu.sh debug

# Common causes:
# - Stack overflow
# - Invalid interrupt handler
# - Page fault
# - Divide by zero
```

**Black Screen:**
```bash
# Try serial console instead
./run-qemu.sh serial

# VGA buffer might not be initialized correctly
```

**QEMU won't start:**
```bash
# Check QEMU is installed
qemu-system-x86_64 --version

# Windows: Check PATH or QEMU install location
```

## Current Limitations

AethelOS is in early development. Currently:

✓ **Working:**
- Boots in QEMU via Multiboot2
- VGA text output
- Kernel initialization
- Memory management (structure)
- Scheduler (structure)

❌ **Not Yet Implemented:**
- Keyboard input (no interaction yet)
- Interrupts/timers
- Actual memory allocation
- Thread execution
- User-space processes
- Filesystem
- Networking

## Next Steps - Adding Interaction

To make AethelOS interactive, you'll need to implement:

### 1. Keyboard Input
```rust
// In heartwood/src/attunement/keyboard.rs
pub fn read_key() -> Option<char> {
    // Read from PS/2 keyboard port 0x60
}
```

### 2. Simple Shell
```rust
// In heartwood/src/main.rs
loop {
    if let Some(key) = keyboard::read_key() {
        shell::handle_key(key);
    }
}
```

### 3. Basic Commands
```rust
// Commands to implement:
// - harmony    -> Show system harmony metrics
// - threads    -> List all threads
// - memory     -> Show memory statistics
// - help       -> Show available commands
```

## Advanced QEMU Options

### More Memory
```bash
qemu-system-x86_64 -kernel heartwood -m 512M
```

### Multiple CPUs
```bash
qemu-system-x86_64 -kernel heartwood -smp 4
```

### Enable KVM (Linux only - faster)
```bash
qemu-system-x86_64 -kernel heartwood -enable-kvm
```

### Serial + VGA
```bash
qemu-system-x86_64 \
  -kernel heartwood \
  -vga std \
  -serial mon:stdio \
  -m 256M
```

### Save Screenshots
```bash
# In QEMU monitor (Ctrl+Alt+2)
screendump screenshot.ppm
```

## Rebuilding

After making code changes:

```bash
# Clean build
cargo clean

# Rebuild
./build.sh         # Linux/macOS
.\build.ps1        # Windows

# Run
./run-qemu.sh
```

## Troubleshooting

### Build Errors

**"error: no global memory allocator"**
- Fixed in current code - has DummyAllocator

**"error: language item required, but not found: eh_personality"**
- Make sure `panic = "abort"` is in Cargo.toml
- Check you're using `no_std`

**"error: linking with link.exe failed"**
- This is expected for Windows host builds
- The kernel binary still works fine
- QEMU loads it directly, doesn't need Windows linking

### Runtime Errors

**QEMU shows "No bootable device"**
- Multiboot header missing or malformed
- Run `setup-boot.sh` again
- Check `heartwood/src/boot.rs` exists

**Triple fault immediately**
- Stack overflow - increase stack in linker.ld
- Bad interrupt descriptor table
- Run with `./run-qemu.sh debug` to see details

**Kernel loads but black screen**
- VGA buffer not initialized
- Try serial mode: `./run-qemu.sh serial`
- Check `vga_buffer::initialize()` is called

## Files Created by Setup

```
aethelos-source/
├── x86_64-aethelos.json       # Custom target spec
├── .cargo/
│   └── config.toml            # Build configuration
├── heartwood/
│   ├── linker.ld             # Linker script
│   └── src/
│       └── boot.rs           # Multiboot2 header
├── build.sh / build.ps1      # Build scripts
└── run-qemu.sh / run-qemu.ps1 # Run scripts
```

## Resources

- **Full Documentation**: See `RUNNING.md`
- **Architecture**: See `DESIGN.md`
- **Philosophy**: See `GENESIS.scroll`
- **Rust OSDev**: https://os.phil-opp.com/
- **OSDev Wiki**: https://wiki.osdev.org/
- **QEMU Manual**: https://www.qemu.org/docs/master/

## Getting Help

If you encounter issues:

1. Check `RUNNING.md` for detailed troubleshooting
2. Run with debug mode: `./run-qemu.sh debug`
3. Check build output for errors
4. Verify QEMU version: `qemu-system-x86_64 --version`
5. Ensure Rust nightly is installed: `rustup toolchain list`

---

**Ready to boot?**

```bash
./setup-boot.sh && ./build.sh && ./run-qemu.sh
```

Welcome to AethelOS - where **Symbiotic Computing** comes alive! 🌳
