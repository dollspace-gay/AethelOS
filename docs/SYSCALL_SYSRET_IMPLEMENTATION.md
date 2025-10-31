# Syscall/Sysret Implementation Guide for AethelOS

> **Status:** Research complete, implementation deferred until userspace is needed
> **References:** Redox OS, Linux kernel, rust-osdev community

---

## Why Not INT 0x80?

**Problem Discovered:** The x86_64 crate's `set_handler_fn` is incompatible with naked functions that bypass the `extern "x86-interrupt"` calling convention. INT 0x80 works in Linux because:
1. Linux uses its own IDT management, not a third-party crate
2. The kernel has full control over interrupt frame layout
3. There's no compiler-generated prologue/epilogue to interfere

**For Rust OS development:** Use the modern **syscall/sysret** mechanism instead.

---

## Syscall/Sysret vs INT 0x80

| Feature | INT 0x80 | syscall/sysret |
|---------|----------|----------------|
| **Speed** | ~100-300 cycles | ~60-80 cycles |
| **Mechanism** | IDT-based interrupt | MSR-based fast path |
| **Compatibility** | Works on all x86 | x86-64 only (requires IA32_EFER.SCE) |
| **Stack handling** | Automatic interrupt frame | Manual stack switching |
| **Rust crate support** | Problematic with x86_64 crate | Full control with MSRs |

**Verdict:** syscall/sysret is faster, modern, and avoids the x86_64 crate compatibility issues.

---

## Architecture Overview

```
Userspace (Ring 3)              Kernel Space (Ring 0)
┌────────────────────┐          ┌─────────────────────────┐
│  mov rax, 1        │          │  syscall_handler:       │
│  mov rdi, "Hello"  │          │    swapgs               │
│  syscall           │──────────>│    save user RSP        │
│                    │          │    load kernel RSP      │
│  ; RAX = result    │<──────────│    dispatch_syscall()   │
│  ret               │          │    sysretq              │
└────────────────────┘          └─────────────────────────┘
```

---

## Implementation Steps

### 1. MSR Configuration (During Kernel Init)

```rust
use x86_64::registers::model_specific::{Msr, Star, LStar, SFMask, Efer, EferFlags};

pub unsafe fn init_syscall() {
    // Step 1: Enable syscall/sysret in EFER
    Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);

    // Step 2: Configure STAR - segment selectors
    // Bits 32:47 = kernel code segment (0x08)
    // Bits 48:63 = user code segment base (0x20 - 3 = Ring 3)
    // Formula: user_cs = STAR[48:63] + 16, user_ss = STAR[48:63] + 8
    Star::write(0x0023_0008_0000_0000u64);

    // Step 3: Set LSTAR - syscall entry point
    LStar::write(VirtAddr::new(syscall_entry as u64));

    // Step 4: Configure SFMASK - mask RFLAGS on entry
    // Clear: IF (0x200) = disable interrupts during stack setup
    //        DF (0x400) = direction flag must be clear per ABI
    //        TF (0x100) = trap flag
    SFMask::write(0x0700); // Mask IF, DF, TF

    crate::serial_println!("[SYSCALL] ✓ MSRs configured for syscall/sysret");
}
```

**Critical:** Call this AFTER setting up GDT with proper segment selectors!

---

### 2. The Naked Syscall Handler

