//! FAT Table Navigation
//!
//! The File Allocation Table (FAT) tracks which clusters belong to which files.
//! Each FAT entry points to the next cluster in a file's cluster chain.
//!
//! Special FAT values (28-bit, top 4 bits ignored):
//! - 0x0000000: Free cluster
//! - 0x0000001: Reserved
//! - 0x0000002-0xFFFFFEF: Regular cluster number (next in chain)
//! - 0xFFFFFF0-0xFFFFFF6: Reserved
//! - 0xFFFFFF7: Bad cluster
//! - 0xFFFFFF8-0xFFFFFFF: End of chain (EOC)

use super::bpb::Fat32Bpb;
use super::super::block_device::{BlockDevice, BlockDeviceError};
use alloc::vec::Vec;

/// FAT entry value constants
pub const FAT_FREE: u32 = 0x00000000;
pub const FAT_BAD: u32 = 0x0FFFFFF7;
pub const FAT_EOC: u32 = 0x0FFFFFF8; // End of chain (any value >= this)

/// FAT Table reader
///
/// Provides methods to read FAT entries and follow cluster chains.
pub struct FatTable<'a> {
    device: &'a dyn BlockDevice,
    bpb: &'a Fat32Bpb,
}

impl<'a> FatTable<'a> {
    /// Create a new FAT table reader
    pub fn new(device: &'a dyn BlockDevice, bpb: &'a Fat32Bpb) -> Self {
        Self { device, bpb }
    }

    /// Read a single FAT entry
    ///
    /// # Arguments
    ///
    /// * `cluster` - Cluster number to look up
    ///
    /// # Returns
    ///
    /// * `Ok(u32)` - FAT entry value (next cluster or EOC)
    /// * `Err(BlockDeviceError)` - Read failed
    pub fn read_entry(&self, cluster: u32) -> Result<u32, BlockDeviceError> {
        // Each FAT entry is 4 bytes (32 bits, but only 28 bits used)
        let fat_offset = self.bpb.fat_offset() + (cluster as u64 * 4);
        let sector = fat_offset / self.bpb.bytes_per_sector as u64;
        let offset_in_sector = (fat_offset % self.bpb.bytes_per_sector as u64) as usize;

        // Read the sector containing this FAT entry
        let sector_data = self.device.read_sector(sector)?;

        // Extract the 32-bit entry
        let entry = u32::from_le_bytes([
            sector_data[offset_in_sector],
            sector_data[offset_in_sector + 1],
            sector_data[offset_in_sector + 2],
            sector_data[offset_in_sector + 3],
        ]);

        // Mask to 28 bits (ignore top 4 bits)
        Ok(entry & 0x0FFFFFFF)
    }

    /// Check if a FAT entry indicates end of chain
    pub fn is_eoc(entry: u32) -> bool {
        (entry & 0x0FFFFFFF) >= FAT_EOC
    }

    /// Check if a FAT entry indicates a bad cluster
    pub fn is_bad(entry: u32) -> bool {
        (entry & 0x0FFFFFFF) == FAT_BAD
    }

    /// Check if a FAT entry indicates a free cluster
    pub fn is_free(entry: u32) -> bool {
        (entry & 0x0FFFFFFF) == FAT_FREE
    }

