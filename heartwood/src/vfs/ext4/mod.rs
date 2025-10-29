//! ext4 Filesystem Driver
//!
//! Implements read-only ext4 support for AethelOS.
//!
//! ext4 is the most widely used filesystem on Linux systems and supports
//! advanced features like extents, large files, and journaling. This driver
//! allows AethelOS to read files from ext4 volumes.
//!
//! # Features
//!
//! - Superblock and group descriptor parsing
//! - Extent tree navigation for efficient large file support
//! - Inode reading (both 128-byte and 256-byte formats)
//! - Directory entry parsing (linear and htree formats)
//! - File reading
//! - Read-only access (write support planned for future)
//!
//! # Example
//!
//! ```
//! use vfs::ext4::Ext4;
//! use vfs::FileSystem;
//!
//! let fs = Ext4::new(block_device)?;
//! let data = fs.read(&Path::new("/home/user/document.txt"))?;
//! ```

pub mod superblock;
pub mod inode;
pub mod extent;
pub mod dir;

use super::{FileSystem, Path, FsError, DirEntry as VfsDirEntry, FileStat};
use super::block_device::BlockDevice;
use superblock::Ext4Superblock;
use inode::Inode;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::ToString;

/// ext4 Filesystem
///
/// Provides read-only access to ext4 volumes.
pub struct Ext4 {
    device: Box<dyn BlockDevice>,
    pub superblock: Ext4Superblock,
}

impl Ext4 {
    /// Create a new ext4 filesystem from a block device
    ///
    /// # Arguments
    ///
    /// * `device` - Block device containing an ext4 filesystem
    ///
    /// # Returns
    ///
    /// * `Ok(Ext4)` - Successfully mounted ext4 filesystem
    /// * `Err(FsError)` - Invalid or corrupted filesystem
    pub fn new(device: Box<dyn BlockDevice>) -> Result<Self, FsError> {
        let superblock = Ext4Superblock::from_device(&*device)
            .map_err(|_| FsError::IoError)?;

        // Verify ext4 magic number
        if !superblock.is_valid() {
            return Err(FsError::IoError);
        }

        Ok(Self { device, superblock })
    }

    /// Read an inode by number
    ///
    /// # Arguments
    ///
    /// * `inode_num` - Inode number (1-indexed, as per ext4 spec)
    ///
    /// # Returns
    ///
    /// * `Ok(Inode)` - The requested inode
    /// * `Err(FsError)` - Invalid inode number or read error
    fn read_inode(&self, inode_num: u32) -> Result<Inode, FsError> {
        Inode::read_from_device(&*self.device, &self.superblock, inode_num)
            .map_err(|_| FsError::IoError)
    }

    /// Find a file or directory by path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to search for (e.g., "/home/user/document.txt")
    ///
    /// # Returns
    ///
    /// * `Ok((inode_num, Inode))` - Found inode and its number
    /// * `Err(FsError::NotFound)` - Path doesn't exist
    /// * `Err(FsError::IoError)` - Read error
    fn find_inode(&self, path: &Path) -> Result<(u32, Inode), FsError> {
        let path_str = path.as_str().trim_start_matches('/');

        // Empty path = root directory (inode 2)
        if path_str.is_empty() {
            let root_inode = self.read_inode(2)?;
            return Ok((2, root_inode));
        }

        // Split path into components
        let components: Vec<&str> = path_str.split('/').collect();

        // Start at root directory (inode 2)
        let mut current_inode_num = 2;
        let mut current_inode = self.read_inode(2)?;

        // Traverse path components
        for component in components {
            // Current inode must be a directory
            if !current_inode.is_dir() {
                return Err(FsError::NotADirectory);
            }

            // Search for component in current directory
            match dir::find_entry_in_dir(&*self.device, &self.superblock, &current_inode, component)? {
                Some((inode_num, entry_type)) => {
                    current_inode_num = inode_num;
                    current_inode = self.read_inode(inode_num)?;
                }
                None => return Err(FsError::NotFound),
            }
        }

        Ok((current_inode_num, current_inode))
    }
}

impl FileSystem for Ext4 {
    fn name(&self) -> &str {
        "ext4"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let (_inode_num, inode) = self.find_inode(path)?;

        if inode.is_dir() {
            return Err(FsError::IsADirectory);
        }

        // Read file data using extent tree
        extent::read_file_data(&*self.device, &self.superblock, &inode)
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
        let (_inode_num, inode) = self.find_inode(path)?;

        if !inode.is_dir() {
            return Err(FsError::NotADirectory);
        }

        // Read directory entries
        dir::read_dir_entries(&*self.device, &self.superblock, &inode)
    }

    fn stat(&self, path: &Path) -> Result<FileStat, FsError> {
        let (_inode_num, inode) = self.find_inode(path)?;

        Ok(FileStat {
            size: inode.size(),
            is_dir: inode.is_dir(),
            created: None,   // ext4 has timestamps, but we're not parsing them yet
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

    // Tests will be added once we have a mock block device with ext4 data
}
