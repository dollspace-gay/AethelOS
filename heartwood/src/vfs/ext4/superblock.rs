//! ext4 Superblock and Group Descriptor Parsing
//!
//! The superblock contains critical filesystem metadata including:
//! - Block size, inode size, and counts
//! - Feature flags (extents, 64-bit, etc.)
//! - Group descriptor location
//! - Filesystem state and mount information

use crate::vfs::block_device::{BlockDevice, BlockDeviceError};
use alloc::vec::Vec;

/// ext4 superblock magic number (0xEF53)
const EXT4_SUPER_MAGIC: u16 = 0xEF53;

/// Superblock is located at byte offset 1024 from start of partition
const SUPERBLOCK_OFFSET: u64 = 1024;

/// ext4 Superblock
///
/// Contains filesystem metadata. The superblock is 1024 bytes and located
/// at byte offset 1024 from the start of the partition.
#[derive(Debug, Clone)]
pub struct Ext4Superblock {
    /// Total number of inodes
    pub s_inodes_count: u32,
    /// Total number of blocks
    pub s_blocks_count: u64,
    /// Number of reserved blocks
    pub s_r_blocks_count: u64,
    /// Number of free blocks
    pub s_free_blocks_count: u64,
    /// Number of free inodes
    pub s_free_inodes_count: u32,
    /// First data block (0 for 1KB blocks, 1 for larger)
    pub s_first_data_block: u32,
    /// Block size (computed as 1024 << s_log_block_size)
    pub s_log_block_size: u32,
    /// Fragment size
    pub s_log_frag_size: u32,
    /// Blocks per group
    pub s_blocks_per_group: u32,
    /// Fragments per group
    pub s_frags_per_group: u32,
    /// Inodes per group
    pub s_inodes_per_group: u32,
    /// Mount time
    pub s_mtime: u32,
    /// Write time
    pub s_wtime: u32,
    /// Mount count
    pub s_mnt_count: u16,
    /// Maximum mount count
    pub s_max_mnt_count: u16,
    /// Magic signature (0xEF53)
    pub s_magic: u16,
    /// Filesystem state
    pub s_state: u16,
    /// Error behavior
    pub s_errors: u16,
    /// Minor revision level
    pub s_minor_rev_level: u16,
    /// Time of last check
    pub s_lastcheck: u32,
    /// Maximum time between checks
    pub s_checkinterval: u32,
    /// Creator OS
    pub s_creator_os: u32,
    /// Revision level
    pub s_rev_level: u32,
    /// Default uid for reserved blocks
    pub s_def_resuid: u16,
    /// Default gid for reserved blocks
    pub s_def_resgid: u16,

    // Extended fields (rev_level >= 1)
    /// First non-reserved inode
    pub s_first_ino: u32,
    /// Size of inode structure (typically 128 or 256)
    pub s_inode_size: u16,
    /// Block group this superblock is part of
    pub s_block_group_nr: u16,
    /// Compatible features
    pub s_feature_compat: u32,
    /// Incompatible features
    pub s_feature_incompat: u32,
    /// Read-only compatible features
    pub s_feature_ro_compat: u32,
    /// 128-bit UUID
    pub s_uuid: [u8; 16],
    /// Volume name
    pub s_volume_name: [u8; 16],
    /// Last mounted path
    pub s_last_mounted: [u8; 64],

    // Additional fields for 64-bit support
    /// High 32 bits of blocks count
    pub s_blocks_count_hi: u32,
    /// High 32 bits of reserved blocks count
    pub s_r_blocks_count_hi: u32,
    /// High 32 bits of free blocks count
    pub s_free_blocks_count_hi: u32,

    /// Descriptor size (for 64-bit mode)
    pub s_desc_size: u16,
}

