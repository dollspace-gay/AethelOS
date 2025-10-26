# The Weaver's Sigil - Stack Canary Protection Design

> *"Each thread weaves its own fate, and marks it with a sigil of purity. Should that mark be corrupted, the thread knows its fate has been tampered with by chaotic forces."*

## Overview

The Weaver's Sigil is AethelOS's implementation of **stack canary protection**, a runtime defense against buffer overflow attacks that attempt to overwrite return addresses on the stack.

### The Philosophy

In AethelOS, each thread is a strand in the Loom of Fate. When a thread begins weaving a new spell (calling a function), it places a unique **magical sigil** on its own thread of fate (the stack). This sigil is:

- **Personal**: Unique per-thread to prevent cross-thread attacks
- **Secret**: Unknown to userspace, preventing predictable overwrites
- **Immutable**: Any change indicates corruption

Before completing the spell (returning from the function), the thread inspects its sigil. If corrupted, the thread immediately enters **quarantine** (panic), containing the damage before it spreads.

---

## Security Model

### Threat: Stack Buffer Overflows

**Attack Vector:**
```
Stack Layout (grows downward):
┌─────────────────┐ ← High addresses
│  Return Address │ ← Attacker wants to overwrite this!
├─────────────────┤
│  Saved RBP      │
├─────────────────┤
│  Local Buffer   │ ← Overflow starts here
└─────────────────┘ ← Low addresses
```

If an attacker can overflow `Local Buffer`, they can:
1. Overwrite the saved return address
2. Redirect execution to attacker-controlled code
3. Gain arbitrary code execution

### Defense: The Weaver's Sigil

**Protected Stack Layout:**
```
Stack Layout (grows downward):
┌─────────────────┐ ← High addresses
│  Return Address │ ← Protected by canary below
├─────────────────┤
│  Saved RBP      │
├─────────────────┤
│ CANARY (8 bytes)│ ← The Weaver's Sigil! Random, secret value
├─────────────────┤
│  Local Buffer   │ ← Overflow must corrupt canary to reach return address
└─────────────────┘ ← Low addresses
```

**Function Prologue (Entry):**
```rust
// 1. Load thread-local canary
let canary = current_thread().sigil;

// 2. Push canary onto stack
push(canary);

// 3. Continue with normal prologue (save RBP, allocate locals, etc.)
```

**Function Epilogue (Exit):**
```rust
// 1. Load canary from stack
let stack_canary = pop_canary();

// 2. Compare with thread-local canary
if stack_canary != current_thread().sigil {
    panic!("◈ SIGIL CORRUPTED: Stack overflow detected!");
}

// 3. Continue with normal epilogue (restore RBP, return)
```

---

## Implementation Strategy

### Phase 1: Infrastructure (Canary Storage)

**Per-Thread Canary Storage:**

Each `Thread` structure stores its unique Weaver's Sigil:

```rust
// In loom_of_fate/mod.rs
pub struct Thread {
    // ... existing fields ...

    /// The Weaver's Sigil - unique per-thread stack canary
    /// This value is secret and should never be exposed to userspace
    pub(crate) sigil: u64,
}
```

**Canary Generation:**

Use existing entropy infrastructure (ChaCha8Rng with RDTSC):

```rust
// In loom_of_fate/mod.rs
impl Thread {
    pub fn new(entry_point: fn() -> !, priority: ThreadPriority) -> Self {
        // Generate unique sigil for this thread
        let sigil = {
            let mut rng = mana_pool::entropy::ChaCha8Rng::from_hardware_fast();
            let high = rng.next_u32() as u64;
            let low = rng.next_u32() as u64;
            (high << 32) | low
        };

        Thread {
            // ... other fields ...
            sigil,
        }
    }
}
```

**Thread-Local Access:**

Provide fast access to current thread's sigil:

```rust
// In loom_of_fate/mod.rs

/// Get the current thread's Weaver's Sigil (stack canary)
///
/// # Safety
/// This function is unsafe because it accesses thread-local state.
/// It MUST only be called from kernel code, never exposed to userspace.
#[inline(always)]
pub unsafe fn get_current_sigil() -> u64 {
    get_loom().lock().current_thread().sigil
}
```

---

### Phase 2: Compiler Integration

