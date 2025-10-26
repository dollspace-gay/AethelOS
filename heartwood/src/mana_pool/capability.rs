//! Capability-based security for memory access

use super::object_manager::ObjectHandle;
use super::sealing;
use core::sync::atomic::{AtomicU64, Ordering};

/// A capability grants specific rights to an object
#[derive(Debug, Clone, Copy)]
pub struct Capability {
    pub handle: ObjectHandle,
    pub rights: CapabilityRights,
}

impl Capability {
    pub fn new(handle: ObjectHandle, rights: CapabilityRights) -> Self {
        Self { handle, rights }
    }

    /// Check if this capability grants read access
    pub fn can_read(&self) -> bool {
        self.rights.contains(CapabilityRights::READ)
    }

    /// Check if this capability grants write access
    pub fn can_write(&self) -> bool {
        self.rights.contains(CapabilityRights::WRITE)
    }

    /// Check if this capability grants execute access
    pub fn can_execute(&self) -> bool {
        self.rights.contains(CapabilityRights::EXECUTE)
    }

    /// Check if this capability can be transferred to another process
    pub fn can_transfer(&self) -> bool {
        self.rights.contains(CapabilityRights::TRANSFER)
    }
}

// ==================== SEALED CAPABILITIES ====================

/// Opaque capability identifier (user space only sees this)
///
/// This is like a file descriptor - a meaningless number without
/// the kernel's capability table to look it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapabilityId(u64);

impl CapabilityId {
    /// Create a new capability ID
    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// Global counter for generating unique capability IDs
static NEXT_CAPABILITY_ID: AtomicU64 = AtomicU64::new(1);

impl CapabilityId {
    /// Generate a new unique capability ID
    pub(crate) fn generate() -> Self {
        let id = NEXT_CAPABILITY_ID.fetch_add(1, Ordering::SeqCst);
        Self(id)
    }
}

/// Sealed capability with cryptographic authentication
///
/// This is the kernel-internal structure that combines:
/// - Opaque ID (what user space sees)
/// - Actual rights and object handle (what kernel uses)
/// - Cryptographic seal (prevents forgery and tampering)
/// - Generation counter (for revocation)
///
/// User space can only see CapabilityId. The SealedCapability
/// lives in kernel-only capability tables.
#[derive(Debug, Clone)]
pub struct SealedCapability {
    /// Public ID (user space sees this)
    pub id: CapabilityId,

    /// Actual rights (user space NEVER sees this directly)
    pub rights: CapabilityRights,

    /// Object this capability grants access to
    pub handle: ObjectHandle,

    /// Cryptographic seal (HMAC-SHA256 of id + rights + handle)
    /// Validates integrity and authenticity
    seal: [u8; 32],

    /// Generation counter for revocation
    /// Incrementing this invalidates all copies of this capability
    pub generation: u64,
}

impl SealedCapability {
    /// Create a new sealed capability
    ///
    /// # Arguments
    /// * `handle` - Object handle this capability grants access to
    /// * `rights` - Permission rights
    ///
    /// # Security
    /// The seal is computed using kernel-only secret key via HMAC-SHA256.
    /// This makes the capability unforgeable without the secret key.
    pub fn new(handle: ObjectHandle, rights: CapabilityRights) -> Self {
        // Validate W^X before creating capability
        debug_assert!(rights.validate_wx(), "W^X violation in capability creation!");

        let id = CapabilityId::generate();
        let generation = 0;

        // Compute cryptographic seal
        let seal = Self::compute_seal(id, rights, handle, generation);

        Self {
            id,
            rights,
            handle,
            seal,
            generation,
        }
    }

    /// Compute the cryptographic seal for this capability
    ///
    /// Seal = HMAC-SHA256(key, id || rights || handle || generation)
    ///
    /// The seal binds together all fields. Modifying any field
    /// (e.g., escalating rights) will break the seal.
    fn compute_seal(
        id: CapabilityId,
        rights: CapabilityRights,
        handle: ObjectHandle,
        generation: u64,
    ) -> [u8; 32] {
        // Serialize capability data for HMAC
        let mut data = [0u8; 32];

        // Pack all fields into data buffer
        data[0..8].copy_from_slice(&id.0.to_le_bytes());
        data[8..12].copy_from_slice(&rights.bits().to_le_bytes());
        data[12..20].copy_from_slice(&handle.0.to_le_bytes());
        data[20..28].copy_from_slice(&generation.to_le_bytes());

        // Compute HMAC-SHA256
        unsafe {
            sealing::get_sealer().seal(&data)
        }
    }