impl Ext4Superblock {
    /// Read superblock from a block device
    ///
    /// # Arguments
    ///
    /// * `device` - Block device to read from
    ///
    /// # Returns
    ///
    /// * `Ok(Ext4Superblock)` - Successfully parsed superblock
    /// * `Err(BlockDeviceError)` - Read error or invalid superblock
    pub fn from_device(device: &dyn BlockDevice) -> Result<Self, BlockDeviceError> {
        let sector_size = device.sector_size() as u64;

        // Calculate which sector contains the superblock
        // Superblock starts at byte 1024
        let start_sector = SUPERBLOCK_OFFSET / sector_size;
        let offset_in_sector = (SUPERBLOCK_OFFSET % sector_size) as usize;

        // Read enough sectors to get the full superblock (1024 bytes)
        let sectors_needed = ((offset_in_sector + 1024 + sector_size as usize - 1) / sector_size as usize) as u32;
        let data = device.read_sectors(start_sector, sectors_needed)?;

        // Extract superblock data starting at the offset
        let sb_data = &data[offset_in_sector..offset_in_sector + 1024];

        // Parse superblock fields (little-endian)
        let s_inodes_count = u32::from_le_bytes([sb_data[0], sb_data[1], sb_data[2], sb_data[3]]);
        let s_blocks_count_lo = u32::from_le_bytes([sb_data[4], sb_data[5], sb_data[6], sb_data[7]]);
        let s_r_blocks_count_lo = u32::from_le_bytes([sb_data[8], sb_data[9], sb_data[10], sb_data[11]]);
        let s_free_blocks_count_lo = u32::from_le_bytes([sb_data[12], sb_data[13], sb_data[14], sb_data[15]]);
        let s_free_inodes_count = u32::from_le_bytes([sb_data[16], sb_data[17], sb_data[18], sb_data[19]]);
        let s_first_data_block = u32::from_le_bytes([sb_data[20], sb_data[21], sb_data[22], sb_data[23]]);
        let s_log_block_size = u32::from_le_bytes([sb_data[24], sb_data[25], sb_data[26], sb_data[27]]);
        let s_log_frag_size = u32::from_le_bytes([sb_data[28], sb_data[29], sb_data[30], sb_data[31]]);
        let s_blocks_per_group = u32::from_le_bytes([sb_data[32], sb_data[33], sb_data[34], sb_data[35]]);
        let s_frags_per_group = u32::from_le_bytes([sb_data[36], sb_data[37], sb_data[38], sb_data[39]]);
        let s_inodes_per_group = u32::from_le_bytes([sb_data[40], sb_data[41], sb_data[42], sb_data[43]]);
        let s_mtime = u32::from_le_bytes([sb_data[44], sb_data[45], sb_data[46], sb_data[47]]);
        let s_wtime = u32::from_le_bytes([sb_data[48], sb_data[49], sb_data[50], sb_data[51]]);
        let s_mnt_count = u16::from_le_bytes([sb_data[52], sb_data[53]]);
        let s_max_mnt_count = u16::from_le_bytes([sb_data[54], sb_data[55]]);
        let s_magic = u16::from_le_bytes([sb_data[56], sb_data[57]]);
        let s_state = u16::from_le_bytes([sb_data[58], sb_data[59]]);
        let s_errors = u16::from_le_bytes([sb_data[60], sb_data[61]]);
        let s_minor_rev_level = u16::from_le_bytes([sb_data[62], sb_data[63]]);
        let s_lastcheck = u32::from_le_bytes([sb_data[64], sb_data[65], sb_data[66], sb_data[67]]);
        let s_checkinterval = u32::from_le_bytes([sb_data[68], sb_data[69], sb_data[70], sb_data[71]]);
        let s_creator_os = u32::from_le_bytes([sb_data[72], sb_data[73], sb_data[74], sb_data[75]]);
        let s_rev_level = u32::from_le_bytes([sb_data[76], sb_data[77], sb_data[78], sb_data[79]]);
        let s_def_resuid = u16::from_le_bytes([sb_data[80], sb_data[81]]);
        let s_def_resgid = u16::from_le_bytes([sb_data[82], sb_data[83]]);

        // Extended fields (offset 84+)
        let s_first_ino = u32::from_le_bytes([sb_data[84], sb_data[85], sb_data[86], sb_data[87]]);
        let s_inode_size = u16::from_le_bytes([sb_data[88], sb_data[89]]);
        let s_block_group_nr = u16::from_le_bytes([sb_data[90], sb_data[91]]);
        let s_feature_compat = u32::from_le_bytes([sb_data[92], sb_data[93], sb_data[94], sb_data[95]]);
        let s_feature_incompat = u32::from_le_bytes([sb_data[96], sb_data[97], sb_data[98], sb_data[99]]);
        let s_feature_ro_compat = u32::from_le_bytes([sb_data[100], sb_data[101], sb_data[102], sb_data[103]]);

        let mut s_uuid = [0u8; 16];
        s_uuid.copy_from_slice(&sb_data[104..120]);

        let mut s_volume_name = [0u8; 16];
        s_volume_name.copy_from_slice(&sb_data[120..136]);

        let mut s_last_mounted = [0u8; 64];
        s_last_mounted.copy_from_slice(&sb_data[136..200]);

        // 64-bit fields (offset 0x150+)
        let s_blocks_count_hi = u32::from_le_bytes([sb_data[0x150], sb_data[0x151], sb_data[0x152], sb_data[0x153]]);
        let s_r_blocks_count_hi = u32::from_le_bytes([sb_data[0x154], sb_data[0x155], sb_data[0x156], sb_data[0x157]]);
        let s_free_blocks_count_hi = u32::from_le_bytes([sb_data[0x158], sb_data[0x159], sb_data[0x15A], sb_data[0x15B]]);

        // Descriptor size (offset 0xFE)
        let s_desc_size = u16::from_le_bytes([sb_data[0xFE], sb_data[0xFF]]);

        // Combine 64-bit values
        let s_blocks_count = (s_blocks_count_hi as u64) << 32 | s_blocks_count_lo as u64;
        let s_r_blocks_count = (s_r_blocks_count_hi as u64) << 32 | s_r_blocks_count_lo as u64;
        let s_free_blocks_count = (s_free_blocks_count_hi as u64) << 32 | s_free_blocks_count_lo as u64;

        Ok(Self {
            s_inodes_count,
            s_blocks_count,
            s_r_blocks_count,
            s_free_blocks_count,
            s_free_inodes_count,
            s_first_data_block,
            s_log_block_size,
            s_log_frag_size,
            s_blocks_per_group,
            s_frags_per_group,
            s_inodes_per_group,
            s_mtime,
            s_wtime,
            s_mnt_count,
            s_max_mnt_count,
            s_magic,
            s_state,
            s_errors,
            s_minor_rev_level,
            s_lastcheck,
            s_checkinterval,
            s_creator_os,
            s_rev_level,
            s_def_resuid,
            s_def_resgid,
            s_first_ino,
            s_inode_size,
            s_block_group_nr,
            s_feature_compat,
            s_feature_incompat,
            s_feature_ro_compat,
            s_uuid,
            s_volume_name,
            s_last_mounted,
            s_blocks_count_hi,
            s_r_blocks_count_hi,
            s_free_blocks_count_hi,
            s_desc_size,
        })
    }

