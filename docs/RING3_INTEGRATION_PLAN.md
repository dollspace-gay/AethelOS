# Ring 3 Integration Plan
## Comprehensive Plan for User Space Support in AethelOS

**Status:** Planning Phase
**Date:** 2025-01-30
**Goal:** Enable full user space (Ring 3) execution with process isolation

---

## Executive Summary

This document outlines the integration of all remaining components needed for Ring 3 user space support. The plan is broken into 5 implementation phases that build on each other.

**Current State:**
- ✅ Page table management (map_user_page, clone_kernel_page_table)
- ✅ Kernel stack allocation (16 KB per Vessel)
- ✅ CR3 switching (already in context.rs:248-256)
- ✅ Thread.vessel_id field (already exists)
- ✅ ThreadContext.cr3 field (already exists)
- ✅ TSS structure (already exists, initialized)
- ✅ GDT with kernel/user segments (already configured)
- ✅ Syscall infrastructure (MSRs, entry point, dispatcher)
- ✅ ELF loader (full validation)
- ✅ Vessel/Harbor infrastructure

**What's Missing:**
- ❌ TSS.rsp[0] not updated during context switches
- ❌ get_tss_mut() accessor function
- ❌ User-mode thread creation (CPL=3, IRETQ-based entry)
- ❌ Harbor integrated with LOOM (thread creation with vessel_id)
- ❌ ThreadContext.cr3 set from Vessel's page_table_phys
- ❌ Test user space program

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    User Space (Ring 3)                   │
│  ┌──────────────┐         syscall          ┌──────────┐ │
│  │ User Program │ ─────────────────────────▶ Kernel   │ │
│  │  (ELF)       │ ◀───────────────────────  (syscall  │ │
│  └──────────────┘        sysret             handler)  │ │
└─────────────────────────────────────────────────────────┘
              │                                 │
              │ IRETQ (ring 0→3)                │ syscall (ring 3→0)
              ▼                                 ▼
┌─────────────────────────────────────────────────────────┐
│                  Kernel (Ring 0)                         │
│  ┌────────────┐  ┌────────────┐  ┌─────────────────┐   │
│  │ LOOM       │  │ Harbor     │  │ Context Switch  │   │
│  │ (Threads)  │◀─│ (Vessels)  │  │ • CR3 switch    │   │
│  └────────────┘  └────────────┘  │ • TSS.rsp[0]    │   │
│                                   └─────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Key Relationships:

1. **Thread ↔ Vessel**
   - Each Thread has `vessel_id: Option<VesselId>`
   - Each Vessel has `main_thread: ThreadId`
   - Harbor provides `find_vessel_by_thread(ThreadId) -> Option<VesselId>`

2. **Context Switch Flow**
   - LOOM selects next thread
   - Check if vessel_id changed
   - If changed: update TSS.rsp[0], load new CR3
   - Perform context switch (already handles CR3 if set)

3. **Syscall Flow**
   - User code: `syscall` instruction
   - CPU switches to ring 0, loads TSS.rsp[0] into RSP
   - Syscall handler runs on kernel stack
   - `sysret` returns to user space

---

## Phase 1: TSS Integration (15 minutes)

**Goal:** Enable TSS.rsp[0] updates during context switches

### 1.1 Add Mutable TSS Accessor

**File:** `heartwood/src/attunement/gdt.rs`

**Add after `get_tss()` function (line 351):**

```rust
/// Get a mutable reference to the TSS
///
/// # Safety
/// Must only be called after init()
/// Caller must ensure no concurrent access to TSS
pub unsafe fn get_tss_mut() -> &'static mut TaskStateSegment {
    if !GDT_INITIALIZED {
        panic!("GDT/TSS not initialized!");
    }
    TSS.assume_init_mut()
}

/// Update the kernel stack pointer in the TSS
///
/// This should be called during context switches when switching to a
/// user-mode thread. The kernel_stack value will be loaded into RSP
/// when a syscall or interrupt occurs from user mode.
///
/// # Safety
/// Must be called after GDT/TSS initialization
pub unsafe fn set_kernel_stack(kernel_stack: u64) {
    let tss = get_tss_mut();
    tss.set_kernel_stack(kernel_stack);
}
```

