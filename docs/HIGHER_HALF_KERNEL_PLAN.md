# Higher-Half Kernel Implementation Plan

**Status:** Planning
**Priority:** P0 - Blocks user-space execution
**Issue:** OS-77
**Date:** 2025-01-30

---

## Executive Summary

AethelOS currently runs its kernel in identity-mapped memory (virtual 0x0-0x40000000 = physical 0x0-0x40000000), which conflicts with user-space programs that also need low virtual addresses. This causes CR3 page table switches to fail when transitioning to ring 3.

**Solution:** Move the kernel to the "higher-half" of virtual address space at `0xFFFF_8000_0000_0000+` while keeping physical load address at 1MB. This is the standard x86-64 kernel architecture used by Linux, FreeBSD, and others.

---

## Problem Statement

### Current Memory Layout

```
Virtual Address Space (PML4 Entry 0):
┌─────────────────────────────────────────┐
│ 0x000000 - 0x100000  │ Identity: Low mem│
│ 0x100000 - 0x400000  │ Kernel code/data │ ← CONFLICT!
│ 0x400000 - 0x7FFFFF  │ Kernel heap      │ ← CONFLICT!
│ ...                   │                  │
└─────────────────────────────────────────┘

User Program Address Space (Also needs PML4 Entry 0):
┌─────────────────────────────────────────┐
│ 0x400000 - 0x500000  │ User code        │ ← CONFLICT!
│ 0x7FFFFF00000 - ...  │ User stack       │
└─────────────────────────────────────────┘
```

### Symptom

Context switch to ring 3 hangs with diagnostic output:
```
[YIELD] to_ctx: RIP=0x400000, RSP=0x7ffffffefff8, CR3=0xbf7000
C      ← Entered context switch code
       ← No 'R' = CR3 switch fails
```

The new page table at physical 0xbf7000 cannot simultaneously map:
- Kernel code (needed for context switch trampoline)
- User program at 0x400000

### Root Cause

Both kernel and user space compete for PML4 entry [0], which covers virtual addresses 0x0000_0000_0000_0000 to 0x0000_7FFF_FFFF_FFFF.

---

## Solution: Higher-Half Kernel

### Target Memory Layout

```
Virtual Address Space:

PML4 Entry [0]: User Space
┌─────────────────────────────────────────┐
│ 0x0000_0040_0000 - ...│ User code/data  │
│ 0x0000_7FFF_FFFF_F000 │ User stack      │
└─────────────────────────────────────────┘

PML4 Entry [256]: Kernel Space (Higher Half)
┌─────────────────────────────────────────┐
│ 0xFFFF_8000_0010_0000 │ Kernel code     │
│ 0xFFFF_8000_0040_0000 │ Kernel heap     │
│ ...                    │                 │
└─────────────────────────────────────────┘
```

### Benefits

1. **No Conflicts:** User space and kernel use different PML4 entries
2. **Standard Architecture:** Matches Linux (0xFFFF_8000_0000_0000+) and others
3. **Easy Context Switch:** Copy only PML4 entries [256-511] to user page tables
4. **Security:** Clear separation between kernel and user addresses

---

## Implementation Phases

### Phase 1: Linker Script Changes ✅

**File:** `heartwood/linker.ld`

**Change:** Separate virtual addresses (VMA) from physical load addresses (LMA).

**Before:**
```ld
SECTIONS
{
    . = 1M;  /* Both VMA and LMA */

    .text ALIGN(4K) : AT(ADDR(.text))
    {
        *(.text .text.*)
    }
}
```

**After:**
```ld
SECTIONS
{
    /* Physical load address: 1MB (where GRUB loads kernel) */
    . = 1M;
    __kernel_physical_start = .;

    /* Virtual base address: Higher half */
    . += 0xFFFF800000000000;
    __kernel_virtual_start = .;

    .text ALIGN(4K) : AT(__kernel_physical_start + (ADDR(.text) - __kernel_virtual_start))
    {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(__kernel_physical_start + (ADDR(.rodata) - __kernel_virtual_start))
    {
        *(.rodata .rodata.*)
    }

    /* ... similar for .data, .bss, etc ... */
}
```

**Key Points:**
- `LMA = 0x100000 + offset` (where GRUB loads)
- `VMA = 0xFFFF_8000_0010_0000 + offset` (where kernel executes)
- Boot code stays at low 1MB for initial execution

---

### Phase 2: Boot Code Page Table Setup ✅

