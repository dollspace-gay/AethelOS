//! Capability-based security for memory access

use super::object_manager::ObjectHandle;

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
    /// Full rights (read, write, execute, transfer)
    pub fn full() -> Self {
        Self::READ | Self::WRITE | Self::EXECUTE | Self::TRANSFER
    }

    /// Read-only rights
    pub fn read_only() -> Self {
        Self::READ
    }

    /// Read-write rights
    pub fn read_write() -> Self {
        Self::READ | Self::WRITE
    }
}
