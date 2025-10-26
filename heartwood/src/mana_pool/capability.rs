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