### 1.2 Export Function

**File:** `heartwood/src/attunement/mod.rs`

**Add to public exports:**

```rust
pub use gdt::{init as gdt_init, set_kernel_stack};
```

### 1.3 Test Build

```bash
cd heartwood && cargo build --target x86_64-aethelos.json
```

**Expected:** Builds successfully, TSS can now be updated

---

## Phase 2: Harbor-LOOM Integration (30 minutes)

**Goal:** Connect Harbor (process table) with LOOM (thread scheduler)

### 2.1 Add Global Harbor

**File:** `heartwood/src/loom_of_fate/mod.rs`

**Add after LOOM static (around line 50):**

```rust
use super::harbor::Harbor;

// Global Harbor (process table)
static mut HARBOR: MaybeUninit<InterruptSafeLock<Harbor>> = MaybeUninit::uninit();
static mut HARBOR_INITIALIZED: bool = false;

/// Get reference to Harbor
pub fn get_harbor() -> &'static InterruptSafeLock<Harbor> {
    unsafe {
        if !HARBOR_INITIALIZED {
            panic!("Harbor not initialized!");
        }
        &*core::ptr::addr_of!(HARBOR).cast::<InterruptSafeLock<Harbor>>()
    }
}
```

### 2.2 Initialize Harbor

**File:** `heartwood/src/loom_of_fate/mod.rs`

**Modify `init()` function (around line 66):**

```rust
pub fn init() {
    // ... existing LOOM initialization ...

    // Initialize Harbor (process table)
    unsafe {
        let harbor = Harbor::new();
        let harbor_lock = InterruptSafeLock::new(harbor, "HARBOR");
        core::ptr::write(core::ptr::addr_of_mut!(HARBOR).cast(), harbor_lock);
        HARBOR_INITIALIZED = true;

        for &byte in b"[HARBOR INIT] Harbor initialized\n".iter() {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
```

### 2.3 Create User Thread Function

**File:** `heartwood/src/loom_of_fate/mod.rs`

**Add new public function:**

```rust
/// Create a user-mode thread for a Vessel
///
/// This creates a thread that will execute in ring 3 (user mode) within
/// the specified Vessel's address space.
///
/// # Arguments
/// * `vessel_id` - The Vessel this thread belongs to
/// * `entry_point` - User space entry point address
/// * `user_stack_top` - Top of user stack
/// * `priority` - Thread priority
///
/// # Returns
/// ThreadId of the created thread
pub fn create_user_thread(
    vessel_id: VesselId,
    entry_point: u64,
    user_stack_top: u64,
    priority: ThreadPriority,
) -> Result<ThreadId, LoomError> {
    // This will be implemented in Phase 4
    // For now, return error
    Err(LoomError::StackAllocationFailed)
}
```

### 2.4 Update Context Switch to Update TSS

**File:** `heartwood/src/loom_of_fate/scheduler.rs`

**Modify `prepare_yield()` to return vessel info (around line 147):**

Change return type from:
```rust
(bool, *mut ThreadContext, *const ThreadContext)
```
To:
```rust
(bool, *mut ThreadContext, *const ThreadContext, Option<u64>)
```

**Before the return statement, add:**

```rust
// Check if we need to update TSS.rsp[0]
let new_kernel_stack = if let Some(vessel_id) = next_thread.vessel_id() {
    // This is a user-mode thread - need to update TSS
    use crate::loom_of_fate::get_harbor;
    let harbor = get_harbor().lock();
    if let Some(vessel) = harbor.find_vessel(vessel_id) {
        Some(vessel.kernel_stack())
    } else {
        None
    }
} else {
    None
};
```

**Update return statement:**

```rust
return (true, from_ctx_ptr, to_ctx_ptr, new_kernel_stack);
```

### 2.5 Update yield_now() to Use New Return

**File:** `heartwood/src/loom_of_fate/mod.rs`