**File:** `heartwood/src/boot/boot32.rs`

**Change:** Set up both identity mapping (temporary) and higher-half mapping (permanent).

**Assembly Changes:**

```x86asm
boot32_start:
    # ... existing stack and serial setup ...

    # Set up page tables for BOTH identity and higher-half mapping
    # Clear 24KB area: 0x70000-0x75FFF
    mov edi, 0x70000
    mov ecx, 0x1800   # 6144 dwords = 24KB
    xor eax, eax
    rep stosd

    # PML4 setup:
    # - Entry [0] -> PDPT at 0x71000 (identity mapping, temporary)
    # - Entry [256] -> PDPT at 0x73000 (higher-half, permanent)
    mov dword ptr [0x70000], 0x71003      # PML4[0] -> PDPT (identity)
    mov dword ptr [0x70800], 0x73003      # PML4[256] -> PDPT (higher-half)

    # PDPT for identity mapping (entry 0): Maps first 1GB
    mov dword ptr [0x71000], 0x72003      # PDPT[0] -> PD at 0x72000

    # PDPT for higher-half (entry 256): Maps first 1GB at higher-half
    mov dword ptr [0x73000], 0x74003      # PDPT[0] -> PD at 0x74000

    # PD for identity mapping: 512 * 2MB huge pages = 1GB
    mov edi, 0x72000
    mov eax, 0x83       # Present + Write + Huge
    mov ecx, 512
1:
    mov [edi], eax
    add eax, 0x200000   # Next 2MB page
    add edi, 8
    loop 1b

    # PD for higher-half: Same physical memory, different virtual address
    mov edi, 0x74000
    mov eax, 0x83       # Present + Write + Huge
    mov ecx, 512
2:
    mov [edi], eax
    add eax, 0x200000   # Next 2MB page
    add edi, 8
    loop 2b

    # Load CR3 with PML4 address
    mov eax, 0x70000
    mov cr3, eax

    # ... existing PAE, EFER, paging enable code ...

    # Jump to 64-bit code
    push 0x08
    lea eax, [boot64_start]
    push eax
    retf

# 64-bit entry point
.code64
boot64_start:
    # ... existing segment setup ...

    # IMPORTANT: We're now executing at HIGHER-HALF addresses!
    # Load higher-half stack address
    mov rsp, 0xFFFF800000400000  # 4MB in higher-half
    mov rbp, rsp

    # ... existing serial/VGA writes ...

    # Call Rust entry point (now at higher-half address)
    call _start
```

**Key Changes:**
1. Allocate 24KB for page tables (was 16KB)
2. Set up PML4[0] for identity mapping (temporary)
3. Set up PML4[256] for higher-half kernel (permanent)
4. Both mappings point to same physical memory (first 1GB)
5. Stack pointer changed to higher-half address

---

### Phase 3: Early Kernel Initialization ✅

**File:** `heartwood/src/main.rs`

**Change:** Remove identity mapping after entering Rust code.

**Add function:**
```rust
/// Remove identity mapping after boot
///
/// We need identity mapping during boot to execute low-address code,
/// but once we're running at higher-half addresses, we should remove
/// it to free PML4[0] for user space.
unsafe fn remove_identity_mapping() {
    use core::arch::asm;

    // Get current PML4
    let pml4_phys: u64;
    asm!("mov {}, cr3", out(reg) pml4_phys, options(nomem, nostack));

    // Convert to virtual address (bootloader identity-maps first 1GB)
    let pml4 = &mut *(pml4_phys as *mut crate::mana_pool::page_tables::PageTable);

    // Clear PML4[0] to remove identity mapping
    *pml4.entry_mut(0) = crate::mana_pool::page_tables::PageTableEntry::new();

    // Flush TLB
    asm!("mov cr3, {}", in(reg) pml4_phys, options(nostack));

    crate::serial_println!("[INIT] Identity mapping removed - higher-half only");
}
```

**In `_start()` function:**
```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // ... existing initialization ...

    // Initialize allocator FIRST (uses identity-mapped addresses)
    init_global_allocator();
    serial_println!("◈ Allocator initialized");

    // NOW remove identity mapping - we're fully in higher-half
    unsafe {
        remove_identity_mapping();
    }

    // ... rest of initialization ...
}
```

---

### Phase 4: Update `phys_to_virt()` Helper ✅

**File:** `heartwood/src/mana_pool/page_tables.rs`

**Change:** Update physical-to-virtual address translation.

