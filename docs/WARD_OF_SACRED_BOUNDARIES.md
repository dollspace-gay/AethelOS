# The Ward of Sacred Boundaries

> *"The Heartwood must never be deceived into treating a mortal's idle thoughts as a true spell,*
> *nor may it touch a cursed scroll without first sanctifying it."*

## Overview

The **Ward of Sacred Boundaries** is AethelOS's implementation of kernel hardening techniques inspired by grsecurity/PaX, specifically SMEP, SMAP, and UDEREF. It enforces a strict separation between the sacred Heartwood (kernel space) and the chaotic mortal lands (user space).

## Philosophy

The Heartwood (kernel space) is a sacred, pure realm. The mortal lands (user space) are chaotic and untrusted. The Ward enforces an absolute separation:

- **SMEP** (Supervisor Mode Execution Prevention): The Heartwood is forbidden from executing code located in the user's memory space. A mortal's idle thoughts cannot become true spells within the sacred realm.

- **SMAP** (Supervisor Mode Access Prevention): When a mortal hands the Heartwood a scroll (a pointer), the Heartwood is forbidden from reading it directly. It must first create a perfect, sanctified copy within its own sacred space.

- **UDEREF** (User Dereference Prevention): All user pointers must be validated and copied through sanctified functions. No kernel code may blindly trust a user pointer.

## Implementation

### Location

[`heartwood/src/attunement/ward_of_sacred_boundaries.rs`](../heartwood/src/attunement/ward_of_sacred_boundaries.rs)

### Components

#### 1. CPU Feature Detection and Enablement

```rust
unsafe fn init_ward() -> Result<(), WardError>
```

- Detects SMEP and SMAP support via CPUID (leaf 0x07)
- Enables SMEP by setting bit 20 in CR4
- Enables SMAP by setting bit 21 in CR4
- Called during kernel initialization (Quest 0 in Grand Attunement)

**CPUID Feature Bits:**
- SMEP: EBX bit 7 (CPUID EAX=0x07, ECX=0x00)
- SMAP: EBX bit 20 (CPUID EAX=0x07, ECX=0x00)

**CR4 Control Bits:**
- SMEP: CR4 bit 20
- SMAP: CR4 bit 21

#### 2. User Pointer Validation

```rust
fn validate_mortal_pointer(addr: u64, size: usize) -> Result<(), WardError>
```

Validates that a pointer and its entire region are in user space:
- User space: `0x0000_0000_0000_0000` to `0x0000_7FFF_FFFF_FFFF`
- Kernel space: `0xFFFF_8000_0000_0000` to `0xFFFF_FFFF_FFFF_FFFF`

**Checks:**
1. Pointer is non-null
2. Pointer is below `KERNEL_SPACE_START` (0xFFFF_8000_0000_0000)
3. Entire region `[addr, addr+size)` is in user space
4. No integer overflow when computing end address

#### 3. Type-Safe Mortal Pointers

```rust
pub struct MortalPointer<T>
```

A zero-cost abstraction that guarantees a pointer points to user space:
- Can only be created through `MortalPointer::new(addr)`, which validates the address
- Once created, the pointer is guaranteed to be in user space
- Prevents accidental use of unvalidated user pointers in kernel code

**Example:**
```rust
let user_addr = 0x10000;
let ptr = MortalPointer::<u64>::new(user_addr)?;
// ptr is now guaranteed to point to user space
```

#### 4. Sanctified Copy Functions

The Heartwood never directly accesses mortal memory. Instead, it performs a "sanctification ritual":

```rust
unsafe fn sanctified_copy_from_mortal<T>(
    mortal_ptr: &MortalPointer<T>,
    dest: &mut T,
) -> Result<(), WardError>
```

**The Sanctification Ritual:**
1. **STAC** (Set AC flag): Temporarily allow supervisor access to user pages
2. **Copy**: Read from user space using `read_volatile`
3. **CLAC** (Clear AC flag): Re-enable SMAP protection

**Why STAC/CLAC?**
- With SMAP enabled, the kernel cannot access user pages by default
- STAC temporarily disables SMAP for intentional, controlled access
- CLAC immediately re-enables protection after the copy
- This prevents accidental kernel→user access (bugs or exploits)

**Copy Operations:**
- `sanctified_copy_from_mortal<T>` - Copy single value from user space
- `sanctified_copy_to_mortal<T>` - Copy single value to user space
- `sanctified_copy_slice_from_mortal<T>` - Copy array from user space
- `sanctified_copy_slice_to_mortal<T>` - Copy array to user space

## Security Benefits

### 1. Prevents Kernel Code Execution (SMEP)

**Attack Prevented:** Kernel exploit jumps to attacker-controlled user space code

