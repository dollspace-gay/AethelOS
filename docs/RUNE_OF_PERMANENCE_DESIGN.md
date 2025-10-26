# The Rune of Permanence - Read-Only Kernel Data Protection

> *"The fundamental laws of the realm, once scribed at the Dawn of Awakening, are immutable. These crystalline structures, etched into the fabric of reality, cannot be altered—for to change them would be to rewrite the very physics of the world."*

## Overview

The Rune of Permanence is AethelOS's implementation of **kernel data protection through immutability**. After the kernel completes its initialization (the "Dawn of Awakening"), critical data structures are marked as read-only at the hardware level, preventing any runtime modification—even by the kernel itself.

### The Philosophy

In AethelOS, the kernel's foundational structures are not merely "protected"—they are **enchanted with permanence**. Once the realm awakens and its laws are established, those laws become as immutable as the laws of physics. The MMU (Memory Management Unit) acts as a guardian of reality itself, enforcing these constraints with hardware precision.

Any attempt to modify a permanent structure is not a "security violation"—it is an attempt to **break the fabric of reality**, and the MMU denies it instantly with a page fault.

---

## Security Model

### Threat: Data-Only Attacks

Even with W^X (no executable data), attackers can still compromise a system by **corrupting data structures** rather than injecting code:

#### Attack Vector 1: Function Pointer Overwrites

```rust
// Vulnerable dispatch table
static mut SYSCALL_TABLE: [fn(); 256] = [ /* ... */ ];

// Attacker exploits buffer overflow to overwrite entry:
SYSCALL_TABLE[42] = attacker_function; // ← Redirect syscall!
```

**Impact:** Attacker gains arbitrary code execution by hijacking control flow through legitimate function pointers.

#### Attack Vector 2: Security Policy Corruption

```rust
// Vulnerable security configuration
static mut CAPABILITIES_ENABLED: bool = true;

// Attacker overwrites this to disable security:
CAPABILITIES_ENABLED = false; // ← Bypass capability checks!
```

**Impact:** Attacker disables security features by corrupting policy flags.

#### Attack Vector 3: Critical Constant Corruption

```rust
// Vulnerable kernel configuration
static mut MAX_THREADS: usize = 256;

// Attacker sets this to 0:
MAX_THREADS = 0; // ← Denial of service!
```

**Impact:** System becomes unstable or unusable.

### Defense: The Rune of Permanence

**After-Boot Immutability:**

```
┌─────────────────────────────────────────────┐
│         Kernel Memory Layout                │
├─────────────────────────────────────────────┤
│  .text (code)        │ R-X │ ← Already W^X  │
├─────────────────────────────────────────────┤
│  .rodata (constants) │ R-- │ ← Already RO   │
├─────────────────────────────────────────────┤
│  .data (variables)   │ RW- │ ← Mutable      │
├─────────────────────────────────────────────┤
│  .rune (permanent)   │ RW- │ ← Boot phase   │
│                      │  ↓  │                │
│                      │ R-- │ ← After boot!  │
└─────────────────────────────────────────────┘
```

**The `.rune` section** contains structures that:
1. **Are initialized at boot** (writable during init)
2. **Never change afterward** (read-only after init)
3. **Are security-critical** (function pointers, policies, etc.)

---

## Implementation Strategy

### Phase 1: Identify Permanent Structures

Categorize kernel data by mutability:

#### ✓ Permanent (Mark as `.rune`)

**Function Pointer Tables:**
- Syscall dispatch table
- Interrupt handler table (IDT entries)
- Virtual function tables (vtables)
- Driver callback tables

**Security Policies:**
- Capability enforcement flags
- W^X enforcement settings
- ASLR configuration
- Stack canary enforcement

**Kernel Configuration:**
- Thread limits (MAX_THREADS)
- Memory layout constants (KERNEL_BASE)
- Hardware resource limits

**Immutable References:**
- Static references to permanent structures
- Global singletons initialized at boot

#### ✗ Mutable (Keep as `.data`)

**Runtime State:**
- Current thread ID
- Scheduler state
- Memory allocator free lists
- I/O buffers

**Statistics:**
- Thread counters
- Memory usage metrics
- Performance timers

