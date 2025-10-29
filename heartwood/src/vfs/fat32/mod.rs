//! FAT32 Filesystem Driver
//!
//! Implements read-only FAT32 support for AethelOS.
//!
//! FAT32 is the most widely used filesystem for removable media (USB drives,
//! SD cards) and is supported by all major operating systems. This driver
//! allows AethelOS to read files from FAT32 volumes.
//!
//! # Features
//!
//! - Boot sector (BPB) parsing
//! - FAT table navigation and cluster chain following
//! - Directory entry parsing (including long filenames)
//! - File reading
//! - Read-only access (write support planned for future)
//!
//! # Example
//!
//! ```
//! use vfs::fat32::Fat32;
//! use vfs::FileSystem;
//!
//! let fs = Fat32::new(block_device)?;
//! let data = fs.read(&Path::new("/README.TXT"))?;
//! ```

pub mod bpb;
pub mod fat;
pub mod dir;

use super::{FileSystem, Path, FsError, DirEntry as VfsDirEntry, FileStat};
use super::block_device::BlockDevice;
use bpb::Fat32Bpb;
use fat::FatTable;
use dir::{DirEntry, DirEntryIter};
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::ToString;

/// FAT32 Filesystem
///
/// Provides read-only access to FAT32 volumes.
pub struct Fat32 {
    device: Box<dyn BlockDevice>,
    pub bpb: Fat32Bpb,
}

impl Fat32 {
    /// Create a new FAT32 filesystem from a block device
    ///
    /// # Arguments
    ///
    /// * `device` - Block device containing a FAT32 filesystem
    ///
    /// # Returns
    ///
    /// * `Ok(Fat32)` - Successfully mounted FAT32 filesystem
    /// * `Err(FsError)` - Invalid or corrupted filesystem
    pub fn new(device: Box<dyn BlockDevice>) -> Result<Self, FsError> {
        // Debug: entering Fat32::new
        unsafe {
        }

        let bpb = Fat32Bpb::from_device(&*device)
            .map_err(|_| FsError::IoError)?;

        // Debug: BPB parsed successfully
        unsafe {
        }

        Ok(Self { device, bpb })
    }

    /// Find a file or directory by path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to search for (e.g., "/DOCS/README.TXT")
    ///
    /// # Returns
    ///
    /// * `Ok(DirEntry)` - Found entry
    /// * `Err(FsError::NotFound)` - Path doesn't exist
    /// * `Err(FsError::IoError)` - Read error
    fn find_entry(&self, path: &Path) -> Result<DirEntry, FsError> {
        let path_str = path.as_str().trim_start_matches('/');

        // Empty path = root directory
        if path_str.is_empty() {
            return Ok(DirEntry {
                name: "/".to_string(),
                attributes: dir::attr::DIRECTORY,
                first_cluster: self.bpb.root_cluster,
                size: 0,
                is_dir: true,
                is_hidden: false,
            });
        }

        // Split path into components
        let components: Vec<&str> = path_str.split('/').collect();

        // Start at root directory
        let mut current_cluster = self.bpb.root_cluster;

        // Traverse path components
        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;

            // Read directory entries
            let fat = FatTable::new(&*self.device, &self.bpb);
            let dir_data = fat.read_chain(current_cluster, u64::MAX)
                .map_err(|_| FsError::IoError)?;

            let mut iter = DirEntryIter::new(dir_data);
            let mut found = false;

            while let Some(entry) = iter.next() {
                // Case-insensitive comparison
                if entry.name.eq_ignore_ascii_case(component) {
                    if is_last {
                        // Found the target!
                        return Ok(entry);
                    } else {
                        // Continue searching in this subdirectory
                        if !entry.is_dir {
                            return Err(FsError::NotADirectory);
                        }
                        current_cluster = entry.first_cluster;
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                return Err(FsError::NotFound);
            }
        }

        Err(FsError::NotFound)
    }
}

impl FileSystem for Fat32 {
    fn name(&self) -> &str {
        "FAT32"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let entry = self.find_entry(path)?;

        if entry.is_dir {
            return Err(FsError::IsADirectory);
        }

        // Read file data
        let fat = FatTable::new(&*self.device, &self.bpb);
        let data = fat.read_chain(entry.first_cluster, entry.size as u64)
            .map_err(|_| FsError::IoError)?;

        Ok(data)
    }

    fn write(&self, _path: &Path, _data: &[u8]) -> Result<(), FsError> {
        // Write support not yet implemented
        Err(FsError::ReadOnly)
    }

    fn remove(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly)
    }

    fn create_dir(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<VfsDirEntry>, FsError> {
        crate::println!("[DEBUG FAT32] read_dir() called for path: {}", path.as_str());

        let entry = self.find_entry(path)?;
        crate::println!("[DEBUG FAT32] find_entry() returned, is_dir: {}, first_cluster: 0x{:x}",
            entry.is_dir, entry.first_cluster);

        if !entry.is_dir {
            return Err(FsError::NotADirectory);
        }

        // Read directory data
        crate::println!("[DEBUG FAT32] Creating FatTable...");
        let fat = FatTable::new(&*self.device, &self.bpb);

        crate::println!("[DEBUG FAT32] Calling read_chain for cluster 0x{:x}...", entry.first_cluster);
        let dir_data = fat.read_chain(entry.first_cluster, u64::MAX)
            .map_err(|_| FsError::IoError)?;

        crate::println!("[DEBUG FAT32] read_chain completed, got {} bytes", dir_data.len());

        // Parse directory entries
        let iter = DirEntryIter::new(dir_data);
        let entries: Vec<VfsDirEntry> = iter.collect()
            .into_iter()
            .filter(|e| e.name != "." && e.name != "..") // Skip . and ..
            .map(|e| VfsDirEntry {
                name: e.name,
                is_dir: e.is_dir,
            })
            .collect();

        Ok(entries)
    }

    fn stat(&self, path: &Path) -> Result<FileStat, FsError> {
        let entry = self.find_entry(path)?;

        Ok(FileStat {
            size: entry.size as u64,
            is_dir: entry.is_dir,
            created: None,   // FAT32 has timestamps, but we're not parsing them yet
            modified: None,
        })
    }

    fn sync(&self) -> Result<(), FsError> {
        // Read-only, nothing to sync
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be added once we have a mock block device with FAT32 data
}
