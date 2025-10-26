# Capability Sealing - Preventing Forgery in AethelOS

## Problem Statement

In capability-based security, capabilities must be **unforgeable**. A malicious program should not be able to:
- Create new capabilities with arbitrary permissions
- Modify existing capabilities to escalate privileges
- Forge capability IDs to access resources they don't own

## Current Vulnerability

Our current `Capability` struct is just a bitflags wrapper:
```rust
pub struct Capability {
    rights: CapabilityRights, // Just a u32 bitfield
}
```

If user space could construct this directly, they could forge any permission level.

## Defense Layers

### Layer 1: Opaque Handles (Primary Defense)

**Concept:** User space never sees actual capabilities, only opaque handles.

```rust
/// Opaque capability identifier - user space only sees this
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(u64);

/// Sealed capability - only constructible by kernel
pub struct SealedCapability {
    /// Public ID (user space sees this)
    id: CapabilityId,

    /// Actual rights (user space NEVER sees this)
    rights: CapabilityRights,

    /// Object this capability grants access to
    object_id: ObjectId,

    /// Cryptographic seal (HMAC of id + rights + object_id)
    seal: [u8; 32],

    /// Generation counter (for revocation)
    generation: u64,
}
```

**How it works:**
1. Kernel creates capability and stores in per-process capability table
2. User space receives only `CapabilityId` (e.g., `CapabilityId(42)`)
3. When user calls syscall with `CapabilityId(42)`, kernel:
   - Looks up actual `SealedCapability` from table
   - Validates the seal
   - Checks permissions
   - Performs operation

**Analogies:**
- Like Linux file descriptors (fd 3 means nothing without kernel lookup)
- Like seL4 capability spaces
- Like handle-based APIs (HANDLE in Windows)

### Layer 2: Cryptographic Sealing (Secondary Defense)

**Concept:** Even if capability table is compromised, capabilities can't be forged without kernel secret.

```rust
/// Kernel-only secret key for sealing capabilities
static SEAL_KEY: OnceCell<[u8; 32]> = OnceCell::new();

impl SealedCapability {
    /// Create a new sealed capability (kernel-only)
    pub(crate) fn new(id: CapabilityId, rights: CapabilityRights, object_id: ObjectId) -> Self {
        let seal = Self::compute_seal(id, rights, object_id);
        Self {
            id,
            rights,
            object_id,
            seal,
            generation: 0,
        }
    }

    /// Compute HMAC-SHA256 seal
    fn compute_seal(id: CapabilityId, rights: CapabilityRights, object_id: ObjectId) -> [u8; 32] {
        let key = SEAL_KEY.get().expect("Seal key not initialized");

        // HMAC-SHA256(key, id || rights || object_id || generation)
        let mut hasher = HmacSha256::new(key);
        hasher.update(&id.0.to_le_bytes());
        hasher.update(&rights.bits().to_le_bytes());
        hasher.update(&object_id.0.to_le_bytes());
        hasher.finalize()
    }

    /// Validate seal before using capability
    pub(crate) fn validate(&self) -> Result<(), SecurityViolation> {
        let expected_seal = Self::compute_seal(self.id, self.rights, self.object_id);

        if constant_time_compare(&self.seal, &expected_seal) {
            Ok(())
        } else {
            Err(SecurityViolation::CapabilityForgery)
        }
    }
}
```

**How it works:**
1. At boot, kernel generates random 256-bit seal key (using RDRAND/ChaCha8)
2. Key never leaves kernel, never exposed to user space
3. Every capability is sealed with HMAC-SHA256(key, capability_data)
4. Before using capability, kernel validates seal
5. Without key, attacker can't forge valid seal

**Benefits:**
- Defense-in-depth: Even if capability table is somehow leaked, forged caps fail validation
- Integrity protection: Tampering with rights/object_id invalidates seal
- Cryptographically secure: Infeasible to forge without key

### Layer 3: Type System Enforcement (Tertiary Defense)

