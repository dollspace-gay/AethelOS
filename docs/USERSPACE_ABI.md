# AethelOS Userspace ABI Documentation

> **Status:** Draft - for Runic Forge and future userspace programs
> **Target Spec:** `x86_64-aethelos-userspace.json`
> **Last Updated:** January 2025

---

## Overview

This document defines the Application Binary Interface (ABI) for userspace programs running on AethelOS, including Ring 1 services (like Runic Forge) and Ring 3 applications.

The ABI covers:
- Function calling conventions
- Syscall interface
- Memory layout
- Linking requirements
- Build instructions

---

## Target Specification

### Using the Target Spec

The target specification file `x86_64-aethelos-userspace.json` enables cross-compilation of Rust programs for AethelOS.

**Location:** `f:\OS\x86_64-aethelos-userspace.json`

**To build a userspace program:**

```bash
# Build with the AethelOS target
cargo build --target x86_64-aethelos-userspace.json --release

# Or use a .cargo/config.toml:
[build]
target = "x86_64-aethelos-userspace.json"
```

### Key Target Features

| Feature | Value | Rationale |
|---------|-------|-----------|
| **OS** | `aethelos` | Distinguishes from bare-metal (`none`) |
| **Environment** | `userspace` | Distinguishes from kernel |
| **Code Model** | `small` | Suitable for userspace programs (<2GB code) |
| **Relocation** | `pic` | Position-independent code for ASLR |
| **PIE** | `true` | Position-independent executables |
| **Float** | `soft-float` | Avoid x87 FPU state management |
| **Panic** | `abort` | No unwinding support yet |
| **Stack Protector** | Supported | LLVM stack canaries enabled |

---

## Function Calling Convention

AethelOS follows the **System V AMD64 ABI** (standard for x86-64 Unix-like systems).

### Register Usage

| Register | Purpose | Preserved Across Calls | Notes |
|----------|---------|----------------------|-------|
| **RAX** | Return value, syscall number | No | |
| **RBX** | Callee-saved | Yes | |
| **RCX** | 4th argument | No | Clobbered by `syscall` |
| **RDX** | 3rd argument | No | |
| **RSI** | 2nd argument | No | |
| **RDI** | 1st argument | No | |
| **RBP** | Frame pointer | Yes | |
| **RSP** | Stack pointer | Yes (but adjusted) | Must be 16-byte aligned |
| **R8** | 5th argument | No | |
| **R9** | 6th argument | No | |
| **R10** | Temporary | No | Used for 4th syscall arg |
| **R11** | Temporary | No | Clobbered by `syscall` |
| **R12-R15** | Callee-saved | Yes | |

### Argument Passing

**For regular function calls:**

1. First 6 integer/pointer arguments: RDI, RSI, RDX, RCX, R8, R9
2. Additional arguments: pushed on stack (right-to-left)
3. Return value: RAX (or RAX:RDX for 128-bit returns)

**Example:**

```rust
// fn example(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) -> u64
// a = RDI, b = RSI, c = RDX, d = RCX, e = R8, f = R9
// Return value in RAX
```

### Stack Alignment

**CRITICAL:** Stack pointer (RSP) must be:
- **16-byte aligned before `call` instructions**
- This means `RSP % 16 == 8` immediately before `call` (because `call` pushes 8-byte return address)

**Why:** SSE instructions (movaps, etc.) require 16-byte alignment. Stack misalignment causes #GP faults.

```rust
// Correct stack setup
#[repr(C, align(16))]
struct Stack {
    data: [u8; STACK_SIZE],
}

// Initialize RSP to &stack[STACK_SIZE - 8] so it's 16n-8 aligned
```

---

## Syscall Interface

AethelOS uses the modern **syscall/sysret** mechanism (not INT 0x80).

### Syscall Calling Convention

**Registers on syscall entry:**