**Lock State:**
- Mutex ownership
- Semaphore counts

---

### Phase 2: Rust Language Support

Rust provides excellent compile-time immutability, but we need **runtime immutability** after boot.

#### Option A: `const` for Compile-Time Immutability

```rust
// Already immutable at compile time
const KERNEL_VERSION: &str = "0.1.0-alpha";

// Compiler places this in .rodata (read-only section)
```

**Limitation:** Can't initialize complex structures at runtime (e.g., generated entropy, hardware detection).

#### Option B: `static` + Custom Section for Runtime Immutability

```rust
// Place in custom .rune section for post-boot protection
#[link_section = ".rune"]
static mut SYSCALL_TABLE: SyscallTable = SyscallTable::empty();

// During boot: Initialize (writable)
unsafe {
    SYSCALL_TABLE.register(0, sys_read);
    SYSCALL_TABLE.register(1, sys_write);
    // ...
}

// After boot: Mark page as read-only
unsafe {
    seal_permanent_structures();
}
```

**Advantages:**
- Runtime initialization (can use hardware RNG, probe devices)
- Explicit control over when immutability takes effect
- Clear separation of permanent vs mutable data

---

### Phase 3: Memory Page Protection

Use x86_64 page tables to enforce read-only protection at the hardware level.

#### Page Table Entry Bits

```
┌───────────────────────────────────────────────┐
│  Page Table Entry (x86_64)                    │
├───────────────────────────────────────────────┤
│  Bit 0: Present (P)         │ 1 = page valid  │
│  Bit 1: Read/Write (RW)     │ 1 = writable    │ ← Clear this!
│  Bit 2: User/Supervisor (US)│ 0 = kernel only │
│  Bit 3: Write-Through (PWT) │                 │
│  Bit 4: Cache Disable (PCD) │                 │
│  Bit 5: Accessed (A)        │                 │
│  Bit 6: Dirty (D)           │                 │
│  Bit 7: Page Size (PS)      │                 │
│  Bit 63: Execute Disable (NX)│ 1 = no execute │
└───────────────────────────────────────────────┘
```

**To make a page read-only:**
1. Find the page table entry (PTE) for the `.rune` section
2. Clear bit 1 (Read/Write bit) → `RW = 0`
3. Flush TLB (translation lookaside buffer) to apply change

#### Implementation

```rust
// In mana_pool/page_tables.rs

/// Seal the `.rune` section as read-only after boot
///
/// # Safety
/// This MUST only be called once, after all permanent structures
/// have been initialized. After this call, ANY writes to the .rune
/// section will cause a page fault.
pub unsafe fn seal_rune_section() {
    extern "C" {
        static __rune_start: u8;
        static __rune_end: u8;
    }

    let start = &__rune_start as *const u8 as u64;
    let end = &__rune_end as *const u8 as u64;

    println!("◈ Sealing The Rune of Permanence...");
    println!("  Range: 0x{:016x} - 0x{:016x}", start, end);

    // Iterate over pages in the .rune section
    let mut addr = start & !0xFFF; // Align to 4KB page boundary
    while addr < end {
        // Get page table entry for this address
        let pte = get_pte_for_address(addr);

        // Clear the Write bit (bit 1)
        let mut entry = pte.read();
        entry &= !(1 << 1); // Clear RW bit → read-only
        pte.write(entry);

        // Move to next page
        addr += 0x1000; // 4KB page size
    }

    // Flush TLB to ensure changes take effect immediately
    flush_tlb();

    println!("  ✓ The Rune is sealed. Permanence enforced by the MMU.");
}

/// Flush the Translation Lookaside Buffer (TLB)
///
/// This forces the CPU to reload page table entries from memory,
/// ensuring that our read-only protection takes effect immediately.
#[inline(always)]
fn flush_tlb() {
    unsafe {
        // Reloading CR3 flushes the entire TLB
        core::arch::asm!(
            "mov rax, cr3",
            "mov cr3, rax",
            out("rax") _,
            options(nostack, preserves_flags)
        );
    }
}
```

---

### Phase 4: Linker Script Configuration