**Modify `yield_now()` (around line 288):**

```rust
let (should_switch, from_ctx_ptr, to_ctx_ptr, new_kernel_stack) = loom.prepare_yield();

if should_switch {
    // Update TSS if needed
    if let Some(kernel_stack) = new_kernel_stack {
        unsafe {
            crate::attunement::set_kernel_stack(kernel_stack);
        }
    }

    drop(loom);
    context::switch_context_cooperative(from_ctx_ptr, to_ctx_ptr);
}
```

---

## Phase 3: User Thread Context Initialization (30 minutes)

**Goal:** Create threads that start in ring 3 (user mode)

### 3.1 Add User Context Creation

**File:** `heartwood/src/loom_of_fate/context.rs`

**Add new function after `ThreadContext::new()`:**

```rust
/// Create a new context for a user-mode thread (Ring 3)
///
/// # Arguments
/// * `entry_point` - User space entry point address
/// * `user_stack_top` - Top of user stack
/// * `kernel_stack_top` - Top of kernel stack (for syscall handling)
/// * `page_table_phys` - Physical address of PML4 (CR3 value)
///
/// # Returns
/// A context ready for ring 3 execution
pub fn new_user_mode(
    entry_point: u64,
    user_stack_top: u64,
    page_table_phys: u64,
) -> Self {
    ThreadContext {
        // General purpose registers start at zero
        r15: 0, r14: 0, r13: 0, r12: 0,
        rbp: 0, rbx: 0, r11: 0, r10: 0,
        r9: 0, r8: 0, rax: 0, rcx: 0,
        rdx: 0, rsi: 0, rdi: 0,

        // Special registers for RING 3
        rip: entry_point,
        cs: 0x20 | 3,  // User code segment (index 4, RPL=3)
        rflags: 0x202,  // Interrupts enabled (IF flag)
        rsp: user_stack_top - 8,  // User stack (aligned)
        ss: 0x18 | 3,  // User data segment (index 3, RPL=3)
        cr3: page_table_phys,  // Vessel's page table
    }
}
```

### 3.2 Create User Mode Entry Function

**File:** `heartwood/src/loom_of_fate/context.rs`

**Add at end of file:**

```rust
/// Enter user mode for the first time
///
/// This function uses IRETQ to transition from ring 0 to ring 3.
/// It's used when starting a user-mode thread for the first time.
///
/// # Safety
/// This function never returns - it jumps to user mode.
/// The context must be properly initialized for user mode.
#[unsafe(naked)]
pub unsafe extern "C" fn enter_user_mode(_context: *const ThreadContext) -> ! {
    core::arch::naked_asm!(
        // rdi = context pointer

        // Load CR3 (page table)
        "mov rax, [rdi + 0xA0]",  // cr3 offset
        "mov cr3, rax",

        // Set up IRETQ frame on stack:
        // Stack layout (pushed in reverse order):
        // [SS]      <- Top
        // [RSP]
        // [RFLAGS]
        // [CS]
        // [RIP]     <- Bottom (RSP points here after pushes)

        // Push SS (user data segment)
        "push qword ptr [rdi + 0x98]",  // ss offset

        // Push RSP (user stack pointer)
        "push qword ptr [rdi + 0x90]",  // rsp offset

        // Push RFLAGS
        "push qword ptr [rdi + 0x88]",  // rflags offset

        // Push CS (user code segment)
        "push qword ptr [rdi + 0x80]",  // cs offset

        // Push RIP (user entry point)
        "push qword ptr [rdi + 0x78]",  // rip offset

        // Zero out all registers for security (don't leak kernel data)
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor rdi, rdi",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",
        "xor rbp, rbp",

        // Enter user mode!
        "iretq",
    );
}
```

### 3.3 Implement create_user_thread()

**File:** `heartwood/src/loom_of_fate/mod.rs`

**Replace the stub with:**

