//! ext4 Inode Structures and Parsing
//!
//! Inodes contain file metadata including:
//! - File type, permissions, and ownership
//! - File size and block count
//! - Timestamps (creation, modification, access)
//! - Block pointers (direct, indirect, or extent tree)

use crate::vfs::block_device::{BlockDevice, BlockDeviceError};
use super::superblock::{Ext4Superblock, BlockGroupDesc};
use alloc::vec::Vec;

/// File type constants (high 4 bits of i_mode)
pub const S_IFREG: u16 = 0x8000;  // Regular file
pub const S_IFDIR: u16 = 0x4000;  // Directory
pub const S_IFLNK: u16 = 0xA000;  // Symbolic link
pub const S_IFCHR: u16 = 0x2000;  // Character device
pub const S_IFBLK: u16 = 0x6000;  // Block device
pub const S_IFIFO: u16 = 0x1000;  // FIFO
pub const S_IFSOCK: u16 = 0xC000; // Socket

/// Inode flags
pub const EXT4_EXTENTS_FL: u32 = 0x00080000;  // Inode uses extents
pub const EXT4_INLINE_DATA_FL: u32 = 0x10000000; // Inode has inline data

/// ext4 Inode
///
/// Represents a file or directory on the filesystem.
/// Inodes can be 128 or 256 bytes (specified in superblock).
#[derive(Debug, Clone)]
pub struct Inode {
    /// File mode (type and permissions)
    pub i_mode: u16,
    /// Owner UID
    pub i_uid: u32,
    /// File size (lower 32 bits)
    pub i_size_lo: u32,
    /// Access time
    pub i_atime: u32,
    /// Creation time
    pub i_ctime: u32,
    /// Modification time
    pub i_mtime: u32,
    /// Deletion time
    pub i_dtime: u32,
    /// Group ID
    pub i_gid: u32,
    /// Hard link count
    pub i_links_count: u16,
    /// Block count (in 512-byte sectors)
    pub i_blocks_lo: u32,
    /// Inode flags
    pub i_flags: u32,
    /// OS-specific value 1
    pub i_osd1: u32,
    /// Block pointers / extent tree (60 bytes)
    pub i_block: [u8; 60],
    /// File version (for NFS)
    pub i_generation: u32,
    /// Extended attribute block
    pub i_file_acl_lo: u32,
    /// File size (upper 32 bits) / directory ACL
    pub i_size_high: u32,

    // Extended fields (for 256-byte inodes)
    /// High 16 bits of block count
    pub i_blocks_high: u16,
    /// High 32 bits of file ACL
    pub i_file_acl_high: u16,
    /// High 16 bits of UID
    pub i_uid_high: u16,
    /// High 16 bits of GID
    pub i_gid_high: u16,

    /// Extra inode size
    pub i_extra_isize: u16,
}