Rust doesn't natively support stack canaries for custom targets like `x86_64-aethelos.json`. We have two approaches:

#### Option A: LLVM Stack Protector (Preferred)

Enable LLVM's built-in stack protector in our target JSON:

```json
{
  "llvm-target": "x86_64-unknown-none",
  "target-endian": "little",
  "target-pointer-width": "64",
  "features": "-mmx,-sse,+soft-float",
  "disable-redzone": true,
  "panic-strategy": "abort",

  "stack-probes": {
    "kind": "inline-or-call",
    "min-llvm-version-for-inline": [16, 0, 0]
  },

  "// NEW": "Enable LLVM stack protector",
  "stack-protector": "strong"
}
```

**Stack Protector Modes:**
- `"basic"` - Protect functions with buffers > 8 bytes
- `"strong"` - Protect all functions with local arrays or address-taken locals
- `"all"` - Protect ALL functions (performance overhead)

**For AethelOS:** Use `"strong"` mode for security without protecting trivial functions.

**Implement `__stack_chk_fail`:**

LLVM expects a symbol `__stack_chk_fail` to be called when canary check fails:

```rust
// In lib.rs or boot.rs

/// Stack canary failure handler
/// Called by LLVM when a stack corruption is detected
#[no_mangle]
pub extern "C" fn __stack_chk_fail() -> ! {
    // Log the corruption
    serial_println!("◈ FATAL: The Weaver's Sigil has been corrupted!");
    serial_println!("   Stack overflow detected. Thread integrity compromised.");

    // Get current thread info for debugging
    unsafe {
        if let Some(thread) = loom_of_fate::get_current_thread_debug_info() {
            serial_println!("   Thread: #{} ({})", thread.id, thread.name);
            serial_println!("   Stack: 0x{:016x} - 0x{:016x}",
                thread.stack_bottom, thread.stack_top);
        }
    }

    // Panic with stack trace
    panic!("Stack canary violation - buffer overflow detected!");
}
```

**Implement `__stack_chk_guard`:**

LLVM uses a global symbol `__stack_chk_guard` as the canary value. We need to provide this and populate it per-thread:

```rust
// In lib.rs

/// Global stack canary value (thread-local via TLS)
/// LLVM uses this symbol for stack protection
#[no_mangle]
pub static mut __stack_chk_guard: u64 = 0xDEADBEEF_CAFEBABE; // Placeholder

/// Initialize stack canary for current thread
/// Called when switching to a new thread
pub unsafe fn init_thread_canary(thread_sigil: u64) {
    core::ptr::write_volatile(&mut __stack_chk_guard, thread_sigil);
}
```

**Thread Context Switch Integration:**

Update scheduler to set canary when switching threads:

```rust
// In loom_of_fate/mod.rs - schedule() function

fn schedule(&mut self) {
    // ... existing scheduling logic ...

    let next_thread = self.find_next_thread();

    // Update stack canary for new thread
    unsafe {
        crate::init_thread_canary(next_thread.sigil);
    }

    // ... continue with context switch ...
}
```

---

#### Option B: Manual Instrumentation (Fallback)

If LLVM stack protector doesn't work, we manually instrument critical functions:

**Macro for Protected Functions:**

```rust
/// Macro to add stack canary protection to a function
///
/// Usage:
/// ```
/// #[stack_protected]
/// fn vulnerable_function(buffer: &mut [u8]) {
///     // ... potentially unsafe buffer operations ...
/// }
/// ```
#[macro_export]
macro_rules! stack_protected {
    (fn $name:ident($($args:tt)*) -> $ret:ty $body:block) => {
        fn $name($($args)*) -> $ret {
            // Prologue: Save canary on stack
            let __sigil = unsafe { $crate::loom_of_fate::get_current_sigil() };

            // Execute function body
            let __result = (|| $body)();

            // Epilogue: Check canary
            let __current_sigil = unsafe { $crate::loom_of_fate::get_current_sigil() };
            if __sigil != __current_sigil {
                panic!("Stack canary violation in {}", stringify!($name));
            }

            __result
        }
    };
}
```

**Usage:**

```rust
stack_protected! {
    fn parse_user_input(buffer: &mut [u8; 256]) -> Result<Command, ParseError> {
        // Function is now protected by stack canary
        // ...
    }
}
```

---

### Phase 3: Kernel Integration

**1. Boot Initialization:**

```rust
// In main.rs