```rust
pub fn create_user_thread(
    vessel_id: VesselId,
    entry_point: u64,
    user_stack_top: u64,
    priority: ThreadPriority,
) -> Result<ThreadId, LoomError> {
    without_interrupts(|| {
        let mut loom = get_loom().lock();

        // Get Vessel info
        let harbor = get_harbor().lock();
        let vessel = harbor.find_vessel(vessel_id)
            .ok_or(LoomError::StackAllocationFailed)?;  // TODO: Add VesselNotFound error

        let page_table_phys = vessel.page_table_phys();
        drop(harbor);

        // Generate thread ID
        let thread_id = ThreadId(loom.next_thread_id);
        loom.next_thread_id += 1;

        // Create user-mode context
        let context = ThreadContext::new_user_mode(
            entry_point,
            user_stack_top,
            page_table_phys,
        );

        // Create thread (no stack allocation - using user stack)
        let thread = Thread::new_with_context(
            thread_id,
            context,
            priority,
            Some(vessel_id),
        );

        loom.threads.push(thread);
        loom.ready_queue.push_back(thread_id);

        Ok(thread_id)
    })
}
```

### 3.4 Add Thread::new_with_context()

**File:** `heartwood/src/loom_of_fate/thread.rs`

**Add new constructor:**

```rust
/// Create a thread with a pre-initialized context
///
/// Used for user-mode threads where the context is set up specially.
pub fn new_with_context(
    id: ThreadId,
    context: ThreadContext,
    priority: ThreadPriority,
    vessel_id: Option<VesselId>,
) -> Self {
    Thread {
        id,
        state: ThreadState::Weaving,
        priority,
        entry_point: dummy_entry as fn() -> !,  // Placeholder
        context,
        stack_bottom: 0,  // User stack, we don't track it
        stack_top: 0,
        sigil: entropy::generate_u64(),
        vessel_id,
        resource_usage: ResourceUsage::new(),
        harmony_score: 1.0,
        time_slices_used: 0,
        yields: 0,
        last_run_time: 0,
    }
}

// Dummy entry point for user threads
fn dummy_entry() -> ! {
    loop {}
}
```

---

## Phase 4: Test User Program (45 minutes)

**Goal:** Write, compile, and embed a test user space program

### 4.1 Create User Space Test Program

**File:** `user_programs/hello/src/main.rs` (new directory)

```rust
#![no_std]
#![no_main]

// Syscall numbers (from heartwood/src/loom_of_fate/syscalls.rs)
const SYS_WRITE: u64 = 1;
const SYS_EXIT: u64 = 60;

#[unsafe(naked)]
unsafe extern "C" fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    core::arch::naked_asm!(
        "mov rax, rdi",  // syscall number
        "mov rdi, rsi",  // arg1
        "mov rsi, rdx",  // arg2
        "mov rdx, rcx",  // arg3
        "syscall",
        "ret",
    );
}

fn write(fd: u64, buf: &[u8]) -> i64 {
    unsafe {
        syscall3(SYS_WRITE, fd, buf.as_ptr() as u64, buf.len() as u64)
    }
}

fn exit(code: u64) -> ! {
    unsafe {
        syscall3(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from user space! (Ring 3)\n";
    write(1, msg);
    exit(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1)
}
```

### 4.2 Create Cargo Config

**File:** `user_programs/hello/Cargo.toml`

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2021"

[profile.release]
panic = "abort"
lto = true

[dependencies]
```

### 4.3 Create Linker Script

**File:** `user_programs/hello/user.ld`

```ld
ENTRY(_start)

SECTIONS
{
    . = 0x400000;  /* User space starts at 4MB */

    .text : {
        *(.text._start)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.note.gnu.build-id)
    }
}
```

### 4.4 Build User Program

```bash
cd user_programs/hello
cargo build --release --target x86_64-unknown-none
objcopy -O binary target/x86_64-unknown-none/release/hello hello.bin
```

### 4.5 Embed in Kernel

**File:** `heartwood/src/test_programs.rs` (new file)

```rust
//! Embedded test programs