    /// Follow a cluster chain and return all clusters in the chain
    ///
    /// # Arguments
    ///
    /// * `start_cluster` - First cluster in the chain
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u32>)` - All cluster numbers in the chain (in order)
    /// * `Err(BlockDeviceError)` - Read failed
    ///
    /// # Panics
    ///
    /// Panics if the chain is too long (>100,000 clusters = ~400MB file with 4KB clusters).
    /// This prevents infinite loops from corrupted FAT tables.
    pub fn follow_chain(&self, start_cluster: u32) -> Result<Vec<u32>, BlockDeviceError> {
        let mut chain = Vec::new();
        let mut current = start_cluster;
        const MAX_CHAIN_LENGTH: usize = 100_000;

        loop {
            if chain.len() >= MAX_CHAIN_LENGTH {
                // Corrupted FAT table with circular chain?
                panic!("FAT chain too long (possible corruption)");
            }

            // Debug: Print every 10th cluster to avoid spam
            if chain.len() % 10 == 0 {
                crate::println!("[DEBUG FAT] Chain length: {}, current cluster: 0x{:x}", chain.len(), current);
            }

            chain.push(current);

            let entry = self.read_entry(current)?;

            // Debug: Show first few entries to diagnose
            if chain.len() <= 5 {
                crate::println!("[DEBUG FAT] Cluster 0x{:x} -> next: 0x{:x} (EOC: {})",
                    current, entry, Self::is_eoc(entry));
            }

            if Self::is_eoc(entry) {
                // End of chain reached
                crate::println!("[DEBUG FAT] Chain complete, {} clusters", chain.len());
                break;
            }

            if Self::is_bad(entry) {
                // Bad cluster in chain - data is corrupt
                return Err(BlockDeviceError::IoError);
            }

            if Self::is_free(entry) {
                // Free cluster in chain - corrupted FAT
                return Err(BlockDeviceError::IoError);
            }

            current = entry;
        }

        Ok(chain)
    }

    /// Read data from a cluster chain
    ///
    /// # Arguments
    ///
    /// * `start_cluster` - First cluster in the chain
    /// * `max_size` - Maximum bytes to read (for files, this is the file size)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - File data (up to max_size bytes)
    /// * `Err(BlockDeviceError)` - Read failed
    pub fn read_chain(&self, start_cluster: u32, max_size: u64) -> Result<Vec<u8>, BlockDeviceError> {
        crate::println!("[DEBUG FAT] read_chain() called, start_cluster: 0x{:x}", start_cluster);

        let chain = self.follow_chain(start_cluster)?;
        crate::println!("[DEBUG FAT] follow_chain() returned {} clusters", chain.len());

        let cluster_size = self.bpb.cluster_size() as usize;
        let mut data = Vec::new();
        let mut remaining = max_size as usize;

        for cluster in chain {
            if remaining == 0 {
                break;
            }

            // Read cluster data
            let sector = self.bpb.cluster_to_sector(cluster);
            let cluster_data = self.device.read_sectors(sector, self.bpb.sectors_per_cluster as u32)?;

            // Append up to remaining bytes
            let to_copy = remaining.min(cluster_size);
            data.extend_from_slice(&cluster_data[..to_copy]);
            remaining -= to_copy;
        }

        Ok(data)
    }

    /// Find the next free cluster in the FAT
    ///
    /// This is used when writing files to allocate new clusters.
    ///
    /// # Arguments
    ///
    /// * `start_hint` - Cluster to start searching from (optimization)
    ///
    /// # Returns
    ///
    /// * `Ok(u32)` - Free cluster number
    /// * `Err(IoError)` - No free clusters (disk full)
    pub fn find_free_cluster(&self, start_hint: u32) -> Result<u32, BlockDeviceError> {
        // Calculate total clusters
        let data_sectors = self.bpb.total_sectors as u64
            - self.bpb.reserved_sectors as u64
            - (self.bpb.num_fats as u64 * self.bpb.sectors_per_fat as u64);
        let total_clusters = (data_sectors / self.bpb.sectors_per_cluster as u64) as u32;

        // Search from hint to end
        for cluster in start_hint..total_clusters + 2 {
            let entry = self.read_entry(cluster)?;
            if Self::is_free(entry) {
                return Ok(cluster);
            }
        }

        // Wrap around and search from cluster 2 to hint
        for cluster in 2..start_hint {
            let entry = self.read_entry(cluster)?;
            if Self::is_free(entry) {
                return Ok(cluster);
            }
        }

        // No free clusters
        Err(BlockDeviceError::IoError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eoc_detection() {
        assert!(FatTable::is_eoc(0x0FFFFFF8));
        assert!(FatTable::is_eoc(0x0FFFFFFF));
        assert!(!FatTable::is_eoc(0x0FFFFFF7));
        assert!(!FatTable::is_eoc(0x00000002));
    }

    #[test]
    fn test_bad_cluster_detection() {
        assert!(FatTable::is_bad(0x0FFFFFF7));
        assert!(!FatTable::is_bad(0x0FFFFFF8));
        assert!(!FatTable::is_bad(0x00000000));
    }

    #[test]
    fn test_free_cluster_detection() {
        assert!(FatTable::is_free(0x00000000));
        assert!(!FatTable::is_free(0x00000002));
        assert!(!FatTable::is_free(0x0FFFFFF8));
    }
}
