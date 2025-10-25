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
pub mod interrupt_lock;
pub mod buddy;

pub use object_manager::{ObjectManager, ObjectHandle, ObjectType, ObjectInfo};
pub use capability::{Capability, CapabilityRights};
pub use sanctuary::Sanctuary;
pub use ephemeral_mist::EphemeralMist;
pub use interrupt_lock::InterruptSafeLock;

use core::mem::MaybeUninit;
use alloc::boxed::Box;

// MANA_POOL stores a Box<ManaPool> - a small pointer to heap-allocated ManaPool
// Using InterruptSafeLock to prevent deadlocks during preemptive multitasking
static mut MANA_POOL: MaybeUninit<InterruptSafeLock<Box<ManaPool>>> = MaybeUninit::uninit();
static mut MANA_POOL_INITIALIZED: bool = false;

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

    /// Create a new ManaPool directly in a Box on the heap
    /// This avoids stack overflow by never creating the ManaPool on the stack
    pub fn new_boxed() -> alloc::boxed::Box<Self> {
        unsafe { serial_out(b'a'); } // Starting boxed creation

        // Allocate uninitialized box
        let mut boxed: alloc::boxed::Box<core::mem::MaybeUninit<Self>> = alloc::boxed::Box::new_uninit();
        unsafe { serial_out(b'b'); } // Box allocated

        // Initialize fields directly in the box
        unsafe {
            let ptr: *mut ManaPool = boxed.as_mut_ptr();
            core::ptr::write(&mut (*ptr).object_manager, ObjectManager::new());
            serial_out(b'c');
            core::ptr::write(&mut (*ptr).sanctuary, Sanctuary::new());
            serial_out(b'd');
            core::ptr::write(&mut (*ptr).ephemeral_mist, EphemeralMist::new());
            serial_out(b'e');

            boxed.assume_init()
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

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Initialize the Mana Pool
///
/// Note: The global allocator must be initialized BEFORE this function!
pub fn init() {
    unsafe {
        serial_out(b'M'); // Mana pool init started

        // Use new_boxed() to create ManaPool directly on heap (in-place)
        serial_out(b'N'); // About to call ManaPool::new_boxed
        let mana_pool_on_heap = ManaPool::new_boxed();
        serial_out(b'O'); // ManaPool::new_boxed returned

        // Create interrupt-safe lock and write to static
        serial_out(b'P'); // Before InterruptSafeLock::new
        let lock = InterruptSafeLock::new(mana_pool_on_heap);
        serial_out(b'Q'); // After InterruptSafeLock::new

        core::ptr::write(MANA_POOL.as_mut_ptr(), lock);
        serial_out(b'R'); // Written to static

        MANA_POOL_INITIALIZED = true;
        serial_out(b'S'); // Marked as initialized
    }
}

/// Get reference to MANA_POOL (assumes initialized)
unsafe fn get_mana_pool() -> &'static InterruptSafeLock<Box<ManaPool>> {
    MANA_POOL.assume_init_ref()
}

/// Allocate memory with a specific purpose
/// Returns a capability with full rights to the newly created object
pub fn animate(size: usize, purpose: AllocationPurpose) -> Result<Capability, ManaError> {
    unsafe { get_mana_pool().lock().animate(size, purpose) }
}

/// Release memory back to the pool
/// Requires a valid capability
pub fn release(capability: &Capability) -> Result<(), ManaError> {
    unsafe { get_mana_pool().lock().release(capability) }
}

/// Validate a capability
pub fn validate_capability(capability: &Capability) -> bool {
    unsafe { get_mana_pool().lock().validate_capability(capability) }
}

/// Clone a capability (requires TRANSFER rights)
pub fn clone_capability(capability: &Capability) -> Result<Capability, ManaError> {
    unsafe { get_mana_pool().lock().clone_capability(capability) }
}

/// Derive a new capability with restricted rights
pub fn derive_capability(
    capability: &Capability,
    new_rights: CapabilityRights,
) -> Result<Capability, ManaError> {
    unsafe { get_mana_pool().lock().derive_capability(capability, new_rights) }
}

/// Access object data through a capability
pub fn access_object(capability: &Capability) -> Result<(usize, usize), ManaError> {
    unsafe { get_mana_pool().lock().access_object(capability) }
}

/// Get object information through a capability
pub fn get_object_info(capability: &Capability) -> Result<ObjectInfo, ManaError> {
    unsafe { get_mana_pool().lock().get_object_info(capability) }
}

/// Get Mana Pool statistics
pub fn stats() -> ManaPoolStats {
    unsafe { get_mana_pool().lock().stats() }
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