pub const HELLO_ELF: &[u8] = include_bytes!("../../user_programs/hello/hello.bin");
```

**File:** `heartwood/src/lib.rs`

Add module:
```rust
pub mod test_programs;
```

### 4.6 Create Shell Command to Launch

**File:** `heartwood/src/eldarin.rs`

**Add new command:**

```rust
"test-user" => cmd_test_user(),
```

**Add implementation:**

```rust
fn cmd_test_user() {
    use crate::test_programs::HELLO_ELF;
    use crate::loom_of_fate::{create_user_thread, get_harbor, ThreadPriority};

    crate::println!("◈ Loading test user program...");

    // Create Vessel from ELF
    let harbor = crate::loom_of_fate::get_harbor();
    let mut harbor_lock = harbor.lock();

    match harbor_lock.moor_user_vessel(
        None,  // No parent
        HELLO_ELF,
        "user".to_string(),
        ThreadId(0),  // Placeholder, will be updated
    ) {
        Ok(vessel_id) => {
            drop(harbor_lock);

            // Get entry point and stack
            let harbor_lock = harbor.lock();
            let vessel = harbor_lock.find_vessel(vessel_id).unwrap();
            let entry_point = vessel.entry_point();
            let user_stack = 0x0000_7FFF_FFFF_0000u64;  // From USER_STACK_TOP
            drop(harbor_lock);

            // Create user thread
            match create_user_thread(
                vessel_id,
                entry_point,
                user_stack,
                ThreadPriority::Normal,
            ) {
                Ok(thread_id) => {
                    crate::println!("✓ User program launched (thread {})", thread_id.0);
                }
                Err(e) => {
                    crate::println!("✗ Failed to create thread: {:?}", e);
                }
            }
        }
        Err(e) => {
            crate::println!("✗ Failed to load ELF: {}", e);
        }
    }
}
```

---

## Phase 5: Integration Testing (30 minutes)

**Goal:** Test the complete ring 3 → ring 0 → ring 3 cycle

### 5.1 Build Complete System

```bash
# Build user program
cd user_programs/hello
cargo build --release --target x86_64-unknown-none
cd ../..

# Build kernel
cd heartwood
cargo build --target x86_64-aethelos.json

# Create ISO
cd ..
wsl bash -c "cp target/x86_64-aethelos/debug/heartwood isodir/boot/aethelos/heartwood.bin && grub-mkrescue -o aethelos.iso isodir"
```

### 5.2 Boot in QEMU

```bash
cd F:\OS
cmd /c "cd /d F:\OS\aethelos-source && taskkill /F /IM qemu-system-x86_64.exe 2>nul & del serial.log 2>nul & start /B "" "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom aethelos.iso -serial file:serial.log -m 256M -display gtk -no-reboot -no-shutdown"
```

### 5.3 Test Commands

In Eldarin shell:
```
test-user
```

**Expected Output:**
```
◈ Loading test user program...
[VESSEL] Creating Vessel 1 from ELF
[ELF] Parsing ELF file:
[USER_SPACE] Created new address space with PML4 @ 0x...
[VESSEL] Allocated kernel stack: 0x... - 0x... (size: 0x4000)
[VESSEL] ✓ Vessel 1 created: entry=0x400000, CR3=0x..., kernel_stack=0x...
✓ User program launched (thread 10)