**Without SMEP:**
```
Attacker: Map malicious code at 0x1234_5000
Attacker: Exploit kernel bug to overwrite function pointer
Kernel: Calls function pointer → executes at 0x1234_5000 (user space)
Result: Attacker code runs with kernel privileges
```

**With SMEP:**
```
Attacker: Map malicious code at 0x1234_5000
Attacker: Exploit kernel bug to overwrite function pointer
Kernel: Attempts to execute at 0x1234_5000
CPU: #PF (Page Fault) - SMEP violation!
Result: Kernel panic, attack fails
```

### 2. Prevents Unintended Data Access (SMAP/UDEREF)

**Attack Prevented:** Kernel blindly dereferences user-supplied pointer

**Without SMAP:**
```
Attacker: Pass pointer to sensitive kernel data (0xFFFF_8000_1234_0000)
Kernel: Reads from pointer (intended for user buffer)
Result: Attacker reads kernel memory
```

**With SMAP:**
```
Attacker: Pass pointer (malicious or accidental)
Kernel: Attempts direct access
CPU: #PF (Page Fault) - SMAP violation!
Result: Access denied
```

**With UDEREF (Type System):**
```
Attacker: Pass raw pointer
Kernel: Must create MortalPointer::new(addr)
Validation: Checks addr < 0x8000_0000_0000_0000
Result: Invalid pointer rejected before access
```

### 3. Defense in Depth

The Ward provides multiple layers:
1. **Compile-time**: `MortalPointer<T>` type system prevents use of raw pointers
2. **Runtime validation**: `validate_mortal_pointer()` checks address range
3. **CPU enforcement**: SMEP/SMAP enforce at hardware level
4. **Controlled access**: STAC/CLAC make access explicit and auditable

## Initialization Sequence

The Ward is initialized as **Quest 0** in the Grand Attunement:

```
◈ Beginning the Grand Attunement...
  ⟡ Quest 0: Raising the Ward of Sacred Boundaries (SMEP/SMAP)...
     [WARD] Initializing Ward of Sacred Boundaries...
     [WARD] ✓ SMEP (Supervisor Mode Execution Prevention) enabled
     [WARD] ✓ SMAP (Supervisor Mode Access Prevention) enabled
     [WARD] ✓ The Ward of Sacred Boundaries stands vigilant
     ✓ The Ward stands vigilant
  ⟡ Quest 1: Establishing privilege boundaries (GDT & TSS)...
  ...
```

## Usage Examples

### Validating and Reading a User Pointer

```rust
use crate::attunement::ward_of_sacred_boundaries::{
    MortalPointer, sanctified_copy_from_mortal
};

// Kernel receives a user space address
let user_addr: u64 = 0x1000_0000;

// Create validated pointer
let mortal_ptr = match MortalPointer::<u64>::new(user_addr) {
    Ok(ptr) => ptr,
    Err(e) => {
        println!("Invalid user pointer: {}", e);
        return Err(SyscallError::InvalidPointer);
    }
};

// Sanctify the data (copy to kernel space)
let mut value: u64 = 0;
unsafe {
    sanctified_copy_from_mortal(&mortal_ptr, &mut value)?;
}

// Now 'value' contains a safe copy of the user's data
println!("User provided value: {}", value);
```

### Copying an Array from User Space

```rust
use crate::attunement::ward_of_sacred_boundaries::sanctified_copy_slice_from_mortal;

// User provides a buffer of 10 u32 values
let user_buffer_addr: u64 = 0x2000_0000;
let count = 10;

// Prepare kernel buffer
let mut kernel_buffer = [0u32; 10];

// Sanctify the entire array
unsafe {
    sanctified_copy_slice_from_mortal(user_buffer_addr, &mut kernel_buffer)?;
}

// kernel_buffer now contains a safe copy
for (i, value) in kernel_buffer.iter().enumerate() {
    println!("buffer[{}] = {}", i, value);
}
```

### Writing Data to User Space

```rust
use crate::attunement::ward_of_sacred_boundaries::{
    MortalPointer, sanctified_copy_to_mortal
};

// Kernel wants to return a result to user space
let result: u32 = 42;
let user_result_addr: u64 = 0x3000_0000;

// Validate user pointer
let mortal_ptr = MortalPointer::<u32>::new(user_result_addr)?;

// Copy result to user space
unsafe {
    sanctified_copy_to_mortal(&result, &mortal_ptr)?;
}

println!("Result written to user space");
```

## Error Handling

```rust
pub enum WardError {
    PointerInKernelSpace,  // Pointer ≥ 0xFFFF_8000_0000_0000
    NullPointer,           // Pointer == 0
    RegionOverflow,        // Region crosses into kernel space
    UnsupportedCpu,        // CPU lacks SMEP/SMAP
    CopyFailed,            // Copy operation failed
}
```