Define the `.rune` section in our linker script so permanent structures are grouped together.

#### Update `linker.ld`

```ld
SECTIONS
{
    . = 0x100000; /* Kernel starts at 1MB */

    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    /* NEW: The Rune of Permanence section */
    .rune : ALIGN(4K) {
        PROVIDE(__rune_start = .);
        *(.rune .rune.*)
        PROVIDE(__rune_end = .);
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
    }
}
```

**Key points:**
- `.rune` section is page-aligned (4KB) for MMU protection
- `__rune_start` and `__rune_end` symbols mark the boundaries
- Placed between `.rodata` and `.data` for clarity

---

### Phase 5: Boot Sequence Integration

Integrate sealing into the kernel boot process.

#### Boot Flow

```rust
// In main.rs

fn kernel_main() -> ! {
    // Phase 1: Early initialization (mutable)
    println!("◈ Dawn of Awakening - Kernel Initialization");

    init_vga_buffer();
    init_gdt();
    init_idt();
    init_pic();
    init_timer();
    init_mana_pool();

    // Phase 2: Initialize permanent structures (still mutable)
    println!("◈ Scribing the foundational runes...");

    init_syscall_table();     // Populate function pointers
    init_security_policy();   // Set capability flags
    init_kernel_config();     // Set limits and constants

    // Phase 3: Seal permanent structures (make read-only)
    println!("◈ Invoking The Rune of Permanence...");
    unsafe {
        mana_pool::page_tables::seal_rune_section();
    }
    println!("  ✓ The foundational laws are now immutable.");

    // Phase 4: Start scheduler (permanent structures are now protected)
    println!("◈ Weaving the Loom of Fate...");
    loom_of_fate::init();

    // Phase 5: Enter userspace (if applicable)
    // ...

    loop {
        // Main kernel loop
    }
}
```

**Critical Timing:**
- **Before sealing:** Initialize all structures that go in `.rune`
- **After sealing:** NEVER write to `.rune` - will cause page fault!

---

## Practical Example: Protecting the IDT

The Interrupt Descriptor Table (IDT) is a prime target for attackers. Let's protect it with The Rune of Permanence.

### Current Implementation (Vulnerable)

```rust
// In attunement/idt.rs

// Mutable IDT in .data section (writable!)
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    unsafe {
        // Set up interrupt handlers
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        // ...

        // Load IDT
        IDT.load();
    }
}
```

**Vulnerability:** Attacker with write-what-where primitive can overwrite IDT entries to hijack interrupts.

### Protected Implementation (Permanent)

```rust
// In attunement/idt.rs

// IDT in .rune section (read-only after boot!)
#[link_section = ".rune"]
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    unsafe {
        // During boot: Initialize IDT (still writable)
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        // ...

        // Load IDT
        IDT.load();

        // NOTE: After seal_rune_section() is called,
        // this IDT becomes read-only and cannot be modified!
    }
}
```

**Protection:**
- Attacker can READ the IDT (see handler addresses) → Mitigated by ASLR
- Attacker CANNOT WRITE to IDT → Hardware enforced (page fault)
- Even kernel bugs can't accidentally corrupt IDT after boot

---

## Security Properties

### ✓ Defense-in-Depth

The Rune of Permanence complements existing protections:

| Protection | Defense Against | Layer |
|------------|----------------|-------|
| **W^X** | Code injection | Data ≠ Code |
| **ASLR** | Address prediction | Randomization |
| **Capability Sealing** | Capability forgery | Cryptography |
| **Weaver's Sigil** | Stack overflows | Stack canaries |
| **Rune of Permanence** | Data corruption | Immutability |

### ✓ Hardware Enforcement

- **Not bypassable in software:** MMU enforces at CPU level
- **No performance overhead:** Page permissions checked in hardware
- **Fail-secure:** Write attempts cause immediate page fault

### ✓ Comprehensive Coverage

Protected structures include:
- ✓ IDT (interrupt handlers)
- ✓ GDT (segment descriptors)
- ✓ Syscall dispatch table
- ✓ Security policy flags
- ✓ Kernel configuration constants
- ✓ Function pointer tables

### ⚠ Limitations