Hello from user space! (Ring 3)
```

### 5.4 Verification Checklist

- [ ] User program loads successfully
- [ ] Vessel created with correct CR3
- [ ] Kernel stack allocated
- [ ] Thread transitions to ring 3
- [ ] SYS_WRITE syscall works
- [ ] Serial output shows "Hello from user space!"
- [ ] SYS_EXIT syscall works
- [ ] Thread terminates cleanly

---

## Error Handling & Edge Cases

### Common Issues:

1. **Page Fault on User Entry**
   - Check CR3 is loaded correctly
   - Verify user stack is mapped
   - Check code segment is mapped with EXECUTE flag

2. **Triple Fault**
   - TSS not loaded correctly
   - Kernel stack invalid
   - GDT segments incorrect

3. **General Protection Fault**
   - CS/SS RPL mismatch
   - Segment selectors wrong
   - User code trying to execute privileged instruction

4. **Syscall Returns to Wrong Location**
   - Check RFLAGS preservation
   - Verify RCX/R11 not corrupted
   - Ensure IA32_STAR configured correctly

### Debugging Tips:

1. **Use serial logging extensively**
   ```rust
   crate::serial_println!("[DEBUG] About to enter user mode: RIP={:#x}", entry);
   ```

2. **Add assertions**
   ```rust
   assert!(entry_point < 0x0000_8000_0000_0000, "Entry point in kernel space!");
   ```

3. **Check QEMU logs**
   ```bash
   qemu-system-x86_64 -d int,cpu_reset -D qemu.log ...
   ```

---

## Timeline Estimate

| Phase | Task | Estimated Time | Cumulative |
|-------|------|----------------|------------|
| 1 | TSS Integration | 15 min | 15 min |
| 2 | Harbor-LOOM Integration | 30 min | 45 min |
| 3 | User Thread Context | 30 min | 1h 15min |
| 4 | Test User Program | 45 min | 2h |
| 5 | Integration Testing | 30 min | 2h 30min |
| **Total** | **All Phases** | **~2.5 hours** | |

---

## Success Criteria

✅ **Complete Success:**
- User program executes in ring 3
- Syscalls transition to ring 0 and back
- Output appears correctly
- System remains stable after execution

✅ **Partial Success:**
- Vessel created successfully
- Thread created in ring 3
- Some syscalls work

❌ **Failure:**
- Triple fault on user entry
- Page faults that halt system
- Unable to create Vessel/Thread

---

## Post-Implementation Tasks

After successful integration:

1. **Add more syscalls:**
   - SYS_READ
   - SYS_OPEN
   - SYS_CLOSE
   - SYS_MMAP

2. **Improve error handling:**
   - Add LoomError::VesselNotFound
   - Better error messages in shell

3. **Add multiple user programs:**
   - Calculator
   - Text editor
   - Shell

4. **Implement process termination:**
   - Clean up Vessel resources
   - Free page tables
   - Free kernel stack

5. **Add fork/exec:**
   - Clone address space
   - Load new ELF

---

## Dependencies Map

```
Phase 1 (TSS)
    ↓
Phase 2 (Harbor-LOOM) ← depends on Phase 1
    ↓
Phase 3 (User Context) ← depends on Phase 1, 2
    ↓
Phase 4 (Test Program) ← depends on Phase 1, 2, 3
    ↓
Phase 5 (Testing) ← depends on all previous
```

**Note:** Phases must be done in order. Each phase builds on previous phases.

---

## Files Modified Summary

| File | Purpose | Lines Changed |
|------|---------|---------------|
| `heartwood/src/attunement/gdt.rs` | Add get_tss_mut(), set_kernel_stack() | +25 |
| `heartwood/src/attunement/mod.rs` | Export set_kernel_stack | +1 |
| `heartwood/src/loom_of_fate/mod.rs` | Add Harbor, create_user_thread(), update yield_now() | +100 |
| `heartwood/src/loom_of_fate/scheduler.rs` | Update prepare_yield() return type | +20 |
| `heartwood/src/loom_of_fate/context.rs` | Add new_user_mode(), enter_user_mode() | +80 |
| `heartwood/src/loom_of_fate/thread.rs` | Add new_with_context() | +25 |
| `heartwood/src/test_programs.rs` | Embed test program | +5 |
| `heartwood/src/eldarin.rs` | Add test-user command | +40 |
| `user_programs/hello/src/main.rs` | User space test program | +50 |
| **Total** | | **~346 lines** |

---

## Conclusion

This plan provides a comprehensive, step-by-step approach to integrating Ring 3 support into AethelOS. Each phase is:
- ✅ **Self-contained** - Can be implemented and tested independently
- ✅ **Buildable** - Code compiles at each phase
- ✅ **Testable** - Has clear success criteria
- ✅ **Incremental** - Builds on previous work

The total implementation time is estimated at **2.5 hours** for an experienced developer familiar with the codebase.

**Next Step:** Begin Phase 1 (TSS Integration)