**Best Practices:**
- Always check `WardError` results
- Log validation failures for debugging
- Return syscall errors to user on validation failure
- Never `unwrap()` Ward operations - always handle errors

## CPU Compatibility

**Requirements:**
- **SMEP**: Intel since Ivy Bridge (2012), AMD since Zen (2017)
- **SMAP**: Intel since Broadwell (2014), AMD since Zen 3 (2020)

**Fallback Behavior:**
- If SMEP/SMAP are unsupported, Ward initialization logs a warning
- Validation and copy functions still work (software checks only)
- Provides defense in depth even on older CPUs

### QEMU CPU Models

**QEMU's default CPU (`qemu64`) does NOT include SMEP/SMAP!**

To test the Ward in QEMU, use a modern CPU model:

```bash
# Option 1: Intel Broadwell (recommended)
qemu-system-x86_64 -cpu Broadwell -cdrom aethelos.iso

# Option 2: Explicit feature flags
qemu-system-x86_64 -cpu qemu64,+smep,+smap -cdrom aethelos.iso

# Option 3: Use host CPU (requires KVM)
qemu-system-x86_64 -cpu host -enable-kvm -cdrom aethelos.iso
```

**QEMU CPU Feature Support:**

| CPU Model | SMEP | SMAP | Command |
|-----------|------|------|---------|
| `qemu64` (default) | ❌ | ❌ | Don't use |
| `IvyBridge` | ✅ | ❌ | `-cpu IvyBridge` |
| `Haswell` | ✅ | ❌ | `-cpu Haswell` |
| `Broadwell` | ✅ | ✅ | `-cpu Broadwell` ← **Recommended** |
| `Skylake-Client` | ✅ | ✅ | `-cpu Skylake-Client` |
| `host` | ✅* | ✅* | `-cpu host -enable-kvm` (KVM only) |

*Depends on your physical CPU

**Checking Feature Support:**
```rust
use crate::attunement::ward_of_sacred_boundaries::is_ward_enabled;

if is_ward_enabled() {
    println!("✓ Ward fully active (SMEP/SMAP enabled)");
} else {
    println!("⚠ Ward using software checks only");
}
```

## Testing

### Unit Tests

Tests validate address range checks:
```rust
#[test]
fn test_mortal_pointer_validation() {
    // Valid user space
    assert!(is_mortal_pointer(0x1000));

    // Invalid kernel space
    assert!(!is_mortal_pointer(0xFFFF_8000_0000_0000));

    // Invalid null
    assert!(!is_mortal_pointer(0));
}
```

### Integration Testing

To test the Ward in QEMU:
1. Boot AethelOS
2. Check boot messages for Ward initialization
3. Create user space program that tries to:
   - Pass invalid pointers to syscalls
   - Map code and try to trick kernel into executing it
4. Verify violations are caught

## Future Enhancements

### Planned Features

1. **PXN (Privileged Execute Never)**: ARM equivalent of SMEP
2. **PAN (Privileged Access Never)**: ARM equivalent of SMAP
3. **KPTI (Kernel Page Table Isolation)**: Separate page tables for user/kernel
4. **Stack Canaries for User Copies**: Detect buffer overflows during copy
5. **Audit Logging**: Log all STAC/CLAC operations for security analysis

### Research Ideas

- **Capability-based Mortal Pointers**: Embed permissions in pointer type
- **Ephemeral Keys**: Encrypt user data during copy to prevent TOCTOU attacks
- **Hardware Transactional Memory**: Use Intel TSX for atomic sanctification

## References

- [Intel SDM Volume 3, Section 4.6: Access Rights](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [grsecurity UDEREF](https://grsecurity.net/)
- [PaX KERNEXEC and UDEREF](https://pax.grsecurity.net/)
- [Linux Kernel SMAP Support](https://lwn.net/Articles/517475/)
- [Windows SMEP/SMAP Implementation](https://www.microsoft.com/en-us/security/blog/2020/07/08/introducing-kernel-data-protection-a-new-platform-security-technology/)

## Contributing

When modifying the Ward:
1. **Never bypass validation** - All user pointers must use `MortalPointer<T>`
2. **Always use sanctified_copy** - Never use `read_volatile` on user pointers directly
3. **Pair STAC/CLAC** - Every STAC must have a matching CLAC
4. **Test on real hardware** - QEMU may not fully emulate SMEP/SMAP
5. **Update bd tracker** - Track all Ward changes in issue system

---

*"The boundary between sacred and profane is absolute. The Ward ensures it remains so."*

**Status:** ✅ Implemented and Active
**Version:** 1.0.0
**Last Updated:** January 2025
