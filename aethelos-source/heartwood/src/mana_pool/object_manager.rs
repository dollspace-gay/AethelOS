//! Object Manager - Manages memory as abstract objects

use super::capability::{Capability, CapabilityRights};
use super::{AllocationPurpose, ManaError};
use alloc::collections::BTreeMap;

/// A handle to an object in the Mana Pool
/// This is what user-space processes receive - never raw pointers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectHandle(pub u64);

/// The type of object
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Memory,
    File,
    Channel,
    Thread,
}

/// An object in the Mana Pool
struct Object {
    /// Object handle - kept for future reverse lookups and debugging
    #[allow(dead_code)]
    pub(super) handle: ObjectHandle,
    pub(super) object_type: ObjectType,
    pub(super) address: usize,
    pub(super) size: usize,
    pub(super) purpose: AllocationPurpose,
    pub(super) ref_count: usize,
}

/// Manages all objects in the Mana Pool
pub struct ObjectManager {
    objects: BTreeMap<ObjectHandle, Object>,
    next_handle: u64,
}

impl Default for ObjectManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectManager {
    pub fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
            next_handle: 1, // 0 is reserved as invalid
        }
    }

    /// Create a new memory object and return a capability with full rights
    pub fn create_object(
        &mut self,
        address: usize,
        size: usize,
        purpose: AllocationPurpose,
    ) -> Result<Capability, ManaError> {
        let handle = ObjectHandle(self.next_handle);
        self.next_handle += 1;

        let object = Object {
            handle,
            object_type: ObjectType::Memory,
            address,
            size,
            purpose,
            ref_count: 1,
        };

        self.objects.insert(handle, object);

        // Return a capability with full rights for the creator
        Ok(Capability::new(handle, CapabilityRights::full()))
    }

    /// Release an object (decrement ref count, free if zero)
    /// Requires a valid capability to the object
    pub fn release_object(&mut self, capability: &Capability) -> Result<(), ManaError> {
        // Validate the capability
        if !self.validate_capability(capability) {
            return Err(ManaError::InvalidCapability);
        }

        let object = self
            .objects
            .get_mut(&capability.handle)
            .ok_or(ManaError::InvalidHandle)?;

        if object.ref_count == 0 {
            return Err(ManaError::AlreadyReleased);
        }

        object.ref_count -= 1;

        if object.ref_count == 0 {
            // Actually free the memory and remove the object
            self.objects.remove(&capability.handle);
        }

        Ok(())
    }

    /// Get the number of objects currently managed
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Validate a capability - checks that the handle exists
    /// In a real implementation, this would also check unforgeable tokens
    pub fn validate_capability(&self, capability: &Capability) -> bool {
        self.objects.contains_key(&capability.handle)
    }

    /// Clone a capability (increment ref count)
    /// Requires TRANSFER rights to share the capability
    pub fn clone_capability(&mut self, capability: &Capability) -> Result<Capability, ManaError> {
        // Verify the capability is valid
        if !self.validate_capability(capability) {
            return Err(ManaError::InvalidCapability);
        }

        // Check if this capability can be transferred
        if !capability.can_transfer() {
            return Err(ManaError::CannotTransfer);
        }

        let object = self
            .objects
            .get_mut(&capability.handle)
            .ok_or(ManaError::InvalidHandle)?;

        object.ref_count += 1;

        // Return a new capability with the same rights
        Ok(Capability::new(capability.handle, capability.rights))
    }

    /// Derive a new capability with restricted rights
    /// This allows creating read-only capabilities from read-write ones, etc.
    pub fn derive_capability(
        &self,
        capability: &Capability,
        new_rights: CapabilityRights,
    ) -> Result<Capability, ManaError> {
        // Verify the original capability is valid
        if !self.validate_capability(capability) {
            return Err(ManaError::InvalidCapability);
        }

        // Can only derive rights that the original capability has
        if !capability.rights.contains(new_rights) {
            return Err(ManaError::InsufficientRights);
        }

        Ok(Capability::new(capability.handle, new_rights))
    }

    /// Access object data through a capability (for reading)
    /// Returns the object's address and size if the capability grants READ rights
    pub fn access_object(&self, capability: &Capability) -> Result<(usize, usize), ManaError> {
        if !self.validate_capability(capability) {
            return Err(ManaError::InvalidCapability);
        }

        if !capability.can_read() {
            return Err(ManaError::InsufficientRights);
        }

        let object = self
            .objects
            .get(&capability.handle)
            .ok_or(ManaError::InvalidHandle)?;

        Ok((object.address, object.size))
    }

    /// Get object metadata through a capability
    pub fn get_object_info(&self, capability: &Capability) -> Result<ObjectInfo, ManaError> {
        if !self.validate_capability(capability) {
            return Err(ManaError::InvalidCapability);
        }

        let object = self
            .objects
            .get(&capability.handle)
            .ok_or(ManaError::InvalidHandle)?;

        Ok(ObjectInfo {
            object_type: object.object_type,
            size: object.size,
            purpose: object.purpose,
            ref_count: object.ref_count,
        })
    }
}

/// Public information about an object (doesn't reveal address)
#[derive(Debug, Clone, Copy)]
pub struct ObjectInfo {
    pub object_type: ObjectType,
    pub size: usize,
    pub purpose: AllocationPurpose,
    pub ref_count: usize,
}