```rust
/// Syscall entry point - called from userspace via syscall instruction
///
/// # Register State on Entry (from userspace)
/// - RAX: syscall number
/// - RDI, RSI, RDX, R10, R8, R9: arguments 1-6
/// - RCX: return RIP (saved by CPU)
/// - R11: RFLAGS (saved by CPU)
/// - RSP: user stack pointer
///
/// # What CPU Does
/// - Loads RIP from IA32_LSTAR
/// - Saves return RIP into RCX
/// - Saves RFLAGS into R11
/// - Masks RFLAGS using IA32_FMASK
/// - Loads CS/SS from IA32_STAR
/// - DOES NOT switch stacks! (we must do this)
#[naked]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // === CRITICAL: swapgs FIRST ===
        // Swap GS base to access per-CPU kernel data
        "swapgs",

        // === Save user RSP, load kernel RSP ===
        // SECURITY: User RSP could be malicious, validate before use!
        "mov gs:[offset_user_rsp], rsp",     // Save user RSP in per-CPU area
        "mov rsp, gs:[offset_kernel_rsp]",   // Load kernel RSP from per-CPU area

        // === Build interrupt-like frame on kernel stack ===
        // This allows sysret or iret depending on situation
        "push 0x23",              // User SS (from STAR)
        "push gs:[offset_user_rsp]", // User RSP
        "push r11",               // RFLAGS (saved by CPU)
        "push 0x2b",              // User CS (from STAR + 16)
        "push rcx",               // Return RIP (saved by CPU)

        // === Save all general-purpose registers ===
        // Must preserve: RCX (return RIP), R11 (RFLAGS)
        // Must save syscall args: RAX, RDI, RSI, RDX, R10, R8, R9
        "push rax",  // syscall number
        "push rbx",
        "push rcx",  // return RIP (already on stack but save again for clarity)
        "push rdx",  // arg3
        "push rsi",  // arg2
        "push rdi",  // arg1
        "push rbp",
        "push r8",   // arg5
        "push r9",   // arg6
        "push r10",  // arg4
        "push r11",  // RFLAGS (already on stack but save for clarity)
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // === Call Rust syscall dispatcher ===
        // Arguments: syscall_num, arg1, arg2, arg3, arg4, arg5, arg6
        "mov rdi, rax",              // syscall_num
        "mov rsi, [rsp + 10*8]",     // arg1 (saved rdi)
        "mov rdx, [rsp + 11*8]",     // arg2 (saved rsi)
        "mov rcx, [rsp + 12*8]",     // arg3 (saved rdx)
        "mov r8,  [rsp + 6*8]",      // arg4 (saved r10)
        "mov r9,  [rsp + 5*8]",      // arg5 (saved r8)
        // arg6 would go on stack, but we'll pass pointer to saved regs instead
        "mov rsi, rsp",              // Pass pointer to all saved registers
        "call {handler}",            // Returns result in RAX

        // === Restore registers ===
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",  // RFLAGS (restore into r11 for sysretq)
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",  // Return RIP (restore into rcx for sysretq)
        "pop rbx",
        // Skip RAX - it contains the return value!
        "add rsp, 8",

        // === Clean up interrupt frame ===
        "pop rcx",   // User return RIP
        "add rsp, 8", // Skip user CS
        "pop r11",   // User RFLAGS
        "add rsp, 8", // Skip user SS
        "pop rsp",   // Restore user RSP (DANGEROUS if not validated!)

        // === Return to userspace ===
        "swapgs",    // Restore user GS
        "sysretq",   // Return to ring 3 (jumps to RCX, restores RFLAGS from R11)

        handler = sym syscall_handler_rust,
    )
}
```

---

### 3. The Rust Syscall Dispatcher

```rust
/// Rust syscall handler (called from naked assembly)
///
/// # Arguments
/// * `regs` - Pointer to saved register state on kernel stack
///
/// # Returns
/// Result in RAX (positive for success, negative for error)
unsafe extern "C" fn syscall_handler_rust(regs: *const SavedRegisters) -> i64 {
    let regs = &*regs;

    // Extract arguments
    let syscall_num = regs.rax;
    let arg1 = regs.rdi;
    let arg2 = regs.rsi;
    let arg3 = regs.rdx;
    let arg4 = regs.r10;  // Note: r10, not rcx!
    let arg5 = regs.r8;
    let arg6 = regs.r9;

    // Validate user pointers BEFORE using them
    // (See ward_of_sacred_boundaries.rs)

    // Dispatch to syscall implementation
    crate::loom_of_fate::syscalls::dispatch_syscall(
        syscall_num,
        arg1, arg2, arg3, arg4, arg5, arg6
    )
}

#[repr(C)]
struct SavedRegisters {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,  // RFLAGS
    r10: u64,  // arg4
    r9: u64,   // arg6
    r8: u64,   // arg5
    rbp: u64,
    rdi: u64,  // arg1
    rsi: u64,  // arg2
    rdx: u64,  // arg3
    rcx: u64,  // return RIP
    rbx: u64,
    rax: u64,  // syscall number
}
```

---

## Security Considerations

### 1. **User Stack Validation**
```rust
// NEVER trust user RSP!
if user_rsp >= KERNEL_BASE || user_rsp < USER_BASE {
    return -EFAULT; // Invalid pointer
}
```