| Register | Purpose | Preserved |
|----------|---------|-----------|
| **RAX** | Syscall number | No (return value) |
| **RDI** | Argument 1 | No |
| **RSI** | Argument 2 | No |
| **RDX** | Argument 3 | No |
| **R10** | Argument 4 (NOT RCX!) | No |
| **R8** | Argument 5 | No |
| **R9** | Argument 6 | No |
| **RCX** | (Clobbered - return RIP) | No |
| **R11** | (Clobbered - RFLAGS) | No |
| **All other registers** | Preserved by kernel | Yes |

**Return value:**
- **RAX >= 0:** Success (return value)
- **RAX < 0:** Error code (negative errno)

### Syscall Numbers

```rust
// Defined in ancient-runes/corelib/src/syscalls.rs (to be created)
pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_OPEN: u64 = 3;
pub const SYS_CLOSE: u64 = 4;
// ... more syscalls to be defined
```

### Userspace Wrapper Functions

**Helper functions for making syscalls from Rust:**

```rust
#[inline(always)]
unsafe fn syscall0(num: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        lateout("rax") ret,
        out("rcx") _,   // Clobbered
        out("r11") _,   // Clobbered
        options(nostack, preserves_flags)
    );
    ret
}

#[inline(always)]
unsafe fn syscall1(num: u64, arg1: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline(always)]
unsafe fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}

// Example high-level wrapper
pub fn sys_write(fd: i32, buf: &[u8]) -> Result<usize, i32> {
    let ret = unsafe {
        syscall3(SYS_WRITE, fd as u64, buf.as_ptr() as u64, buf.len() as u64)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(ret as usize)
    }
}

pub fn sys_exit(code: i32) -> ! {
    unsafe { syscall1(SYS_EXIT, code as u64) };
    unreachable!("sys_exit returned!");
}
```

**Full documentation:** See [SYSCALL_SYSRET_IMPLEMENTATION.md](SYSCALL_SYSRET_IMPLEMENTATION.md)

---

## Memory Layout

### Virtual Address Space Layout

```
┌─────────────────────────────────────┐ 0xFFFF_FFFF_FFFF_FFFF
│         Kernel Space                │ (Inaccessible from userspace)
│         (Higher Half)               │
├─────────────────────────────────────┤ 0xFFFF_8000_0000_0000
│         Non-canonical               │ (Invalid addresses)
├─────────────────────────────────────┤ 0x0000_7FFF_FFFF_FFFF
│                                     │
│         User Heap (grows up)        │
│                                     │
├─────────────────────────────────────┤ (Dynamic)
│                                     │
│         .bss (uninitialized data)   │
│         .data (initialized data)    │
│         .rodata (read-only data)    │
│         .text (code)                │
├─────────────────────────────────────┤ (PIE base, randomized)
│         Reserved                    │
├─────────────────────────────────────┤ 0x0000_0000_1000_0000 (256 MB)
│         Stack (grows down)          │
├─────────────────────────────────────┤ (Stack top, varies)
│         Guard Page                  │
├─────────────────────────────────────┤
│         Reserved/Null Page          │
└─────────────────────────────────────┘ 0x0000_0000_0000_0000
```

### Memory Regions

| Region | Typical Range | Permissions | Notes |
|--------|--------------|-------------|-------|
| **Null Page** | 0x0000 - 0x0FFF | None | Trap null pointer dereferences |
| **Code (.text)** | Variable (PIE) | R-X | Position-independent |
| **Data (.rodata)** | After .text | R-- | Read-only data |
| **Data (.data)** | After .rodata | RW- | Initialized globals |
| **BSS (.bss)** | After .data | RW- | Zero-initialized |
| **Heap** | After .bss | RW- | Managed by allocator |
| **Stack** | Near 256 MB | RW- | Fixed size, grows down |

**ASLR:** The PIE base is randomized by the kernel on load (when KASLR is enabled).

---

## Linking Requirements

### Linker Script