**Not Protected:**
- Runtime state (scheduler queues, memory allocators)
- Performance statistics
- Lock state

**Attack Scenarios:**
- **Physical memory access:** Attacker with DMA can modify memory
  - **Mitigation:** Future IOMMU support
- **Speculative execution bugs:** Spectre/Meltdown-class attacks
  - **Mitigation:** CPU microcode updates, software mitigations
- **Kernel exploits before sealing:** If attacker compromises kernel during boot
  - **Mitigation:** Secure boot, measured boot

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_section_exists() {
        extern "C" {
            static __rune_start: u8;
            static __rune_end: u8;
        }

        let start = unsafe { &__rune_start as *const u8 as usize };
        let end = unsafe { &__rune_end as *const u8 as usize };

        assert!(end > start, "Rune section must have non-zero size");
        assert!(start % 0x1000 == 0, "Rune section must be page-aligned");
    }

    #[test]
    #[should_panic]
    fn test_write_to_sealed_rune_causes_fault() {
        #[link_section = ".rune"]
        static mut TEST_VAR: u64 = 0;

        unsafe {
            // Initialize
            TEST_VAR = 42;

            // Seal
            seal_rune_section();

            // Attempt write (should page fault!)
            TEST_VAR = 99; // ← PANIC!
        }
    }
}
```

### Integration Tests

```rust
// In tests/permanence_test.rs

#[test_case]
fn test_idt_cannot_be_modified_after_boot() {
    // Get IDT base address
    let idtr = x86_64::instructions::tables::sidt();
    let idt_base = idtr.base.as_ptr::<u8>();

    // Attempt to write (should fail)
    let result = std::panic::catch_unwind(|| unsafe {
        core::ptr::write_volatile(idt_base, 0xFF);
    });

    assert!(result.is_err(), "Writing to IDT after sealing should panic");
}
```

---

## Implementation Phases

### Phase 1: Linker Script (Week 1)
- [ ] Add `.rune` section to `linker.ld`
- [ ] Add `__rune_start` and `__rune_end` symbols
- [ ] Verify section alignment (4KB pages)
- [ ] Test that section is created and non-empty

### Phase 2: Page Table Infrastructure (Week 1-2)
- [ ] Implement `get_pte_for_address()` function
- [ ] Implement `seal_rune_section()` function
- [ ] Implement `flush_tlb()` function
- [ ] Add debug logging for sealed pages
- [ ] Test on dummy data before real structures

### Phase 3: Move Critical Structures (Week 2-3)
- [ ] Move IDT to `.rune` section
- [ ] Move GDT to `.rune` section (if applicable)
- [ ] Move syscall table to `.rune` (future)
- [ ] Move security policy flags to `.rune`
- [ ] Test each structure after migration

### Phase 4: Boot Integration (Week 3)
- [ ] Add sealing call to `kernel_main()`
- [ ] Ensure all `.rune` structures initialized before sealing
- [ ] Add boot message for sealing
- [ ] Test that kernel boots successfully
- [ ] Verify writes fail after sealing

### Phase 5: Monitoring & Validation (Week 3-4)
- [ ] Add page fault handler logging for .rune violations
- [ ] Add Rune of Permanence status to `wards` command
- [ ] Implement debug command to inspect page permissions
- [ ] Create attack simulations (try to corrupt IDT, etc.)
- [ ] Performance benchmarks (should be zero overhead)

---

## Eldarin Shell Integration

### New Command: `permanence`

Display protected structures and their status:

```
◈ The Rune of Permanence - Immutable Kernel Structures

  Status: ✓ SEALED (hardware-enforced)
  Protection: MMU Read-Only Pages

  Protected Structures:
    .rune section: 0x00120000 - 0x00124000 (16 KB)
      Pages: 4 (all read-only)

    ◈ Interrupt Descriptor Table (IDT)
      Address: 0x00120000
      Size: 4096 bytes
      Status: ✓ Sealed (cannot modify handlers)

    ◈ Global Descriptor Table (GDT)
      Address: 0x00121000
      Size: 256 bytes
      Status: ✓ Sealed (cannot modify segments)

    ◈ Security Policy
      Address: 0x00121100
      Status: ✓ Sealed
        - Capability enforcement: ENABLED (permanent)
        - W^X enforcement: ENABLED (permanent)
        - Stack canaries: ENABLED (permanent)

  Violations Detected: 0 (since boot)
  Last Violation: None

  The foundational laws remain unbroken. The Rune stands eternal.