### 2. **RCX Canonicalization** (Redox approach)
```rust
// Prevent non-canonical addresses from crashing sysretq
// Shift away upper 16 bits, then sign-extend bit 47
rcx = (rcx << 16) >> 16;  // Arithmetic shift extends sign bit
```

### 3. **SMAP/SMEP Enforcement**
- Use `stac`/`clac` for user memory access (already implemented in Ward)
- Validate ALL user pointers before dereferencing

### 4. **Per-CPU Data via GS**
```rust
// Each CPU needs its own kernel stack pointer
struct PerCpuData {
    kernel_rsp: u64,   // Kernel stack for this CPU
    user_rsp: u64,     // Saved user stack during syscall
    // ... other per-CPU data
}
```

---

## Userspace Calling Convention (AethelOS ABI)

### From Userspace Assembly
```asm
; Example: sys_write(fd=1, buf="Hello", len=5)
mov rax, 1              ; syscall number: SYS_WRITE
mov rdi, 1              ; arg1: fd
lea rsi, [message]      ; arg2: buf pointer
mov rdx, 5              ; arg3: len
syscall                 ; invoke kernel
; Result in RAX (positive = bytes written, negative = error)
```

### From Userspace Rust
```rust
#[inline(always)]
unsafe fn syscall1(num: u64, arg1: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        lateout("rax") ret,
        out("rcx") _,   // Clobbered by syscall
        out("r11") _,   // Clobbered by syscall
    );
    ret
}

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
```

---

## Integration with AethelOS

### When to Implement

**Prerequisites:**
1. ✅ GDT with proper segments (already done)
2. ✅ Per-CPU data structures (TODO)
3. ✅ Kernel stacks allocated per thread (already done in Loom of Fate)
4. ⚪ Userspace loader (ELF loader, process management)
5. ⚪ Virtual memory management for user address space

**Suggested Timeline:**
- **Phase 1:** Implement per-CPU data structures with GS
- **Phase 2:** Create ELF loader and process abstraction (Vessel)
- **Phase 3:** Implement syscall/sysret as described above
- **Phase 4:** Port Eldarin shell to userspace as first test program

---

## Testing Strategy

### 1. **Minimal Test Program**
```rust
// First userspace program: just call exit(42)
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe { syscall1(SYS_EXIT, 42) };
    unreachable!();
}
```

### 2. **Hello World**
```rust
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from userspace!\n";
    sys_write(1, msg).unwrap();
    sys_exit(0);
}
```

### 3. **Stress Test**
- Rapid syscall invocations (10,000+ per second)
- Multi-threaded syscalls
- Invalid pointer tests (should return -EFAULT, not panic)

---

## Advantages Over INT 0x80

| Benefit | Impact |
|---------|--------|
| **Performance** | ~2-3x faster than INT 0x80 |
| **No x86_64 crate conflicts** | Full control over implementation |
| **Modern approach** | Used by Linux, Redox, all modern OSes |
| **Better debugging** | Stack frames are clearer than interrupt frames |
| **Flexibility** | Can optimize fast paths (e.g., getpid doesn't need full context) |

---

## References

- **Redox OS:** [kernel/src/arch/x86_64/interrupt/syscall.rs](https://github.com/redox-os/kernel/blob/master/src/arch/x86_64/interrupt/syscall.rs)
- **Linux Kernel:** `arch/x86/entry/entry_64.S` - entry_SYSCALL_64
- **Blog:** [Rust-OS Kernel - To userspace and back!](https://nfil.dev/kernel/rust/coding/rust-kernel-to-userspace-and-back/)
- **Intel Manual:** Volume 3, Chapter 5.8.8 "Fast System Calls"
- **rust-osdev:** [x86_64 issue #244](https://github.com/rust-osdev/x86_64/issues/244) - syscall/sysret wrappers

---

## Summary

**Current Status:** INT 0x80 disabled due to x86_64 crate incompatibility
**Recommended Path:** Implement syscall/sysret when userspace is ready
**Complexity:** Medium (requires per-CPU data, but avoids IDT issues)
**Performance:** Significantly better than INT 0x80
**Compatibility:** Modern, well-documented, production-proven

**Next Steps:** Focus on userspace infrastructure (ELF loader, process management) before implementing syscalls.
