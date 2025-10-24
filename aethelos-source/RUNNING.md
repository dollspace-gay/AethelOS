# Running AethelOS in QEMU

This guide explains how to build and run AethelOS in QEMU.

## Current Status

AethelOS is currently in **early development**. The kernel libraries compile successfully, but the bootloader infrastructure needs to be completed before it can run in QEMU.

## Prerequisites

### Required Tools

1. **Rust** (stable 1.90+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default stable
   ```

2. **QEMU** (for x86_64)
   - **Windows**: Download from https://qemu.weilnetz.de/w64/
   - **Linux**: `sudo apt install qemu-system-x86`
   - **macOS**: `brew install qemu`

3. **Rust cargo-binutils** (for creating bootable images)
   ```bash
   cargo install cargo-binutils
   rustup component add llvm-tools-preview
   ```

## What Needs to Be Completed

To make AethelOS bootable, the following components need implementation:

### 1. Custom Target Specification

Create `x86_64-aethelos.json`:
```json
{
  "llvm-target": "x86_64-unknown-none",
  "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128",
  "arch": "x86_64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "os": "none",
  "executables": true,
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "panic-strategy": "abort",
  "disable-redzone": true,
  "features": "-mmx,-sse,+soft-float"
}
```

### 2. Linker Script

Create `heartwood/linker.ld`:
```ld
ENTRY(_start)

SECTIONS {
    . = 1M;

    .boot :
    {
        KEEP(*(.multiboot))
    }

    .text :
    {
        *(.text .text.*)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
    }

    .data :
    {
        *(.data .data.*)
    }

    .bss :
    {
        *(.bss .bss.*)
    }

    /DISCARD/ :
    {
        *(.eh_frame)
    }
}
```

### 3. Multiboot2 Header

Add to `heartwood/src/boot.rs`:
```rust
#[repr(C, align(8))]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // End tag
    end_tag: [u32; 2],
}

#[used]
#[link_section = ".multiboot"]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header {
    magic: 0xe85250d6,
    architecture: 0, // i386
    header_length: core::mem::size_of::<Multiboot2Header>() as u32,
    checksum: 0u32.wrapping_sub(
        0xe85250d6 + 0 + core::mem::size_of::<Multiboot2Header>() as u32
    ),
    end_tag: [0, 8],
};
```

### 4. Cargo Build Configuration

Create `.cargo/config.toml`:
```toml
[build]
target = "x86_64-aethelos.json"

[target.'cfg(target_os = "none")']
runner = "bootimage runner"

[unstable]
build-std = ["core", "alloc"]
build-std-features = ["compiler-builtins-mem"]
```

### 5. Alternative: Use bootloader crate

The easier approach is to use the `bootloader` crate:

Add to `heartwood/Cargo.toml`:
```toml
[dependencies]
bootloader = "0.9"
```

Update `Cargo.toml` root:
```toml
[dependencies]
bootimage = "0.10"
```

## Building (Once Bootloader is Complete)

### Option A: Using bootloader crate
```bash
# Install bootimage
cargo install bootimage

# Build the kernel
cd heartwood
cargo bootimage

# The bootable image is created at:
# target/x86_64-aethelos/debug/bootimage-heartwood.bin
```

### Option B: Manual build with linker script
```bash
# Build for custom target
cargo build --target x86_64-aethelos.json

# Create bootable ISO
mkdir -p isofiles/boot/grub
cp target/x86_64-aethelos/debug/heartwood isofiles/boot/heartwood

# Create GRUB config
cat > isofiles/boot/grub/grub.cfg << EOF
menuentry "AethelOS" {
    multiboot2 /boot/heartwood
    boot
}
EOF

# Create ISO
grub-mkrescue -o aethelos.iso isofiles
```

## Running in QEMU (Once Bootable)

### Basic Boot
```bash
qemu-system-x86_64 -drive format=raw,file=bootimage-heartwood.bin
```

### With Serial Console Output
```bash
qemu-system-x86_64 \
  -drive format=raw,file=bootimage-heartwood.bin \
  -serial stdio \
  -display none
```

### With VGA Display (Recommended)
```bash
qemu-system-x86_64 \
  -drive format=raw,file=bootimage-heartwood.bin \
  -vga std \
  -serial mon:stdio
```

### With Debugging
```bash
qemu-system-x86_64 \
  -drive format=raw,file=bootimage-heartwood.bin \
  -serial stdio \
  -d int,cpu_reset \
  -no-reboot \
  -no-shutdown