**Current (temporary workaround):**
```rust
unsafe fn phys_to_virt(phys: u64) -> u64 {
    if phys < 0x4000_0000 {
        phys  // Identity mapping
    } else {
        phys + 0xFFFF_8000_0000_0000  // Higher-half
    }
}
```

**After Phase 1-3 complete:**
```rust
/// Convert physical address to kernel virtual address
///
/// After higher-half kernel migration, ALL kernel accesses use higher-half mapping.
/// Physical memory 0x0 - 0x3FFFFFFF maps to virtual 0xFFFF_8000_0000_0000+
unsafe fn phys_to_virt(phys: u64) -> u64 {
    const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;
    KERNEL_BASE + phys
}
```

**Update ALL functions in `page_tables.rs` that dereference page table addresses:**
- `clone_kernel_page_table()`
- `map_user_page()`
- `walk_page_tables()`
- `debug_page_mapping()`
- `make_huge_page_readonly()`
- `make_readonly()`

---

### Phase 5: Update Heap Initialization ✅

**File:** `heartwood/src/lib.rs` (or wherever `init_global_allocator()` is)

**Change:** Use higher-half addresses for heap region.

**Current:**
```rust
pub fn init_global_allocator() {
    unsafe {
        const HEAP_START: usize = 0x400000;  // 4MB (identity-mapped)
        const HEAP_SIZE: usize = 0x800000;   // 8MB
        GLOBAL_ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    }
}
```

**After:**
```rust
pub fn init_global_allocator() {
    unsafe {
        // Heap in higher-half address space
        const KERNEL_BASE: usize = 0xFFFF_8000_0000_0000;
        const HEAP_START: usize = KERNEL_BASE + 0x400000;  // 4MB in higher-half
        const HEAP_SIZE: usize = 0x800000;   // 8MB
        GLOBAL_ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    }
}
```

---

### Phase 6: Fix User-Space Context Switching ✅

**File:** `heartwood/src/mana_pool/user_space.rs` (and `page_tables.rs`)

**Change:** Clone only higher-half kernel mappings to user page tables.

**In `clone_kernel_page_table()`:**
```rust
pub unsafe fn clone_kernel_page_table() -> Result<u64, UserSpaceError> {
    // ... allocate new PML4 ...

    let kernel_pml4_phys = read_cr3();
    let kernel_pml4_virt = phys_to_virt(kernel_pml4_phys);
    let kernel_pml4 = &*(kernel_pml4_virt as *const PageTable);

    // Copy ONLY higher-half entries [256-511] to user page table
    // Leave entries [0-255] empty for user space
    for i in 256..512 {
        *new_pml4.entry_mut(i) = kernel_pml4.entry(i);
    }

    serial_println!("[CLONE_PML4] Cloned kernel entries [256-511] to user page table");

    Ok(new_pml4_phys)
}
```

**Key Points:**
- User space gets PML4 entries [0-255] for user memory
- Kernel entries [256-511] are copied to every user page table
- CR3 switch now succeeds because kernel code is mapped at higher-half

---

### Phase 7: Update Interrupt Handlers (If Needed) ⚠️

**Files:** `heartwood/src/attunement/idt.rs`, `keyboard.rs`, etc.

**Check:** Most interrupt handlers should work unchanged since they use kernel functions that will automatically resolve to higher-half addresses.

**Verify:**
- No hardcoded low addresses (e.g., `0x100000`)
- All pointer dereferences go through proper virtual addresses
- Serial port I/O still works (uses I/O ports, not memory addresses)

---

## Testing Strategy

### Phase 1 Testing: Linker Script

1. **Build kernel:**
   ```bash
   cd heartwood
   cargo build --target x86_64-aethelos.json
   ```

2. **Check symbol addresses:**
   ```bash
   objdump -t target/x86_64-aethelos/debug/heartwood | grep _start
   # Should show: 0xffff800000100xxx _start (higher-half address)
   ```

3. **Verify load addresses:**
   ```bash
   readelf -l target/x86_64-aethelos/debug/heartwood
   # Physical addresses should be ~0x100000
   # Virtual addresses should be ~0xFFFF800000100000
   ```

### Phase 2 Testing: Boot Code

1. **Build and create ISO**
2. **Boot in QEMU**
3. **Expected serial output:**
   ```
   BSLSR    ← All bootstrap characters should appear
   ```
4. **Check for triple fault:** If it doesn't print all characters, page table setup is wrong