Userspace programs require a custom linker script. Create `userspace.ld`:

```ld
/* AethelOS Userspace Linker Script */

ENTRY(_start)

SECTIONS
{
    /* PIE base address (will be relocated by kernel) */
    . = 0x400000;

    .text : ALIGN(4K)
    {
        KEEP(*(.text._start))  /* Entry point first */
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    .bss : ALIGN(4K)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    /* Discard unwanted sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.comment)
    }
}
```

**To use in Cargo project:**

```toml
# .cargo/config.toml
[target.x86_64-aethelos-userspace]
rustflags = ["-C", "link-arg=-T../userspace.ld"]
```

### Entry Point

**All userspace programs must define `_start`:**

```rust
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize runtime (if any)
    // Call main()
    // Exit with return code

    sys_exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Log panic to serial or display
    sys_exit(1);
}
```

---

## Build Example: Minimal Userspace Program

### Project Structure

```
my_program/
├── Cargo.toml
├── .cargo/
│   └── config.toml
├── userspace.ld
└── src/
    └── main.rs
```

### Cargo.toml

```toml
[package]
name = "my_program"
version = "0.1.0"
edition = "2021"

[dependencies]
# No std library available yet

[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
strip = true           # Strip debug symbols
panic = "abort"        # No unwinding
```

### .cargo/config.toml

```toml
[build]
target = "../x86_64-aethelos-userspace.json"

[target.x86_64-aethelos-userspace]
rustflags = [
    "-C", "link-arg=-T../userspace.ld",
    "-C", "relocation-model=pic",
]
```

### src/main.rs

```rust
#![no_std]
#![no_main]

// Syscall wrappers
unsafe fn syscall1(num: u64, arg1: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
    );
    ret
}

unsafe fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
    );
    ret
}

const SYS_EXIT: u64 = 0;
const SYS_WRITE: u64 = 1;

fn sys_write(fd: i32, buf: &[u8]) -> Result<usize, i32> {
    let ret = unsafe {
        syscall3(SYS_WRITE, fd as u64, buf.as_ptr() as u64, buf.len() as u64)
    };
    if ret < 0 {
        Err(ret as i32)
    } else {
        Ok(ret as usize)
    }
}

fn sys_exit(code: i32) -> ! {
    unsafe { syscall1(SYS_EXIT, code as u64) };
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from AethelOS userspace!\n";

    match sys_write(1, msg) {
        Ok(n) => {
            // Successfully wrote n bytes
        }
        Err(_) => {
            // Write failed
            sys_exit(1);
        }
    }

    sys_exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sys_exit(1);
}
```

### Build Commands

```bash
# Build the program
cargo build --release

# The binary will be at:
# target/x86_64-aethelos-userspace/release/my_program

# Inspect the binary
rust-objdump -d target/x86_64-aethelos-userspace/release/my_program
rust-readelf -h target/x86_64-aethelos-userspace/release/my_program

# Convert to raw binary (if needed)
rust-objcopy --strip-all \
    -O binary \
    target/x86_64-aethelos-userspace/release/my_program \
    my_program.bin
```

---

## Ring 1 vs Ring 3 Differences

### Ring 1 Services (Groves)

Ring 1 services like **Runic Forge** run with elevated privileges:

- **Code Segment:** 0x28 (Ring 1 code segment with RPL=1)
- **Data Segment:** 0x30 (Ring 1 data segment with RPL=1)
- **Capabilities:** Can access specific hardware (I/O ports, physical memory) via Grove Manager
- **Memory Access:** Can use privileged instructions (with capabilities)
- **Loading:** Loaded via `groves::load_service()` function

### Ring 3 Applications

Regular userspace applications run unprivileged:

- **Code Segment:** 0x20 (Ring 3 code segment with RPL=3)
- **Data Segment:** 0x18 (Ring 3 data segment with RPL=3)
- **Capabilities:** No special permissions, pure capability-based access
- **Memory Access:** Cannot access I/O ports or privileged instructions
- **Loading:** Loaded via ELF loader (to be implemented)