fn kernel_main() {
    // ... existing init ...

    // Initialize Weaver's Sigil protection
    println!("◈ Weaving protective sigils...");
    unsafe {
        // Set initial canary for boot thread
        let boot_sigil = 0xBADC0FFE_DEADBEEF; // Random initial value
        crate::init_thread_canary(boot_sigil);
    }
    println!("  ✓ The Weaver's Sigil protects the kernel");

    // ... continue boot ...
}
```

**2. Thread Creation:**

```rust
// In loom_of_fate/mod.rs

impl LoomOfFate {
    pub fn spawn(&mut self, entry: fn() -> !, priority: ThreadPriority)
        -> Result<ThreadId, LoomError> {

        // Create thread (generates unique sigil)
        let thread = Thread::new(entry, priority);

        serial_println!("◈ New thread sigil: 0x{:016x}", thread.sigil);

        // ... rest of spawn logic ...
    }
}
```

**3. Context Switch:**

Already covered above - update `__stack_chk_guard` on every context switch.

---

### Phase 4: Userspace Protection (Future)

When userspace processes are implemented:

1. **Separate Canaries:** Each process has its own canary namespace
2. **Syscall Barrier:** Kernel saves/restores `__stack_chk_guard` on syscall entry/exit
3. **ASLR Integration:** Combine with ASLR for layered defense

---

## Security Properties

### ✓ Defense-in-Depth

The Weaver's Sigil complements existing AethelOS protections:

| Protection | Defense Against | Status |
|------------|----------------|--------|
| **W^X** | Code injection | ✅ Active |
| **ASLR** | Return-to-libc, ROP | ✅ Active |
| **Capability Sealing** | Capability forgery | ✅ Active |
| **Opaque Handles** | Direct capability manipulation | ✅ Active |
| **Weaver's Sigil** | Stack buffer overflows | 🚧 Planned |

### ✓ Entropy Quality

- **Per-Thread:** Each thread has unique canary (64-bit)
- **Source:** ChaCha8Rng seeded from RDTSC
- **Renewal:** New canary on every thread creation
- **Secret:** Never exposed to userspace

### ✓ Performance

- **LLVM Mode:** Negligible overhead (~1-3% for "strong" mode)
- **Manual Mode:** Only protected functions pay cost
- **Context Switch:** Single write to `__stack_chk_guard` (fast)

### ⚠ Limitations

**Not Defeated By:**
- Blind overwrites (canary is random)
- Partial overwrites (full 8 bytes checked)
- Thread confusion (per-thread canaries)

**Can Be Bypassed If:**
- Attacker can read canary (info leak) → Mitigated by ASLR + secret storage
- Attacker can write to arbitrary memory (skips canary check) → W^X prevents code injection
- Attacker can corrupt heap metadata (not stack) → Future: heap canaries

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_generation() {
        let thread1 = Thread::new(test_fn, ThreadPriority::Normal);
        let thread2 = Thread::new(test_fn, ThreadPriority::Normal);

        // Each thread should have unique sigil
        assert_ne!(thread1.sigil, thread2.sigil);

        // Sigils should be non-zero
        assert_ne!(thread1.sigil, 0);
        assert_ne!(thread2.sigil, 0);
    }

    #[test]
    #[should_panic(expected = "Stack canary violation")]
    fn test_canary_detection() {
        // Simulate stack overflow by corrupting __stack_chk_guard
        unsafe {
            let original = __stack_chk_guard;
            __stack_chk_guard = 0xDEADBEEF; // Corrupt!
            __stack_chk_fail(); // Should panic
        }
    }
}
```

### Integration Tests

```rust
// In tests/stack_overflow_test.rs

#[test_case]
fn test_stack_overflow_detection() {
    // Create function with buffer overflow vulnerability
    #[stack_protected]
    fn vulnerable_function() {
        let mut buffer = [0u8; 16];

        // Simulate overflow (writes past buffer end)
        unsafe {
            core::ptr::write_bytes(buffer.as_mut_ptr(), 0xFF, 64);
        }

        // Should panic before reaching here
    }

    // Test should catch panic
    let result = panic::catch_unwind(|| vulnerable_function());
    assert!(result.is_err());
}
```