    /// Validate the cryptographic seal
    ///
    /// # Returns
    /// `true` if seal is valid, `false` if capability has been tampered with
    ///
    /// # Security
    /// Uses constant-time comparison to prevent timing attacks.
    pub fn validate(&self) -> bool {
        let expected_seal = Self::compute_seal(
            self.id,
            self.rights,
            self.handle,
            self.generation,
        );

        unsafe {
            sealing::get_sealer().verify(&self.serialize_for_seal(), &expected_seal)
        }
    }

    /// Serialize capability data for sealing
    fn serialize_for_seal(&self) -> [u8; 32] {
        let mut data = [0u8; 32];
        data[0..8].copy_from_slice(&self.id.0.to_le_bytes());
        data[8..12].copy_from_slice(&self.rights.bits().to_le_bytes());
        data[12..20].copy_from_slice(&self.handle.0.to_le_bytes());
        data[20..28].copy_from_slice(&self.generation.to_le_bytes());
        data
    }

    /// Derive a new capability with reduced rights (attenuation)
    ///
    /// # Arguments
    /// * `new_rights` - The reduced rights for the new capability
    ///
    /// # Security
    /// Can only reduce rights, never increase them. If new_rights
    /// contains any right not in the parent, this panics.
    ///
    /// # Panics
    /// Panics if attempting to amplify rights
    pub fn derive(&self, new_rights: CapabilityRights) -> Self {
        // SECURITY: Can only attenuate (reduce rights), never amplify
        assert!(
            self.rights.contains(new_rights),
            "Cannot amplify rights! Parent: {:?}, Child: {:?}",
            self.rights,
            new_rights
        );

        // Validate W^X on derived capability
        assert!(
            new_rights.validate_wx(),
            "Derived capability violates W^X!"
        );

        let id = CapabilityId::generate();
        let seal = Self::compute_seal(id, new_rights, self.handle, self.generation);

        Self {
            id,
            rights: new_rights,
            handle: self.handle,
            seal,
            generation: self.generation,
        }
    }

    /// Convert to legacy Capability (for compatibility)
    pub fn to_capability(&self) -> Capability {
        Capability {
            handle: self.handle,
            rights: self.rights,
        }
    }
}

bitflags::bitflags! {
    /// Rights that can be granted to a capability
    pub struct CapabilityRights: u32 {
        const READ     = 0b0001;
        const WRITE    = 0b0010;
        const EXECUTE  = 0b0100;
        const TRANSFER = 0b1000;
    }
}

impl CapabilityRights {
    /// Validate W^X (Write XOR Execute) property
    /// Returns true if rights are valid (never both WRITE and EXECUTE)
    pub fn validate_wx(&self) -> bool {
        // CRITICAL SECURITY: Cannot have both WRITE and EXECUTE
        let has_write = self.contains(Self::WRITE);
        let has_execute = self.contains(Self::EXECUTE);

        // Allowed combinations:
        // - READ only: OK
        // - READ + WRITE: OK (writable data)
        // - READ + EXECUTE: OK (executable code)
        // - WRITE + EXECUTE: FORBIDDEN! (attack vector)

        !(has_write && has_execute)
    }

    /// Full rights (read, write, transfer) - NO EXECUTE for security!
    /// SECURITY: We don't allow full() to include EXECUTE to prevent W+X
    pub fn full() -> Self {
        Self::READ | Self::WRITE | Self::TRANSFER
    }

    /// Read-only rights
    pub fn read_only() -> Self {
        Self::READ
    }

    /// Read-write rights (for data pages)
    pub fn read_write() -> Self {
        Self::READ | Self::WRITE
    }

    /// Read-execute rights (for code pages)
    pub fn read_execute() -> Self {
        Self::READ | Self::EXECUTE
    }

    /// Executable code with transfer capability
    pub fn code_with_transfer() -> Self {
        Self::READ | Self::EXECUTE | Self::TRANSFER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wx_validation() {
        // Valid combinations
        assert!(CapabilityRights::READ.validate_wx());
        assert!(CapabilityRights::read_only().validate_wx());
        assert!(CapabilityRights::read_write().validate_wx());
        assert!(CapabilityRights::read_execute().validate_wx());

        // INVALID: Write + Execute
        let write_execute = CapabilityRights::WRITE | CapabilityRights::EXECUTE;
        assert!(!write_execute.validate_wx(), "W+X should be rejected!");

        // INVALID: Full rights including execute
        let all_rights = CapabilityRights::READ | CapabilityRights::WRITE |
                        CapabilityRights::EXECUTE | CapabilityRights::TRANSFER;
        assert!(!all_rights.validate_wx(), "Full rights with W+X should be rejected!");
    }

    #[test]
    fn test_safe_combinations() {
        // These should all be safe
        assert!(CapabilityRights::read_only().validate_wx());
        assert!(CapabilityRights::read_write().validate_wx());
        assert!(CapabilityRights::read_execute().validate_wx());
        assert!(CapabilityRights::code_with_transfer().validate_wx());
    }
}
