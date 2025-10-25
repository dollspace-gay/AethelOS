//! Sanctuary - Long-lived, stable memory allocations

use super::ManaError;

/// Size of the Sanctuary region (for now, a placeholder)
const SANCTUARY_SIZE: usize = 16 * 1024 * 1024; // 16 MB

/// The Sanctuary manages long-lived memory allocations
pub struct Sanctuary {
    base_address: usize,
    total_size: usize,
    used_size: usize,
}

impl Default for Sanctuary {
    fn default() -> Self {
        Self::new()
    }
}

impl Sanctuary {
    pub fn new() -> Self {
        // In a real implementation, this would reserve actual memory
        Self {
            base_address: 0x1000_0000, // Placeholder address
            total_size: SANCTUARY_SIZE,
            used_size: 0,
        }
    }

    /// Allocate memory in the Sanctuary
    pub fn allocate(&mut self, size: usize) -> Result<usize, ManaError> {
        if size > self.total_size - self.used_size {
            return Err(ManaError::OutOfMemory);
        }

        let address = self.base_address + self.used_size;
        self.used_size += size;

        Ok(address)
    }

    /// Get the number of bytes used
    pub fn used_bytes(&self) -> usize {
        self.used_size
    }

    /// Get the total size
    pub fn total_bytes(&self) -> usize {
        self.total_size
    }
}