**Concept:** Use Rust's type system to prevent capability construction outside kernel.

```rust
/// Capability rights - construction is private
pub struct CapabilityRights(u32);

impl CapabilityRights {
    /// Private constructor - only kernel can create
    pub(crate) const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// No public constructor!
    // (user code can't do CapabilityRights::new())
}

/// Sealed capability - all fields private, no public constructor
pub struct SealedCapability {
    id: CapabilityId,           // Private
    rights: CapabilityRights,   // Private
    object_id: ObjectId,        // Private
    seal: [u8; 32],             // Private
    generation: u64,            // Private
}

// No Debug/Clone for SealedCapability to prevent leakage
```

**How it works:**
- User space can't construct `SealedCapability` (no public constructor)
- User space can't access fields (all private)
- User space can only receive `CapabilityId` and pass it back to kernel

### Layer 4: Memory Isolation (Future - With User Space)

**Concept:** Separate kernel and user address spaces.

```
Kernel Space (Ring 0):
  - Capability tables
  - Seal key
  - ObjectManager

User Space (Ring 3):
  - Only has CapabilityId values
  - Can't access kernel memory
  - Page tables enforced by MMU
```

**How it works:**
- User space runs in Ring 3 with restricted page tables
- Capability tables live in kernel-only memory
- Syscalls cross privilege boundary to access capabilities
- MMU prevents direct memory access

## Implementation Architecture

```
┌────────────────────────────────────────────────────┐
│                   User Space                       │
│                                                    │
│  CapabilityId(42)  CapabilityId(17)               │
│       ↓                  ↓                         │
└───────┼──────────────────┼─────────────────────────┘
        │                  │
     syscall           syscall
        │                  │
┌───────┼──────────────────┼─────────────────────────┐
│       ↓                  ↓          Kernel Space   │
│  ┌──────────────────────────────────────┐          │
│  │  Per-Process Capability Table         │          │
│  │                                       │          │
│  │  42 → SealedCapability {              │          │
│  │         rights: READ|WRITE,           │          │
│  │         object: MemoryObject(0x1000), │          │
│  │         seal: [crypto hash],          │          │
│  │       }                               │          │
│  │                                       │          │
│  │  17 → SealedCapability {              │          │
│  │         rights: READ|EXECUTE,         │          │
│  │         object: MemoryObject(0x5000), │          │
│  │         seal: [crypto hash],          │          │
│  │       }                               │          │
│  └──────────────────────────────────────┘          │
│                                                    │
│  Validation Flow:                                 │
│  1. Lookup capability by ID                       │
│  2. Validate cryptographic seal                   │
│  3. Check rights match requested operation        │
│  4. Perform operation if valid                    │
│                                                    │
└────────────────────────────────────────────────────┘
```

## Capability Operations

### Creating a Capability

```rust
// In kernel, when allocating memory:
pub fn allocate_memory(size: usize, process: &mut Process) -> Result<CapabilityId, Error> {
    // 1. Allocate the actual memory object
    let object_id = OBJECT_MANAGER.lock().create_memory_object(size)?;

    // 2. Create sealed capability
    let cap_id = process.next_capability_id();
    let rights = CapabilityRights::READ | CapabilityRights::WRITE;
    let sealed_cap = SealedCapability::new(cap_id, rights, object_id);

    // 3. Store in process capability table
    process.capability_table.insert(cap_id, sealed_cap);

    // 4. Return only the ID to user space
    Ok(cap_id)
}
```

### Using a Capability

```rust
// Syscall: read from memory using capability
pub fn sys_memory_read(cap_id: CapabilityId, offset: usize, buffer: &mut [u8]) -> Result<(), Error> {
    let process = current_process();

    // 1. Lookup capability (prevents forgery - user can't access table)
    let sealed_cap = process.capability_table
        .get(&cap_id)
        .ok_or(Error::InvalidCapability)?;

    // 2. Validate seal (prevents tampering)
    sealed_cap.validate()
        .map_err(|_| Error::CapabilityForgery)?;

    // 3. Check rights
    if !sealed_cap.rights.contains(CapabilityRights::READ) {
        return Err(Error::PermissionDenied);
    }

    // 4. Perform operation
    let object = OBJECT_MANAGER.lock()
        .get_object(sealed_cap.object_id)?;
    object.read(offset, buffer)
}
```