```

### Full Featured Run
```bash
qemu-system-x86_64 \
  -drive format=raw,file=bootimage-heartwood.bin \
  -m 256M \
  -vga std \
  -serial mon:stdio \
  -cpu max \
  -smp 2 \
  -enable-kvm  # Linux only, for better performance
```

## Expected Output (When Running)

When AethelOS successfully boots, you should see:

```
[] Awakening the Heartwood...
[] Kindling the Mana Pool...
[] Opening the Nexus...
[] Weaving the Loom of Fate...
[] Attuning to the hardware...
[] The Heartwood lives!

[AethelOS Banner displayed via VGA buffer]
```

## Interacting with AethelOS

Currently, AethelOS enters an idle loop after initialization. To add interaction:

### 1. Keyboard Input
Add keyboard driver in `attunement/keyboard.rs` to read PS/2 keyboard events.

### 2. Serial Console
Enable UART 16550 serial driver for text-based interaction:
```rust
// In attunement/mod.rs
pub mod serial;

// Add serial commands
pub fn serial_read() -> Option<u8>;
pub fn serial_write(byte: u8);
```

### 3. Shell Interface
Implement the Eldarin Shell (placeholder in `ancient-runes/script`):
- Parse commands
- Execute via Nexus IPC
- Display results in VGA buffer

## QEMU Monitor Commands

When running with `-serial mon:stdio`, you can use:

- `Ctrl+A, C` - Switch to QEMU monitor
- `info registers` - Show CPU state
- `info mem` - Show memory mappings
- `info mtree` - Show memory tree
- `q` or `quit` - Exit QEMU

## Debugging with GDB

Terminal 1 - Start QEMU with GDB server:
```bash
qemu-system-x86_64 \
  -drive format=raw,file=bootimage-heartwood.bin \
  -s -S
```

Terminal 2 - Connect GDB:
```bash
rust-gdb target/x86_64-aethelos/debug/heartwood
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

## Troubleshooting

### "No bootable device" Error
- Ensure multiboot2 header is properly included
- Check linker script places `.multiboot` section first
- Verify GRUB can find the kernel

### Black Screen
- VGA buffer initialization may have failed
- Try serial output instead: `-serial stdio -display none`
- Check panic handler is being called

### Triple Fault / Reboot Loop
- Stack overflow (increase stack size in linker script)
- Invalid interrupt descriptor table
- Page fault due to incorrect memory mapping
- Use QEMU debugging: `-d int,cpu_reset -no-reboot`

### Build Errors
- Ensure using `--target x86_64-aethelos.json`
- Check all `no_std` crates are compatible
- Verify `build-std` is enabled in `.cargo/config.toml`

## Next Steps for Full Bootability

1. **Implement Multiboot2 Support**
   - Add multiboot2 header
   - Parse bootloader information
   - Set up initial page tables

2. **Complete Hardware Initialization**
   - Set up IDT (Interrupt Descriptor Table)
   - Configure GDT (Global Descriptor Table)
   - Enable hardware interrupts

3. **Implement Memory Management**
   - Initialize physical memory manager
   - Set up heap allocator with actual memory region
   - Complete Sanctuary and Ephemeral Mist allocators

4. **Add Device Drivers**
   - VGA text mode (currently placeholder)
   - PS/2 keyboard
   - Serial UART (for debugging)
   - Timer (PIT or APIC)

5. **Enable User Interaction**
   - Implement keyboard input handler
   - Create simple shell (Eldarin)
   - Add basic commands (harmony, threads, memory stats)

## Resources

- **Rust OSDev**: https://os.phil-opp.com/
- **OSDev Wiki**: https://wiki.osdev.org/
- **QEMU Documentation**: https://www.qemu.org/docs/master/
- **Multiboot2 Spec**: https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html

## Current Development Status

✓ **Completed:**
- Kernel library architecture
- Capability-based memory management
- Harmony-based cooperative scheduler
- Nexus IPC system (structure)
- Basic VGA buffer (placeholder)

⚠ **In Progress:**
- Bootloader integration
- Hardware initialization
- Device drivers

❌ **Not Yet Started:**
- User-space services (Groves)
- Filesystem (World-Tree)
- GUI (The Weave, Lanthir)
- Network stack (Network Sprite)

---

**Note:** AethelOS is an experimental operating system focused on "Symbiotic Computing" philosophy. The current code represents the architectural foundation. Full QEMU support requires completing the bootloader and hardware initialization layers.