    /// Check if this is a valid ext4 superblock
    pub fn is_valid(&self) -> bool {
        self.s_magic == EXT4_SUPER_MAGIC
    }

    /// Get the block size in bytes
    pub fn block_size(&self) -> u32 {
        1024 << self.s_log_block_size
    }

    /// Get the number of block groups
    pub fn block_group_count(&self) -> u32 {
        ((self.s_blocks_count + self.s_blocks_per_group as u64 - 1) / self.s_blocks_per_group as u64) as u32
    }

    /// Check if 64-bit feature is enabled
    pub fn has_64bit(&self) -> bool {
        const INCOMPAT_64BIT: u32 = 0x0080;
        (self.s_feature_incompat & INCOMPAT_64BIT) != 0
    }

    /// Check if extents feature is enabled
    pub fn has_extents(&self) -> bool {
        const INCOMPAT_EXTENTS: u32 = 0x0040;
        (self.s_feature_incompat & INCOMPAT_EXTENTS) != 0
    }

    /// Check if flex_bg feature is enabled
    pub fn has_flex_bg(&self) -> bool {
        const INCOMPAT_FLEX_BG: u32 = 0x0200;
        (self.s_feature_incompat & INCOMPAT_FLEX_BG) != 0
    }

    /// Get descriptor size (32 bytes for normal, 64 bytes for 64-bit)
    pub fn descriptor_size(&self) -> u16 {
        if self.has_64bit() && self.s_desc_size != 0 {
            self.s_desc_size
        } else {
            32  // Default descriptor size
        }
    }
}

