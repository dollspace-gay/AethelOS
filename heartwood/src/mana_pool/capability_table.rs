//! Capability Table - Opaque handle mapping
//!
//! The capability table enforces the "opaque handle" security model:
//! - User space only sees `CapabilityId` numbers (meaningless without the table)
//! - Kernel maintains the mapping from ID → `SealedCapability`
//! - All capability operations require table lookup + seal validation
//!
//! This prevents forgery because:
//! 1. User can't create valid CapabilityId that exists in table
//! 2. User can't access table directly (kernel-only memory)
//! 3. Even if they guess an ID, seal validation catches tampering

use super::capability::{CapabilityId, SealedCapability, CapabilityRights};
use super::object_manager::ObjectHandle;
use alloc::collections::BTreeMap;

/// Errors that can occur during capability table operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    /// Capability ID not found in table
    InvalidId,
    /// Capability seal validation failed (tampering detected)
    SealBroken,
    /// Attempted operation requires rights not granted by capability
    PermissionDenied,
    /// Capability table is full
    TableFull,
}

/// Maximum number of capabilities per table
/// This prevents DoS attacks via capability exhaustion
const MAX_CAPABILITIES: usize = 4096;

/// Capability table - maps opaque IDs to sealed capabilities
///
/// # Security Model
/// - Table lives in kernel-only memory (user space never sees it)
/// - User space only receives `CapabilityId` values
/// - All operations require lookup by ID + seal validation
/// - Invalid IDs or broken seals are rejected
///
/// # Usage
/// ```
/// // Create and insert capability
/// let cap = SealedCapability::new(handle, rights);
/// let cap_id = table.insert(cap)?;
///
/// // Lookup and validate
/// let cap = table.get(cap_id)?;  // Validates seal automatically
/// if cap.rights.contains(READ) {
///     // Perform read operation
/// }
/// ```
pub struct CapabilityTable {
    /// Mapping from opaque ID → sealed capability
    /// Using BTreeMap instead of HashMap for deterministic iteration
    table: BTreeMap<CapabilityId, SealedCapability>,
}

impl CapabilityTable {
    /// Create a new empty capability table
    pub const fn new() -> Self {
        Self {
            table: BTreeMap::new(),
        }
    }

    /// Insert a sealed capability into the table
    ///
    /// # Arguments
    /// * `cap` - The sealed capability to insert
    ///
    /// # Returns
    /// * `Ok(CapabilityId)` - The opaque ID for this capability
    /// * `Err(TableFull)` - If table has reached maximum capacity
    ///
    /// # Security
    /// The returned CapabilityId is the ONLY handle user space gets.
    /// Without access to this table, the ID is meaningless.
    pub fn insert(&mut self, cap: SealedCapability) -> Result<CapabilityId, CapabilityError> {
        if self.table.len() >= MAX_CAPABILITIES {
            return Err(CapabilityError::TableFull);
        }

        let id = cap.id;
        self.table.insert(id, cap);
        Ok(id)
    }

    /// Lookup capability by ID and validate seal
    ///
    /// # Arguments
    /// * `id` - The opaque capability ID to lookup
    ///
    /// # Returns
    /// * `Ok(&SealedCapability)` - Valid capability with intact seal
    /// * `Err(InvalidId)` - ID not found in table
    /// * `Err(SealBroken)` - Seal validation failed (tampering detected)
    ///
    /// # Security
    /// This is the PRIMARY security boundary. Every capability use
    /// must go through this function, which:
    /// 1. Verifies the ID exists in the table (prevents forgery)
    /// 2. Validates the cryptographic seal (prevents tampering)
    pub fn get(&self, id: CapabilityId) -> Result<&SealedCapability, CapabilityError> {
        // 1. Lookup capability by ID
        let cap = self.table.get(&id)
            .ok_or(CapabilityError::InvalidId)?;

        // 2. SECURITY: Validate cryptographic seal
        if !cap.validate() {
            return Err(CapabilityError::SealBroken);
        }

        Ok(cap)
    }

    /// Lookup capability with mutable access (for revocation)
    ///
    /// # Security
    /// Mutable access is restricted to kernel operations like revocation.
    /// Still validates seal before returning.
    pub fn get_mut(&mut self, id: CapabilityId) -> Result<&mut SealedCapability, CapabilityError> {
        let cap = self.table.get_mut(&id)
            .ok_or(CapabilityError::InvalidId)?;

        if !cap.validate() {
            return Err(CapabilityError::SealBroken);
        }

        Ok(cap)
    }

    /// Check if capability grants specific rights
    ///
    /// # Security
    /// Validates seal before checking rights. This is a convenience
    /// function that combines lookup + rights check.
    pub fn check_rights(&self, id: CapabilityId, required: CapabilityRights) -> Result<(), CapabilityError> {
        let cap = self.get(id)?;

        if cap.rights.contains(required) {
            Ok(())
        } else {
            Err(CapabilityError::PermissionDenied)
        }
    }

    /// Remove capability from table (revocation)
    ///
    /// # Returns
    /// The removed capability, if it existed
    pub fn remove(&mut self, id: CapabilityId) -> Option<SealedCapability> {
        self.table.remove(&id)
    }

