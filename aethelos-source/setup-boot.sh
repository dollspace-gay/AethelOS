#!/bin/bash
# Setup script to make AethelOS bootable in QEMU

set -e

echo "=== AethelOS Boot Setup ==="
echo ""

# Create custom target specification
echo "[1/6] Creating custom target specification..."
cat > x86_64-aethelos.json << 'EOF'
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
EOF

# Create linker script
echo "[2/6] Creating linker script..."
cat > heartwood/linker.ld << 'EOF'
ENTRY(_start)

SECTIONS {
    /* Start at 1MB to leave space for bootloader */
    . = 1M;

    /* Multiboot header must be first */
    .boot ALIGN(4K) :
    {
        KEEP(*(.multiboot))
    }

    .text ALIGN(4K) :
    {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) :
    {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) :
    {
        *(.data .data.*)
    }

    .bss ALIGN(4K) :
    {
        *(COMMON)
        *(.bss .bss.*)
    }

    /* Discard unwanted sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.note.gnu.build-id)
    }
}
EOF

# Create boot.rs with multiboot2 header
echo "[3/6] Creating boot module with Multiboot2 header..."
cat > heartwood/src/boot.rs << 'EOF'
//! Boot initialization and Multiboot2 support

/// Multiboot2 header structure
#[repr(C, align(8))]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // End tag
    end_type: u16,
    end_flags: u16,
    end_size: u32,
}

const MULTIBOOT2_MAGIC: u32 = 0xe85250d6;
const MULTIBOOT2_ARCH_I386: u32 = 0;
const HEADER_LENGTH: u32 = core::mem::size_of::<Multiboot2Header>() as u32;

/// The Multiboot2 header - must be in first 8KB of kernel
#[used]
#[link_section = ".multiboot"]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MULTIBOOT2_MAGIC,
    architecture: MULTIBOOT2_ARCH_I386,
    header_length: HEADER_LENGTH,
    checksum: 0u32
        .wrapping_sub(MULTIBOOT2_MAGIC)
        .wrapping_sub(MULTIBOOT2_ARCH_I386)
        .wrapping_sub(HEADER_LENGTH),
    end_type: 0,
    end_flags: 0,
    end_size: 8,
};

/// Parse Multiboot2 information structure
pub unsafe fn parse_multiboot_info(_multiboot_info_addr: usize) {
    // In a real implementation, this would:
    // 1. Parse memory map
    // 2. Get framebuffer info
    // 3. Find loaded modules
    // 4. Get bootloader name
    // For now, this is a placeholder
}
EOF

# Create cargo config
echo "[4/6] Creating cargo configuration..."
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[build]
target = "x86_64-aethelos.json"

[target.'cfg(target_os = "none")']
rustflags = ["-C", "link-arg=-T./heartwood/linker.ld"]

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
build-std-features = ["compiler-builtins-mem"]
EOF

# Update heartwood lib.rs to include boot module
echo "[5/6] Adding boot module to heartwood..."
if ! grep -q "pub mod boot;" heartwood/src/lib.rs; then
    sed -i '1i pub mod boot;' heartwood/src/lib.rs 2>/dev/null || \
    gsed -i '1i pub mod boot;' heartwood/src/lib.rs 2>/dev/null || \
    echo "Warning: Could not auto-add boot module. Add 'pub mod boot;' to heartwood/src/lib.rs manually."
fi

# Create run script
echo "[6/6] Creating QEMU run script..."
cat > run-qemu.sh << 'EOF'
#!/bin/bash
# Run AethelOS in QEMU

# Check if binary exists
if [ ! -f "target/x86_64-aethelos/debug/heartwood" ]; then
    echo "Error: Kernel binary not found. Run './build.sh' first."
    exit 1
fi

# Default to VGA mode
MODE=${1:-vga}

case $MODE in
    vga)
        echo "Running AethelOS with VGA display..."
        qemu-system-x86_64 \
            -kernel target/x86_64-aethelos/debug/heartwood \
            -m 256M \
            -vga std \
            -serial mon:stdio
        ;;
    serial)
        echo "Running AethelOS with serial output..."
        qemu-system-x86_64 \
            -kernel target/x86_64-aethelos/debug/heartwood \
            -m 256M \
            -serial stdio \
            -display none
        ;;
    debug)
        echo "Running AethelOS in debug mode..."
        qemu-system-x86_64 \
            -kernel target/x86_64-aethelos/debug/heartwood \
            -m 256M \
            -vga std \
            -serial mon:stdio \
            -d int,cpu_reset \
            -no-reboot \
            -no-shutdown
        ;;
    gdb)
        echo "Running AethelOS with GDB server on :1234..."
        echo "In another terminal, run: rust-gdb target/x86_64-aethelos/debug/heartwood"
        echo "Then in GDB: (gdb) target remote :1234"
        qemu-system-x86_64 \
            -kernel target/x86_64-aethelos/debug/heartwood \
            -m 256M \
            -vga std \
            -serial mon:stdio \
            -s -S
        ;;
    *)
        echo "Usage: $0 [vga|serial|debug|gdb]"
        echo ""
        echo "  vga    - Run with VGA display (default)"
        echo "  serial - Run with serial console only"
        echo "  debug  - Run with debugging output"
        echo "  gdb    - Run with GDB server"
        exit 1
        ;;
esac
EOF
chmod +x run-qemu.sh

# Create build script
cat > build.sh << 'EOF'
#!/bin/bash
# Build AethelOS kernel

set -e

echo "=== Building AethelOS Kernel ==="

# Check for required tools
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Install Rust from https://rustup.rs/"
    exit 1
fi

# Check for nightly toolchain (needed for build-std)
if ! rustup toolchain list | grep -q nightly; then
    echo "Installing nightly toolchain for build-std..."
    rustup toolchain install nightly
fi

# Add rust-src component if not present
if ! rustup component list --toolchain nightly | grep -q "rust-src (installed)"; then
    echo "Adding rust-src component..."
    rustup component add rust-src --toolchain nightly
fi

# Build the kernel
echo ""
echo "Building heartwood kernel..."
cargo +nightly build --package heartwood --bin heartwood

echo ""
echo "✓ Build complete!"
echo "  Binary: target/x86_64-aethelos/debug/heartwood"
echo ""
echo "To run in QEMU:"
echo "  ./run-qemu.sh          # VGA display mode"
echo "  ./run-qemu.sh serial   # Serial console mode"
echo "  ./run-qemu.sh debug    # Debug mode"
echo "  ./run-qemu.sh gdb      # GDB debugging"
EOF
chmod +x build.sh

echo ""
echo "=== Setup Complete! ==="
echo ""
echo "Next steps:"
echo "  1. Run: ./build.sh          # Build the kernel"
echo "  2. Run: ./run-qemu.sh       # Boot in QEMU"
echo ""
echo "QEMU modes:"
echo "  ./run-qemu.sh vga      # VGA display (default)"
echo "  ./run-qemu.sh serial   # Serial console only"
echo "  ./run-qemu.sh debug    # Debug mode with logging"
echo "  ./run-qemu.sh gdb      # GDB debugging on :1234"
echo ""
echo "Note: You'll need QEMU installed (qemu-system-x86_64)"
echo "  Windows: https://qemu.weilnetz.de/w64/"
echo "  Linux:   sudo apt install qemu-system-x86"
echo "  macOS:   brew install qemu"