### Phase 3 Testing: Remove Identity Mapping

1. **After Phase 2 succeeds, add identity removal code**
2. **Boot in QEMU**
3. **Expected serial output:**
   ```
   [INIT] Identity mapping removed - higher-half only
   ◈ Loom of Fate initialized
   ```
4. **Test:** Try accessing low address (should page fault)

### Phase 4-6 Testing: User Space

1. **Complete all phases**
2. **Boot in QEMU**
3. **Run shell command that spawns user thread**
4. **Expected serial output:**
   ```
   [YIELD] to_ctx: RIP=0x400000, RSP=0x7FFFFFFEFFF8, CR3=0xbf7000
   CRI   ← All three characters (C=enter, R=CR3 switched, I=interrupts off)
   ```
5. **Success:** User program executes without hang

---

## Rollback Plan

If higher-half migration fails catastrophically:

1. **Revert linker script** to original (1MB flat layout)
2. **Revert boot32.rs** to single identity mapping
3. **Revert `phys_to_virt()`** to identity passthrough
4. **Document issue** in OS-77 with specific failure mode
5. **Alternative:** Try "split identity/higher-half" approach:
   - Keep kernel identity-mapped
   - Use trampoline code for context switches
   - Accepted as interim solution until full migration feasible

---

## Constants and Definitions

```rust
/// Kernel virtual base address (higher-half)
pub const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;

/// PML4 index for kernel space (entry 256)
pub const KERNEL_PML4_START: usize = 256;

/// PML4 index for user space (entries 0-255)
pub const USER_PML4_END: usize = 255;

/// Physical address where kernel is loaded by GRUB
pub const KERNEL_PHYSICAL_BASE: u64 = 0x100000;  // 1MB

/// Convert physical address to kernel virtual address
#[inline]
pub const fn phys_to_virt(phys: u64) -> u64 {
    KERNEL_BASE + phys
}

/// Convert kernel virtual address to physical address
#[inline]
pub const fn virt_to_phys(virt: u64) -> u64 {
    virt - KERNEL_BASE
}
```

---

## References

### External Documentation

- [OSDev Wiki: Higher Half x86-64](https://wiki.osdev.org/Higher_Half_x86_Bare_Bones)
- [Linux x86-64 memory map](https://www.kernel.org/doc/Documentation/x86/x86_64/mm.txt)
- [Intel SDM Vol 3A: Paging](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)

### Internal Documentation

- [AethelOS Design Philosophy](../DESIGN.md)
- [Mana Pool Architecture](../README.md#mana-pool)
- [Production Readiness Plan](./PRODUCTION_READINESS_PLAN.md)

---

## Implementation Checklist

- [ ] **Phase 1:** Update `linker.ld` with higher-half VMAs
- [ ] **Phase 2:** Update `boot32.rs` with dual page table mapping
- [ ] **Phase 3:** Add `remove_identity_mapping()` function
- [ ] **Phase 4:** Update `phys_to_virt()` to use consistent higher-half offset
- [ ] **Phase 5:** Update heap initialization to use higher-half addresses
- [ ] **Phase 6:** Fix `clone_kernel_page_table()` to copy entries [256-511] only
- [ ] **Phase 7:** Audit interrupt handlers for hardcoded addresses
- [ ] **Testing:** Verify each phase independently
- [ ] **Testing:** Confirm user-space context switch works (C-R-I characters)
- [ ] **Documentation:** Update CLAUDE.md with new memory layout
- [ ] **Cleanup:** Remove workaround code and temporary diagnostics

---

## Notes

### Why 0xFFFF_8000_0000_0000?

This is the canonical address for kernel space on x86-64:
- Matches Linux kernel convention
- Falls in higher half of 64-bit address space
- Canonical addresses must have bits 48-63 all same (either all 1s or all 0s)
- `0xFFFF_8xxx_xxxx_xxxx` is canonical (bits 48-63 all 1s)

### Why Not Recursively Map Page Tables?

Recursive mapping is clever but adds complexity:
- Requires reserving specific PML4 entry
- Makes page table walking non-obvious
- Complicates debugging

We use direct physical-to-virtual translation instead (`phys_to_virt()`).

### Stack Considerations

Stack is also in higher-half:
- Boot: RSP = `0xFFFF_8000_0040_0000` (4MB in higher-half)
- Thread stacks: Allocated from heap, automatically higher-half
- User stacks: Low addresses (PML4 entry 0)

---

**End of Plan**