**ABI is the same for both** - only privilege level and capabilities differ.

---

## Error Codes

Syscalls return negative error codes on failure:

```rust
// Standard POSIX-like error codes
pub const EPERM: i32 = -1;      // Operation not permitted
pub const ENOENT: i32 = -2;     // No such file or directory
pub const ESRCH: i32 = -3;      // No such process
pub const EINTR: i32 = -4;      // Interrupted system call
pub const EIO: i32 = -5;        // I/O error
pub const EBADF: i32 = -9;      // Bad file descriptor
pub const ENOMEM: i32 = -12;    // Out of memory
pub const EACCES: i32 = -13;    // Permission denied
pub const EFAULT: i32 = -14;    // Bad address
pub const EINVAL: i32 = -22;    // Invalid argument
pub const ENOSYS: i32 = -38;    // Function not implemented
// ... more error codes
```

---

## Testing the Target Spec

### Test 1: Verify Target Loads

```bash
# Check that Rust recognizes the target
cd user_programs/test
cargo build --target ../../x86_64-aethelos-userspace.json

# Should compile without errors
```

### Test 2: Verify PIE Relocation

```bash
# Check that binary is position-independent
rust-readelf -h target/x86_64-aethelos-userspace/release/test | grep "Type:"
# Should show: Type: DYN (Position-Independent Executable)
```

### Test 3: Verify Stack Alignment

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Check RSP alignment
    let rsp: u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) rsp);
    }

    // RSP should be 16-byte aligned (or 16n-8 before call)
    assert!(rsp % 16 == 8 || rsp % 16 == 0);

    sys_exit(0);
}
```

---

## Future Enhancements

### Planned ABI Extensions

1. **Dynamic Linking:** Support for shared libraries (.so files)
2. **Thread Local Storage (TLS):** Per-thread variables
3. **C++ Support:** Exception handling, unwinding
4. **DWARF Debug Info:** Better debugging support
5. **vDSO:** Fast syscall alternatives for frequent operations

### Ancient Runes Standard Library

**Goal:** Provide Rust standard library for AethelOS

```rust
// Future: ancient-runes/corelib
pub mod syscalls;     // Syscall wrappers
pub mod alloc;        // Memory allocator
pub mod io;           // I/O traits
pub mod fs;           // Filesystem access
pub mod collections;  // Vec, HashMap, etc.
```

---

## References

- **System V AMD64 ABI:** https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf
- **Redox OS ABI:** https://doc.redox-os.org/book/ch05-02-userspace.html
- **Linux syscall interface:** `man 2 syscall`
- **AethelOS Syscall Implementation:** [SYSCALL_SYSRET_IMPLEMENTATION.md](SYSCALL_SYSRET_IMPLEMENTATION.md)
- **rust-osdev:** https://os.phil-opp.com/

---

## Summary

### Quick Reference Card

| Aspect | Details |
|--------|---------|
| **Target Spec** | `x86_64-aethelos-userspace.json` |
| **Function ABI** | System V AMD64 (RDI, RSI, RDX, RCX, R8, R9) |
| **Syscall ABI** | syscall/sysret (RAX=num, RDI-R10-R8-R9=args) |
| **Stack Alignment** | 16 bytes (RSP % 16 == 8 before call) |
| **PIE** | Enabled (code-model=small, relocation=pic) |
| **Entry Point** | `_start` (no_mangle, extern "C") |
| **Error Codes** | Negative errno values in RAX |
| **Ring 1** | CS=0x28, SS=0x30 (with capabilities) |
| **Ring 3** | CS=0x20, SS=0x18 (pure capabilities) |

**Status:** Ready for use with Runic Forge and future userspace programs.

---

*Last updated: January 2025 - AethelOS Userspace ABI v0.1*
