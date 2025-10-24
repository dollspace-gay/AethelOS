//! # The Mana Pool
//!
//! The memory management system of AethelOS.
//! Memory is not raw bytes - it is the lifeblood of the system,
//! carefully allocated, protected, and reclaimed with purpose.
//!
//! ## Philosophy
//! The Mana Pool does not hand out memory; it grants access to living objects.
//! Every allocation has a purpose, every object has an owner,
//! and every access is mediated by capabilities.
//!
//! ## Architecture
//! - Object-oriented memory (not page-based)
//! - Capability-based security (handles, not pointers)
//! - Purpose-driven allocation (Sanctuary vs Ephemeral Mist)
//! - Automatic reclamation (when the last capability is released)

pub mod object_manager;
pub mod capability;
pub mod sanctuary;
pub mod ephemeral_mist;
pub mod allocator;

pub use object_manager::{ObjectManager, ObjectHandle, ObjectType, ObjectInfo};
pub use capability::{Capability, CapabilityRights};
pub use sanctuary::Sanctuary;
pub use ephemeral_mist::EphemeralMist;

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref MANA_POOL: Mutex<ManaPool> = Mutex::new(ManaPool::new());
}

pub struct ManaPool {
    object_manager: ObjectManager,
    sanctuary: Sanctuary,
    ephemeral_mist: EphemeralMist,
}

impl Default for ManaPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ManaPool {
    pub fn new() -> Self {
        Self {
            object_manager: ObjectManager::new(),
            sanctuary: Sanctuary::new(),
            ephemeral_mist: EphemeralMist::new(),
        }
    }

    /// Animate (allocate) memory with a specific purpose
    /// Returns a capability with full rights to the newly created object
    pub fn animate(
        &mut self,
        size: usize,
        purpose: AllocationPurpose,
    ) -> Result<Capability, ManaError> {
        let address = match purpose {
            AllocationPurpose::LongLived | AllocationPurpose::Static => {
                self.sanctuary.allocate(size)?
            }
            AllocationPurpose::ShortLived | AllocationPurpose::Ephemeral => {
                self.ephemeral_mist.allocate(size)?
            }
        };

        self.object_manager.create_object(address, size, purpose)
    }

    /// Release an object back to the Mana Pool
    /// Requires a valid capability to the object
    pub fn release(&mut self, capability: &Capability) -> Result<(), ManaError> {
        self.object_manager.release_object(capability)
    }

    /// Validate a capability
    pub fn validate_capability(&self, capability: &Capability) -> bool {
        self.object_manager.validate_capability(capability)
    }

    /// Clone a capability (requires TRANSFER rights)
    pub fn clone_capability(&mut self, capability: &Capability) -> Result<Capability, ManaError> {
        self.object_manager.clone_capability(capability)
    }

    /// Derive a new capability with restricted rights
    pub fn derive_capability(
        &self,
        capability: &Capability,
        new_rights: CapabilityRights,
    ) -> Result<Capability, ManaError> {
        self.object_manager.derive_capability(capability, new_rights)
    }

    /// Access object data through a capability
    pub fn access_object(&self, capability: &Capability) -> Result<(usize, usize), ManaError> {
        self.object_manager.access_object(capability)
    }

    /// Get object information through a capability
    pub fn get_object_info(&self, capability: &Capability) -> Result<ObjectInfo, ManaError> {
        self.object_manager.get_object_info(capability)
    }

    /// Get statistics about the Mana Pool
    pub fn stats(&self) -> ManaPoolStats {
        ManaPoolStats {
            sanctuary_used: self.sanctuary.used_bytes(),
            sanctuary_total: self.sanctuary.total_bytes(),
            ephemeral_used: self.ephemeral_mist.used_bytes(),
            ephemeral_total: self.ephemeral_mist.total_bytes(),
            total_objects: self.object_manager.object_count(),
        }
    }
}

/// Initialize the Mana Pool
pub fn init() {
    // Initialization happens on first access via lazy_static
    let _ = MANA_POOL.lock();
}

/// Allocate memory with a specific purpose
/// Returns a capability with full rights to the newly created object
pub fn animate(size: usize, purpose: AllocationPurpose) -> Result<Capability, ManaError> {
    MANA_POOL.lock().animate(size, purpose)
}

/// Release memory back to the pool
/// Requires a valid capability
pub fn release(capability: &Capability) -> Result<(), ManaError> {
    MANA_POOL.lock().release(capability)
}

/// Validate a capability
pub fn validate_capability(capability: &Capability) -> bool {
    MANA_POOL.lock().validate_capability(capability)
}

/// Clone a capability (requires TRANSFER rights)
pub fn clone_capability(capability: &Capability) -> Result<Capability, ManaError> {
    MANA_POOL.lock().clone_capability(capability)
}

/// Derive a new capability with restricted rights
pub fn derive_capability(
    capability: &Capability,
    new_rights: CapabilityRights,
) -> Result<Capability, ManaError> {
    MANA_POOL.lock().derive_capability(capability, new_rights)
}

/// Access object data through a capability
pub fn access_object(capability: &Capability) -> Result<(usize, usize), ManaError> {
    MANA_POOL.lock().access_object(capability)
}

/// Get object information through a capability
pub fn get_object_info(capability: &Capability) -> Result<ObjectInfo, ManaError> {
    MANA_POOL.lock().get_object_info(capability)
}

/// Get Mana Pool statistics
pub fn stats() -> ManaPoolStats {
    MANA_POOL.lock().stats()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationPurpose {
    /// Long-lived, stable objects (allocated in Sanctuary)
    LongLived,

    /// Static data that persists for the lifetime of the system
    Static,

    /// Short-lived, temporary objects (allocated in Ephemeral Mist)
    ShortLived,

    /// Very temporary data that can be reclaimed aggressively
    Ephemeral,
}

#[derive(Debug, Clone, Copy)]
pub struct ManaPoolStats {
    pub sanctuary_used: usize,
    pub sanctuary_total: usize,
    pub ephemeral_used: usize,
    pub ephemeral_total: usize,
    pub total_objects: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaError {
    OutOfMemory,
    InvalidHandle,
    InvalidCapability,
    AlreadyReleased,
    AllocationTooLarge,
    /// Attempting to transfer a non-transferable capability
    CannotTransfer,
    /// Attempting to perform an operation without sufficient rights
    InsufficientRights,
}