    /// Derive a new capability with reduced rights
    ///
    /// Creates a new sealed capability with attenuated rights and
    /// inserts it into the table, returning the new ID.
    ///
    /// # Arguments
    /// * `parent_id` - ID of parent capability
    /// * `new_rights` - Reduced rights for child capability
    ///
    /// # Security
    /// Can only reduce rights, never amplify. The parent capability's
    /// seal is validated before derivation.
    ///
    /// # Panics
    /// Panics if attempting to amplify rights or violate W^X
    pub fn derive(&mut self, parent_id: CapabilityId, new_rights: CapabilityRights) -> Result<CapabilityId, CapabilityError> {
        // Lookup and validate parent
        let parent = self.get(parent_id)?;

        // Derive child capability (validates rights attenuation)
        let child = parent.derive(new_rights);

        // Insert child into table
        self.insert(child)
    }

    /// Get the object handle for a capability (after validation)
    ///
    /// # Security
    /// Validates seal and rights before returning the handle.
    pub fn get_handle(&self, id: CapabilityId, required_rights: CapabilityRights) -> Result<ObjectHandle, CapabilityError> {
        let cap = self.get(id)?;

        if !cap.rights.contains(required_rights) {
            return Err(CapabilityError::PermissionDenied);
        }

        Ok(cap.handle)
    }

    /// Get number of capabilities in table
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Clear all capabilities from table
    ///
    /// # Security
    /// Use with caution - this revokes ALL capabilities in the table.
    pub fn clear(&mut self) {
        self.table.clear();
    }
}

impl Default for CapabilityTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_lookup() {
        let mut table = CapabilityTable::new();
        let handle = ObjectHandle(0x1000);
        let rights = CapabilityRights::READ;

        let cap = SealedCapability::new(handle, rights);
        let cap_id = table.insert(cap).unwrap();

        // Should be able to retrieve
        let retrieved = table.get(cap_id).unwrap();
        assert_eq!(retrieved.handle, handle);
        assert_eq!(retrieved.rights, rights);
    }

    #[test]
    fn test_invalid_id() {
        let table = CapabilityTable::new();
        let invalid_id = CapabilityId::new(9999);

        // Should fail - ID not in table
        assert_eq!(table.get(invalid_id), Err(CapabilityError::InvalidId));
    }

    #[test]
    fn test_rights_check() {
        let mut table = CapabilityTable::new();
        let handle = ObjectHandle(0x1000);
        let rights = CapabilityRights::READ;

        let cap = SealedCapability::new(handle, rights);
        let cap_id = table.insert(cap).unwrap();

        // Should succeed - has READ
        assert!(table.check_rights(cap_id, CapabilityRights::READ).is_ok());

        // Should fail - doesn't have WRITE
        assert_eq!(
            table.check_rights(cap_id, CapabilityRights::WRITE),
            Err(CapabilityError::PermissionDenied)
        );
    }

    #[test]
    fn test_derive_attenuation() {
        let mut table = CapabilityTable::new();
        let handle = ObjectHandle(0x1000);
        let parent_rights = CapabilityRights::READ | CapabilityRights::WRITE;

        let parent = SealedCapability::new(handle, parent_rights);
        let parent_id = table.insert(parent).unwrap();

        // Derive read-only capability
        let child_id = table.derive(parent_id, CapabilityRights::READ).unwrap();

        // Child should have reduced rights
        let child = table.get(child_id).unwrap();
        assert!(child.rights.contains(CapabilityRights::READ));
        assert!(!child.rights.contains(CapabilityRights::WRITE));
    }

    #[test]
    #[should_panic(expected = "Cannot amplify rights")]
    fn test_derive_amplification_panics() {
        let mut table = CapabilityTable::new();
        let handle = ObjectHandle(0x1000);
        let parent_rights = CapabilityRights::READ;

        let parent = SealedCapability::new(handle, parent_rights);
        let parent_id = table.insert(parent).unwrap();

        // Try to derive with MORE rights (should panic)
        let _ = table.derive(parent_id, CapabilityRights::READ | CapabilityRights::WRITE);
    }

    #[test]
    fn test_revocation() {
        let mut table = CapabilityTable::new();
        let handle = ObjectHandle(0x1000);
        let cap = SealedCapability::new(handle, CapabilityRights::READ);
        let cap_id = table.insert(cap).unwrap();

        // Should exist
        assert!(table.get(cap_id).is_ok());

        // Revoke
        let removed = table.remove(cap_id);
        assert!(removed.is_some());

        // Should no longer exist
        assert_eq!(table.get(cap_id), Err(CapabilityError::InvalidId));
    }

    #[test]
    fn test_table_full() {
        let mut table = CapabilityTable::new();

        // Fill table to capacity
        for i in 0..MAX_CAPABILITIES {
            let handle = ObjectHandle(i as u64);
            let cap = SealedCapability::new(handle, CapabilityRights::READ);
            assert!(table.insert(cap).is_ok());
        }

        // Next insert should fail
        let cap = SealedCapability::new(ObjectHandle(0xFFFF), CapabilityRights::READ);
        assert_eq!(table.insert(cap), Err(CapabilityError::TableFull));
    }
}