```

### Update `wards` Command

Add Rune of Permanence to the security overview:

```rust
// In wards_command.rs (Page 1)

crate::println!("  Rune of Permanence (Immutable Structures): ✓ Active");
crate::println!("    Protected: IDT, GDT, Security Policy");
crate::println!("    Pages: 4 read-only");
crate::println!();
```

---

## Page Fault Handler Enhancement

Detect writes to `.rune` section and provide helpful diagnostics:

```rust
// In attunement/idt_handlers.rs

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    // Get the address that caused the fault
    let fault_addr = Cr2::read().as_u64();

    // Check if this was a write to the .rune section
    extern "C" {
        static __rune_start: u8;
        static __rune_end: u8;
    }
    let rune_start = unsafe { &__rune_start as *const u8 as u64 };
    let rune_end = unsafe { &__rune_end as *const u8 as u64 };

    if fault_addr >= rune_start && fault_addr < rune_end {
        if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
            // Write to sealed section!
            panic!(
                "◈ RUNE VIOLATION: Attempt to modify permanent structure!\n\
                 Address: 0x{:016x}\n\
                 Section: .rune (read-only)\n\
                 Error: Write to immutable memory after Dawn of Awakening\n\
                 \n\
                 The Rune of Permanence has been violated. The foundational \n\
                 laws cannot be rewritten. The MMU denies this reality break.",
                fault_addr
            );
        }
    }

    // Standard page fault handling...
    panic!("Page fault at 0x{:016x}: {:?}", fault_addr, error_code);
}
```

---

## Future Enhancements

### Incremental Sealing

Instead of sealing everything at once, seal structures incrementally:

```rust
pub fn seal_idt() {
    seal_range(&IDT as *const _ as u64, size_of::<IDT>());
}

pub fn seal_gdt() {
    seal_range(&GDT as *const _ as u64, size_of::<GDT>());
}
```

**Benefit:** More granular control, easier debugging.

### Runtime Re-initialization (Emergency Mode)

For debugging or emergency patches, temporarily unseal:

```rust
pub unsafe fn unseal_rune_section_emergency(auth_token: &EmergencyAuth) {
    // Verify cryptographic auth token
    if !verify_emergency_auth(auth_token) {
        panic!("Unauthorized unseal attempt!");
    }

    // Log the event
    log::warn!("EMERGENCY: Unsealing .rune section");

    // Temporarily make writable
    // ... (reverse the sealing process)
}
```

**Use case:** Hot-patching critical vulnerabilities without reboot.

### Signature Verification

Before sealing, compute a cryptographic hash of `.rune` section:

```rust
pub fn seal_with_verification() {
    // Compute hash before sealing
    let hash = compute_rune_hash();

    // Seal the section
    seal_rune_section();

    // Store hash in secure location
    store_rune_hash(hash);
}

pub fn verify_rune_integrity() -> bool {
    let current_hash = compute_rune_hash();
    let stored_hash = retrieve_rune_hash();

    constant_time_compare(&current_hash, &stored_hash)
}
```

**Benefit:** Detect if `.rune` was modified before sealing (bootkit attack).

---

## References

- **PaX CONSTIFY:** https://pax.grsecurity.net/docs/constify.txt
- **Linux RODATA:** https://lwn.net/Articles/666550/
- **x86_64 Paging:** Intel SDM Volume 3A, Chapter 4
- **Rust Linker Scripts:** https://doc.rust-lang.org/rustc/codegen-options/index.html#link-args

---

*"The Rune of Permanence is etched not in ink, but in the silicon itself. It is enforced not by software, but by the immutable laws of the MMU. To break it is to break reality—and reality does not yield."*

**Status:** 🚧 Design Complete, Implementation Pending
**Priority:** High (Security Critical)
**Estimated Effort:** 3-4 weeks
**Dependencies:** Functional page table management