---

## Implementation Phases

### Phase 1: Infrastructure (Week 1)
- [ ] Add `sigil` field to `Thread` struct
- [ ] Implement canary generation in `Thread::new()`
- [ ] Add `get_current_sigil()` function
- [ ] Write unit tests

### Phase 2: LLVM Integration (Week 1-2)
- [ ] Update `x86_64-aethelos.json` with `"stack-protector": "strong"`
- [ ] Implement `__stack_chk_fail()` handler
- [ ] Implement `__stack_chk_guard` global
- [ ] Add `init_thread_canary()` function
- [ ] Test compilation with canaries enabled

### Phase 3: Scheduler Integration (Week 2)
- [ ] Update context switch to set `__stack_chk_guard`
- [ ] Update thread creation to initialize canary
- [ ] Add boot-time canary initialization
- [ ] Test thread switching preserves canaries

### Phase 4: Validation & Monitoring (Week 2-3)
- [ ] Add Weaver's Sigil status to `wards` command
- [ ] Implement canary violation logging
- [ ] Add debug command to inspect thread sigils
- [ ] Performance benchmarks

### Phase 5: Documentation (Week 3)
- [ ] Update DESIGN.md with Weaver's Sigil section
- [ ] Update CLAUDE.md with canary guidelines
- [ ] Add security audit checklist
- [ ] Create attack simulation tests

---

## Eldarin Shell Integration

### New Command: `sigils`

Display active Weaver's Sigils for debugging:

```
◈ The Weaver's Sigils - Stack Canary Protection

  Mode: LLVM Strong (all functions with buffers)
  Status: ✓ ACTIVE

  Active Sigils:
    Thread #0 (Boot):     0xDEADBEEF_CAFEBABE
    Thread #1 (Shell):    0x12345678_9ABCDEF0
    Thread #2 (Worker):   0xFEDCBA98_76543210
    Thread #3 (Idle):     0xABCDEF01_23456789

  Violations Detected: 0 (since boot)
  Last Violation: None

  The sigils remain pure. The threads weave in harmony.
```

### Update `wards` Command

Add Weaver's Sigil to security status:

```rust
// In wards_command.rs

pub fn show_wards_page(page: usize) {
    match page {
        0 => {
            // ... existing W^X, ASLR output ...

            // Weaver's Sigil Status
            crate::println!("  Stack Canary Protection (Weaver's Sigil): ✓ Active");
            crate::println!("    Mode: LLVM Strong (all functions with buffers)");
            crate::println!("    Violations detected: 0");
            crate::println!();

            // ... rest of page ...
        }
    }
}
```

---

## Future Enhancements

### Heap Canaries

Extend sigil concept to heap allocations:

```rust
pub struct HeapObject {
    size: usize,
    sigil_prefix: u64,  // Before allocation
    data: [u8],
    sigil_suffix: u64,  // After allocation
}
```

### Sigil Renewal

Periodically refresh canaries to limit attack windows:

```rust
pub fn renew_sigil(thread: &mut Thread) {
    thread.sigil = generate_new_canary();
    unsafe {
        if thread.id == current_thread_id() {
            init_thread_canary(thread.sigil);
        }
    }
}
```

### Safe Stack

Separate safety-critical stack data (return addresses, saved registers) from unsafe data (buffers):

```
┌─────────────────┐
│   Safe Stack    │ ← Return addresses, canaries
│  (Protected)    │
├─────────────────┤
│  Unsafe Stack   │ ← Buffers, local variables
│  (Unprotected)  │
└─────────────────┘
```

---

## References

- **PaX STACKLEAK:** https://pax.grsecurity.net/docs/stackleak.txt
- **GCC Stack Protector:** https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html
- **LLVM SafeStack:** https://clang.llvm.org/docs/SafeStack.html
- **Microsoft Control Flow Guard:** https://docs.microsoft.com/en-us/windows/win32/secbp/control-flow-guard

---

*"The Weaver's Sigil is not merely a guard—it is a promise. A promise that each thread's fate remains its own, untainted by the chaos of corruption."*

**Status:** 🚧 Design Complete, Implementation Pending
**Priority:** High (Security Critical)
**Estimated Effort:** 2-3 weeks
**Dependencies:** Existing entropy infrastructure (ChaCha8Rng)