### Deriving a Capability (Attenuation)

```rust
// User wants to pass read-only version of their read-write capability
pub fn sys_capability_derive(
    parent_id: CapabilityId,
    new_rights: CapabilityRights,
) -> Result<CapabilityId, Error> {
    let process = current_process();

    // 1. Lookup parent capability
    let parent = process.capability_table
        .get(&parent_id)
        .ok_or(Error::InvalidCapability)?;

    // 2. Validate parent seal
    parent.validate()
        .map_err(|_| Error::CapabilityForgery)?;

    // 3. Check attenuation (can only reduce rights, never increase)
    if !parent.rights.contains(new_rights) {
        return Err(Error::CannotAmplifyRights);
    }

    // 4. Create new sealed capability
    let child_id = process.next_capability_id();
    let child = SealedCapability::new(child_id, new_rights, parent.object_id);

    // 5. Store and return
    process.capability_table.insert(child_id, child);
    Ok(child_id)
}
```

## Security Properties

### Achieved Properties

1. **Unforgeable**: User space can't create valid `CapabilityId` that passes validation
2. **Tamper-Proof**: Modifying capability data breaks cryptographic seal
3. **Non-Amplifiable**: Can only derive capabilities with fewer rights (attenuation)
4. **Revocable**: Incrementing generation counter invalidates all old capabilities
5. **Isolated**: Capability tables live in kernel-only memory

### Attack Scenarios

| Attack | Defense |
|--------|---------|
| Forge arbitrary CapabilityId | Lookup fails - ID not in table |
| Guess valid CapabilityId | 64-bit space, random generation → infeasible |
| Modify rights in memory | Seal validation fails |
| Replay old capability | Generation counter mismatch |
| Steal another process's capability | Per-process capability tables |
| Extract seal key | Key never leaves kernel, in protected memory |

## Implementation Phases

### Phase 1: Opaque Handles (Current Priority)
- Replace direct `Capability` exposure with `CapabilityId`
- Add `CapabilityTable` struct to ObjectManager
- Update Mana Pool to use capability lookup

### Phase 2: Cryptographic Sealing
- Implement HMAC-SHA256 (or use ChaCha8-based MAC)
- Generate seal key at boot
- Add seal validation to all capability operations

### Phase 3: Per-Process Tables
- Move capability tables from global to per-process
- Implement proper process isolation

### Phase 4: Hardware Isolation
- Implement user/kernel space separation (Ring 3/Ring 0)
- Page table-based memory protection
- Syscall interface for capability operations

## Comparison to Other Systems

| System | Approach | Strength |
|--------|----------|----------|
| **seL4** | Capability spaces + hardware isolation | Formally verified, hardware-backed |
| **CHERI** | Tagged pointers in hardware | Hardware enforcement, very fast |
| **Unix** | File descriptors (opaque handles) | Simple, proven design |
| **AethelOS (Proposed)** | Opaque handles + crypto sealing | Layered defense, practical for x86-64 |

## References

- seL4 Capability Model: https://sel4.systems/Info/Docs/seL4-manual.pdf
- CHERI ISA: https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-951.pdf
- Capability Myths Demolished: https://srl.cs.jhu.edu/pubs/SRL2003-02.pdf
- L4 Capabilities: http://www.cse.unsw.edu.au/~cs9242/02/lectures/05-caps.pdf

## Next Steps

1. Implement `CapabilityId` and `SealedCapability` structs
2. Add `CapabilityTable` to ObjectManager
3. Update Mana Pool to use capability lookup
4. Implement HMAC-SHA256 or ChaCha8-MAC
5. Add seal validation
6. Test with capability forgery attempts