/// Block Group Descriptor
///
/// Each block group has a descriptor that specifies where its
/// inode table, block bitmap, and inode bitmap are located.
#[derive(Debug, Clone)]
pub struct BlockGroupDesc {
    /// Block number of block bitmap
    pub bg_block_bitmap: u64,
    /// Block number of inode bitmap
    pub bg_inode_bitmap: u64,
    /// Block number of inode table
    pub bg_inode_table: u64,
    /// Number of free blocks
    pub bg_free_blocks_count: u32,
    /// Number of free inodes
    pub bg_free_inodes_count: u32,
    /// Number of directories
    pub bg_used_dirs_count: u32,
}

impl BlockGroupDesc {
    /// Read block group descriptor from device
    ///
    /// # Arguments
    ///
    /// * `device` - Block device to read from
    /// * `sb` - Superblock containing filesystem info
    /// * `group_num` - Block group number (0-indexed)
    ///
    /// # Returns
    ///
    /// * `Ok(BlockGroupDesc)` - Successfully parsed descriptor
    /// * `Err(BlockDeviceError)` - Read error
    pub fn from_device(
        device: &dyn BlockDevice,
        sb: &Ext4Superblock,
        group_num: u32,
    ) -> Result<Self, BlockDeviceError> {
        let block_size = sb.block_size();
        let desc_size = sb.descriptor_size() as u32;

        // Group descriptor table starts after the superblock
        // If block size is 1024, it's at block 2. Otherwise, block 1.
        let gdt_block = if block_size == 1024 { 2 } else { 1 };

        // Calculate descriptor offset
        let desc_offset = group_num * desc_size;
        let desc_block = gdt_block + (desc_offset / block_size);
        let offset_in_block = desc_offset % block_size;

        // Read the block containing this descriptor
        let sector_size = device.sector_size() as u64;
        let block_offset = desc_block as u64 * block_size as u64;
        let start_sector = block_offset / sector_size;
        let sectors_per_block = (block_size as u64 + sector_size - 1) / sector_size;

        let data = device.read_sectors(start_sector, sectors_per_block as u32)?;
        let offset = (block_offset % sector_size) as usize + offset_in_block as usize;

        // Parse descriptor (first 32 bytes are always present)
        let bg_block_bitmap_lo = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        let bg_inode_bitmap_lo = u32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
        let bg_inode_table_lo = u32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
        let bg_free_blocks_count_lo = u16::from_le_bytes([data[offset+12], data[offset+13]]);
        let bg_free_inodes_count_lo = u16::from_le_bytes([data[offset+14], data[offset+15]]);
        let bg_used_dirs_count_lo = u16::from_le_bytes([data[offset+16], data[offset+17]]);

        // For 64-bit, read high 32 bits (if descriptor is 64 bytes)
        let (bg_block_bitmap_hi, bg_inode_bitmap_hi, bg_inode_table_hi) = if desc_size >= 64 {
            let hi1 = u32::from_le_bytes([data[offset+32], data[offset+33], data[offset+34], data[offset+35]]);
            let hi2 = u32::from_le_bytes([data[offset+36], data[offset+37], data[offset+38], data[offset+39]]);
            let hi3 = u32::from_le_bytes([data[offset+40], data[offset+41], data[offset+42], data[offset+43]]);
            (hi1, hi2, hi3)
        } else {
            (0, 0, 0)
        };

        Ok(Self {
            bg_block_bitmap: (bg_block_bitmap_hi as u64) << 32 | bg_block_bitmap_lo as u64,
            bg_inode_bitmap: (bg_inode_bitmap_hi as u64) << 32 | bg_inode_bitmap_lo as u64,
            bg_inode_table: (bg_inode_table_hi as u64) << 32 | bg_inode_table_lo as u64,
            bg_free_blocks_count: bg_free_blocks_count_lo as u32,
            bg_free_inodes_count: bg_free_inodes_count_lo as u32,
            bg_used_dirs_count: bg_used_dirs_count_lo as u32,
        })
    }
}
