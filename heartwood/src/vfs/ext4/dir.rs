//! ext4 Directory Entry Parsing
//!
//! Directories in ext4 are stored as a sequence of variable-length entries.
//! Each entry contains the inode number, entry length, name length, file type,
//! and the file name.

use crate::vfs::block_device::BlockDevice;
use crate::vfs::{DirEntry as VfsDirEntry, FsError};
use super::superblock::Ext4Superblock;
use super::inode::Inode;
use super::extent;
use alloc::vec::Vec;
use alloc::string::{String, ToString};

/// File type constants (from ext4 directory entry)
const EXT4_FT_UNKNOWN: u8 = 0;
const EXT4_FT_REG_FILE: u8 = 1;
const EXT4_FT_DIR: u8 = 2;
const EXT4_FT_CHRDEV: u8 = 3;
const EXT4_FT_BLKDEV: u8 = 4;
const EXT4_FT_FIFO: u8 = 5;
const EXT4_FT_SOCK: u8 = 6;
const EXT4_FT_SYMLINK: u8 = 7;

/// ext4 Directory Entry (variable length)
///
/// Format:
/// - inode (4 bytes)
/// - rec_len (2 bytes) - total length of this entry
/// - name_len (1 byte) - length of file name
/// - file_type (1 byte) - file type
/// - name (variable) - file name (not null-terminated)
#[derive(Debug, Clone)]
struct DirEntry {
    /// Inode number (0 = unused entry)
    inode: u32,
    /// Length of this directory entry in bytes
    rec_len: u16,
    /// Length of the file name
    name_len: u8,
    /// File type
    file_type: u8,
    /// File name (not null-terminated)
    name: String,
}

impl DirEntry {
    /// Parse a directory entry from bytes
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing the directory entry
    ///
    /// # Returns
    ///
    /// * `Ok(DirEntry)` - Successfully parsed entry
    /// * `Err(())` - Parse error
    fn from_bytes(data: &[u8]) -> Result<Self, ()> {
        if data.len() < 8 {
            return Err(());
        }

        let inode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let rec_len = u16::from_le_bytes([data[4], data[5]]);
        let name_len = data[6];
        let file_type = data[7];

        // Validate rec_len
        if rec_len < 8 || rec_len as usize > data.len() {
            return Err(());
        }

        // Validate name_len
        if name_len as usize + 8 > rec_len as usize {
            return Err(());
        }

        // Extract name
        let name_bytes = &data[8..8 + name_len as usize];
        let name = String::from_utf8_lossy(name_bytes).to_string();

        Ok(Self {
            inode,
            rec_len,
            name_len,
            file_type,
            name,
        })
    }

    /// Check if this entry is valid (non-zero inode)
    fn is_valid(&self) -> bool {
        self.inode != 0
    }

    /// Check if this is a directory
    fn is_dir(&self) -> bool {
        self.file_type == EXT4_FT_DIR
    }
}

/// Find an entry in a directory by name
///
/// # Arguments
///
/// * `device` - Block device to read from
/// * `sb` - Superblock
/// * `dir_inode` - Directory inode to search in
/// * `name` - Name to search for
///
/// # Returns
///
/// * `Ok(Some((inode_num, file_type)))` - Found the entry
/// * `Ok(None)` - Entry not found
/// * `Err(FsError)` - Read error
pub fn find_entry_in_dir(
    device: &dyn BlockDevice,
    sb: &Ext4Superblock,
    dir_inode: &Inode,
    name: &str,
) -> Result<Option<(u32, u8)>, FsError> {
    // Read directory data
    let dir_data = extent::read_file_data(device, sb, dir_inode)?;

    // Parse directory entries
    let mut offset = 0;
    while offset < dir_data.len() {
        if offset + 8 > dir_data.len() {
            break;  // Not enough space for a valid entry
        }

        match DirEntry::from_bytes(&dir_data[offset..]) {
            Ok(entry) => {
                if entry.is_valid() && entry.name == name {
                    return Ok(Some((entry.inode, entry.file_type)));
                }

                // Move to next entry
                offset += entry.rec_len as usize;

                // Safety: prevent infinite loop on corrupt data
                if entry.rec_len == 0 {
                    break;
                }
            }
            Err(_) => break,  // Parse error, stop processing
        }
    }

    Ok(None)  // Not found
}

/// Read all entries from a directory
///
/// # Arguments
///
/// * `device` - Block device to read from
/// * `sb` - Superblock
/// * `dir_inode` - Directory inode to read from
///
/// # Returns
///
/// * `Ok(Vec<VfsDirEntry>)` - List of directory entries
/// * `Err(FsError)` - Read error
pub fn read_dir_entries(
    device: &dyn BlockDevice,
    sb: &Ext4Superblock,
    dir_inode: &Inode,
) -> Result<Vec<VfsDirEntry>, FsError> {
    // Read directory data
    let dir_data = extent::read_file_data(device, sb, dir_inode)?;

    let mut entries = Vec::new();
    let mut offset = 0;

    while offset < dir_data.len() {
        if offset + 8 > dir_data.len() {
            break;  // Not enough space for a valid entry
        }

        match DirEntry::from_bytes(&dir_data[offset..]) {
            Ok(entry) => {
                // Skip invalid entries, ".", and ".."
                if entry.is_valid() && entry.name != "." && entry.name != ".." {
                    entries.push(VfsDirEntry {
                        name: entry.name.clone(),
                        is_dir: entry.is_dir(),
                    });
                }

                // Move to next entry
                offset += entry.rec_len as usize;

                // Safety: prevent infinite loop on corrupt data
                if entry.rec_len == 0 {
                    break;
                }
            }
            Err(_) => break,  // Parse error, stop processing
        }
    }

    Ok(entries)
}