impl Inode {
    /// Read an inode from a block device
    ///
    /// # Arguments
    ///
    /// * `device` - Block device to read from
    /// * `sb` - Superblock containing filesystem info
    /// * `inode_num` - Inode number (1-indexed as per ext4 spec)
    ///
    /// # Returns
    ///
    /// * `Ok(Inode)` - Successfully parsed inode
    /// * `Err(BlockDeviceError)` - Read error or invalid inode number
    pub fn read_from_device(
        device: &dyn BlockDevice,
        sb: &Ext4Superblock,
        inode_num: u32,
    ) -> Result<Self, BlockDeviceError> {
        // Inode numbers are 1-indexed
        if inode_num == 0 || inode_num > sb.s_inodes_count {
            return Err(BlockDeviceError::InvalidSector);
        }

        // Calculate which block group contains this inode
        let inode_index = inode_num - 1;
        let block_group = inode_index / sb.s_inodes_per_group;
        let index_in_group = inode_index % sb.s_inodes_per_group;

        // Read block group descriptor to find inode table location
        let bg_desc = BlockGroupDesc::from_device(device, sb, block_group)?;

        // Calculate inode offset within the inode table
        let inode_size = sb.s_inode_size as u32;
        let inode_table_block = bg_desc.bg_inode_table;
        let block_size = sb.block_size();

        let inode_offset_in_table = index_in_group * inode_size;
        let inode_block = inode_table_block + (inode_offset_in_table / block_size) as u64;
        let offset_in_block = inode_offset_in_table % block_size;

        // Read the block containing this inode
        let sector_size = device.sector_size() as u64;
        let block_offset = inode_block * block_size as u64;
        let start_sector = block_offset / sector_size;
        let sectors_per_block = (block_size as u64 + sector_size - 1) / sector_size;

        let data = device.read_sectors(start_sector, sectors_per_block as u32)?;
        let offset = (block_offset % sector_size) as usize + offset_in_block as usize;

        // Parse inode structure (first 128 bytes are standard)
        let i_mode = u16::from_le_bytes([data[offset], data[offset+1]]);
        let i_uid_lo = u16::from_le_bytes([data[offset+2], data[offset+3]]);
        let i_size_lo = u32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
        let i_atime = u32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
        let i_ctime = u32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
        let i_mtime = u32::from_le_bytes([data[offset+16], data[offset+17], data[offset+18], data[offset+19]]);
        let i_dtime = u32::from_le_bytes([data[offset+20], data[offset+21], data[offset+22], data[offset+23]]);
        let i_gid_lo = u16::from_le_bytes([data[offset+24], data[offset+25]]);
        let i_links_count = u16::from_le_bytes([data[offset+26], data[offset+27]]);
        let i_blocks_lo = u32::from_le_bytes([data[offset+28], data[offset+29], data[offset+30], data[offset+31]]);
        let i_flags = u32::from_le_bytes([data[offset+32], data[offset+33], data[offset+34], data[offset+35]]);
        let i_osd1 = u32::from_le_bytes([data[offset+36], data[offset+37], data[offset+38], data[offset+39]]);

        // i_block is at offset 40, 60 bytes
        let mut i_block = [0u8; 60];
        i_block.copy_from_slice(&data[offset+40..offset+100]);

        let i_generation = u32::from_le_bytes([data[offset+100], data[offset+101], data[offset+102], data[offset+103]]);
        let i_file_acl_lo = u32::from_le_bytes([data[offset+104], data[offset+105], data[offset+106], data[offset+107]]);
        let i_size_high = u32::from_le_bytes([data[offset+108], data[offset+109], data[offset+110], data[offset+111]]);

        // Extended fields (offset 116+)
        let i_blocks_high = if inode_size >= 128 {
            u16::from_le_bytes([data[offset+116], data[offset+117]])
        } else {
            0
        };

        let i_file_acl_high = if inode_size >= 128 {
            u16::from_le_bytes([data[offset+118], data[offset+119]])
        } else {
            0
        };

        let i_uid_high = if inode_size >= 128 {
            u16::from_le_bytes([data[offset+120], data[offset+121]])
        } else {
            0
        };

        let i_gid_high = if inode_size >= 128 {
            u16::from_le_bytes([data[offset+122], data[offset+123]])
        } else {
            0
        };

        let i_extra_isize = if inode_size >= 128 {
            u16::from_le_bytes([data[offset+128], data[offset+129]])
        } else {
            0
        };

        // Combine UID/GID fields
        let i_uid = (i_uid_high as u32) << 16 | i_uid_lo as u32;
        let i_gid = (i_gid_high as u32) << 16 | i_gid_lo as u32;

        Ok(Self {
            i_mode,
            i_uid,
            i_size_lo,
            i_atime,
            i_ctime,
            i_mtime,
            i_dtime,
            i_gid,
            i_links_count,
            i_blocks_lo,
            i_flags,
            i_osd1,
            i_block,
            i_generation,
            i_file_acl_lo,
            i_size_high,
            i_blocks_high,
            i_file_acl_high,
            i_uid_high,
            i_gid_high,
            i_extra_isize,
        })
    }

    /// Get file type from mode
    pub fn file_type(&self) -> u16 {
        self.i_mode & 0xF000
    }

    /// Check if this is a regular file
    pub fn is_file(&self) -> bool {
        self.file_type() == S_IFREG
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.file_type() == S_IFDIR
    }

    /// Check if this is a symbolic link
    pub fn is_symlink(&self) -> bool {
        self.file_type() == S_IFLNK
    }

    /// Get the full file size (combining lower and upper 32 bits)
    pub fn size(&self) -> u64 {
        if self.is_dir() {
            // For directories, i_size_high is actually dir_acl
            self.i_size_lo as u64
        } else {
            (self.i_size_high as u64) << 32 | self.i_size_lo as u64
        }
    }

    /// Check if inode uses extents
    pub fn uses_extents(&self) -> bool {
        (self.i_flags & EXT4_EXTENTS_FL) != 0
    }

    /// Check if inode has inline data
    pub fn has_inline_data(&self) -> bool {
        (self.i_flags & EXT4_INLINE_DATA_FL) != 0
    }

    /// Get block count (in filesystem blocks, not 512-byte sectors)
    pub fn block_count(&self, block_size: u32) -> u64 {
        // i_blocks is in 512-byte sectors
        let sectors = (self.i_blocks_high as u64) << 32 | self.i_blocks_lo as u64;
        (sectors * 512) / block_size as u64
    }
}
