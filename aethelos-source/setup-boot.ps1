# Setup script to make AethelOS bootable in QEMU (Windows PowerShell)

Write-Host "=== AethelOS Boot Setup ===" -ForegroundColor Cyan
Write-Host ""

# Create custom target specification
Write-Host "[1/6] Creating custom target specification..." -ForegroundColor Yellow
$targetSpec = @'
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
'@
$targetSpec | Out-File -FilePath "x86_64-aethelos.json" -Encoding ASCII

# Create linker script
Write-Host "[2/6] Creating linker script..." -ForegroundColor Yellow
$linkerScript = @'
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
'@
$linkerScript | Out-File -FilePath "heartwood\linker.ld" -Encoding ASCII

# Create boot.rs
Write-Host "[3/6] Creating boot module with Multiboot2 header..." -ForegroundColor Yellow
$bootRs = @'
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
'@
$bootRs | Out-File -FilePath "heartwood\src\boot.rs" -Encoding UTF8

# Create cargo config
Write-Host "[4/6] Creating cargo configuration..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path ".cargo" | Out-Null
$cargoConfig = @'
[build]
target = "x86_64-aethelos.json"

[target.'cfg(target_os = "none")']
rustflags = ["-C", "link-arg=-T./heartwood/linker.ld"]

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
build-std-features = ["compiler-builtins-mem"]
'@
$cargoConfig | Out-File -FilePath ".cargo\config.toml" -Encoding ASCII

# Update lib.rs
Write-Host "[5/6] Adding boot module to heartwood..." -ForegroundColor Yellow
$libRsPath = "heartwood\src\lib.rs"
$libRsContent = Get-Content $libRsPath -Raw
if ($libRsContent -notmatch "pub mod boot;") {
    $newContent = "pub mod boot;`r`n" + $libRsContent
    $newContent | Out-File -FilePath $libRsPath -Encoding UTF8 -NoNewline
}

# Create run script
Write-Host "[6/6] Creating QEMU run script..." -ForegroundColor Yellow
$runScript = @'
# Run AethelOS in QEMU (Windows PowerShell)

param(
    [ValidateSet("vga", "serial", "debug", "gdb")]
    [string]$Mode = "vga"
)

# Check if binary exists
if (-not (Test-Path "target\x86_64-aethelos\debug\heartwood.exe")) {
    Write-Host "Error: Kernel binary not found. Run '.\build.ps1' first." -ForegroundColor Red
    exit 1
}

# Find QEMU
$qemu = $null
$qemuPaths = @(
    "C:\Program Files\qemu\qemu-system-x86_64.exe",
    "C:\Program Files (x86)\qemu\qemu-system-x86_64.exe",
    "$env:ProgramFiles\qemu\qemu-system-x86_64.exe"
)

foreach ($path in $qemuPaths) {
    if (Test-Path $path) {
        $qemu = $path
        break
    }
}

if (-not $qemu) {
    # Try to find in PATH
    $qemu = (Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue).Source
}

if (-not $qemu) {
    Write-Host "Error: QEMU not found. Install from https://qemu.weilnetz.de/w64/" -ForegroundColor Red
    exit 1
}

switch ($Mode) {
    "vga" {
        Write-Host "Running AethelOS with VGA display..." -ForegroundColor Green
        & $qemu `
            -kernel target\x86_64-aethelos\debug\heartwood.exe `
            -m 256M `
            -vga std `
            -serial mon:stdio
    }
    "serial" {
        Write-Host "Running AethelOS with serial output..." -ForegroundColor Green
        & $qemu `
            -kernel target\x86_64-aethelos\debug\heartwood.exe `
            -m 256M `
            -serial stdio `
            -display none
    }
    "debug" {
        Write-Host "Running AethelOS in debug mode..." -ForegroundColor Green
        & $qemu `
            -kernel target\x86_64-aethelos\debug\heartwood.exe `
            -m 256M `
            -vga std `
            -serial mon:stdio `
            -d int,cpu_reset `
            -no-reboot `
            -no-shutdown
    }
    "gdb" {
        Write-Host "Running AethelOS with GDB server on :1234..." -ForegroundColor Green
        Write-Host "In another terminal, run: rust-gdb target\x86_64-aethelos\debug\heartwood.exe" -ForegroundColor Cyan
        Write-Host "Then in GDB: (gdb) target remote :1234" -ForegroundColor Cyan
        & $qemu `
            -kernel target\x86_64-aethelos\debug\heartwood.exe `
            -m 256M `
            -vga std `
            -serial mon:stdio `
            -s -S
    }
}
'@
$runScript | Out-File -FilePath "run-qemu.ps1" -Encoding ASCII

# Create build script
$buildScript = @'
# Build AethelOS kernel (Windows PowerShell)

Write-Host "=== Building AethelOS Kernel ===" -ForegroundColor Cyan

# Check for cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: cargo not found. Install Rust from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Check for nightly toolchain
$toolchains = rustup toolchain list
if ($toolchains -notmatch "nightly") {
    Write-Host "Installing nightly toolchain for build-std..." -ForegroundColor Yellow
    rustup toolchain install nightly
}

# Add rust-src component
$components = rustup component list --toolchain nightly
if ($components -notmatch "rust-src.*installed") {
    Write-Host "Adding rust-src component..." -ForegroundColor Yellow
    rustup component add rust-src --toolchain nightly
}

# Build the kernel
Write-Host ""
Write-Host "Building heartwood kernel..." -ForegroundColor Yellow
cargo +nightly build --package heartwood --bin heartwood

Write-Host ""
Write-Host "✓ Build complete!" -ForegroundColor Green
Write-Host "  Binary: target\x86_64-aethelos\debug\heartwood.exe" -ForegroundColor Cyan
Write-Host ""
Write-Host "To run in QEMU:" -ForegroundColor Cyan
Write-Host "  .\run-qemu.ps1          # VGA display mode"
Write-Host "  .\run-qemu.ps1 serial   # Serial console mode"
Write-Host "  .\run-qemu.ps1 debug    # Debug mode"
Write-Host "  .\run-qemu.ps1 gdb      # GDB debugging"
'@
$buildScript | Out-File -FilePath "build.ps1" -Encoding ASCII

Write-Host ""
Write-Host "=== Setup Complete! ===" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Run: .\build.ps1          # Build the kernel"
Write-Host "  2. Run: .\run-qemu.ps1       # Boot in QEMU"
Write-Host ""
Write-Host "QEMU modes:" -ForegroundColor Cyan
Write-Host "  .\run-qemu.ps1 vga      # VGA display (default)"
Write-Host "  .\run-qemu.ps1 serial   # Serial console only"
Write-Host "  .\run-qemu.ps1 debug    # Debug mode with logging"
Write-Host "  .\run-qemu.ps1 gdb      # GDB debugging on :1234"
Write-Host ""
Write-Host "Note: You'll need QEMU installed" -ForegroundColor Yellow
Write-Host "  Download from: https://qemu.weilnetz.de/w64/"
