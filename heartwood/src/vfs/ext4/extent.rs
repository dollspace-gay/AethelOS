//! ext4 Extent Tree Navigation
//!
//! Extent trees are used to efficiently map logical file blocks to physical
//! disk blocks. An extent is a contiguous range of blocks, which is much more
//! efficient than storing individual block pointers.
//!
//! The extent tree is stored in the inode's i_block field (60 bytes).

use crate::vfs::block_device::{BlockDevice, BlockDeviceError};
use crate::vfs::FsError;
use super::superblock::Ext4Superblock;
use super::inode::Inode;
use alloc::vec::Vec;

/// Extent tree header (12 bytes)
///
/// Located at the start of each extent tree node.
#[derive(Debug, Clone, Copy)]
struct ExtentHeader {
    /// Magic number (0xF30A)
    eh_magic: u16,
    /// Number of valid entries following the header
    eh_entries: u16,
    /// Maximum number of entries that could follow
    eh_max: u16,
    /// Depth of tree (0 = leaf node, >0 = index node)
    eh_depth: u16,
    /// Generation of the tree
    eh_generation: u32,
}

impl ExtentHeader {
    const MAGIC: u16 = 0xF30A;

    /// Parse extent header from bytes
    fn from_bytes(data: &[u8]) -> Result<Self, ()> {
        if data.len() < 12 {
            return Err(());
        }

        let eh_magic = u16::from_le_bytes([data[0], data[1]]);
        if eh_magic != Self::MAGIC {
            return Err(());
        }

        let eh_entries = u16::from_le_bytes([data[2], data[3]]);
        let eh_max = u16::from_le_bytes([data[4], data[5]]);
        let eh_depth = u16::from_le_bytes([data[6], data[7]]);
        let eh_generation = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

        Ok(Self {
            eh_magic,
            eh_entries,
            eh_max,
            eh_depth,
            eh_generation,
        })
    }
}

/// Extent tree index entry (12 bytes)
///
/// Points to a child node in the extent tree.
#[derive(Debug, Clone, Copy)]
struct ExtentIndex {
    /// First logical block covered by this index
    ei_block: u32,
    /// Lower 32 bits of physical block where child node is located
    ei_leaf_lo: u32,
    /// Upper 16 bits of physical block
    ei_leaf_hi: u16,
    /// Unused
    ei_unused: u16,
}

impl ExtentIndex {
    /// Parse extent index from bytes
    fn from_bytes(data: &[u8]) -> Result<Self, ()> {
        if data.len() < 12 {
            return Err(());
        }

        let ei_block = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let ei_leaf_lo = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let ei_leaf_hi = u16::from_le_bytes([data[8], data[9]]);
        let ei_unused = u16::from_le_bytes([data[10], data[11]]);

        Ok(Self {
            ei_block,
            ei_leaf_lo,
            ei_leaf_hi,
            ei_unused,
        })
    }

    /// Get the full physical block number
    fn physical_block(&self) -> u64 {
        (self.ei_leaf_hi as u64) << 32 | self.ei_leaf_lo as u64
    }
}

/// Extent leaf entry (12 bytes)
///
/// Maps a logical block range to a physical block range.
#[derive(Debug, Clone, Copy)]
struct Extent {
    /// First logical block covered by this extent
    ee_block: u32,
    /// Number of blocks covered by this extent
    ee_len: u16,
    /// Upper 16 bits of physical block
    ee_start_hi: u16,
    /// Lower 32 bits of physical block
    ee_start_lo: u32,
}

impl Extent {
    /// Parse extent from bytes
    fn from_bytes(data: &[u8]) -> Result<Self, ()> {
        if data.len() < 12 {
            return Err(());
        }

        let ee_block = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let ee_len = u16::from_le_bytes([data[4], data[5]]);
        let ee_start_hi = u16::from_le_bytes([data[6], data[7]]);
        let ee_start_lo = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

        Ok(Self {
            ee_block,
            ee_len,
            ee_start_hi,
            ee_start_lo,
        })
    }

    /// Get the full physical start block number
    fn physical_start(&self) -> u64 {
        (self.ee_start_hi as u64) << 32 | self.ee_start_lo as u64
    }

    /// Check if this extent covers the given logical block
    fn contains(&self, logical_block: u32) -> bool {
        logical_block >= self.ee_block && logical_block < self.ee_block + self.ee_len as u32
    }

    /// Get physical block for a logical block within this extent
    fn translate(&self, logical_block: u32) -> Option<u64> {
        if self.contains(logical_block) {
            let offset = logical_block - self.ee_block;
            Some(self.physical_start() + offset as u64)
        } else {
            None
        }
    }
}

/// Read a block from the device
fn read_block(
    device: &dyn BlockDevice,
    sb: &Ext4Superblock,
    block_num: u64,
) -> Result<Vec<u8>, BlockDeviceError> {
    let block_size = sb.block_size();
    let sector_size = device.sector_size() as u64;
    let block_offset = block_num * block_size as u64;
    let start_sector = block_offset / sector_size;
    let sectors_per_block = (block_size as u64 + sector_size - 1) / sector_size;

    let mut data = device.read_sectors(start_sector, sectors_per_block as u32)?;

    // Trim to exact block size if we read extra sectors
    let offset = (block_offset % sector_size) as usize;
    data.drain(0..offset);
    data.truncate(block_size as usize);

    Ok(data)
}

/// Lookup a logical block in the extent tree
///
/// # Arguments
///
/// * `device` - Block device to read from
/// * `sb` - Superblock
/// * `extent_data` - Root extent tree data (from inode's i_block field)
/// * `logical_block` - Logical block number to look up
///
/// # Returns
///
/// * `Ok(Some(physical_block))` - Found the mapping
/// * `Ok(None)` - Block is sparse (not allocated)
/// * `Err(BlockDeviceError)` - Read error or corrupt extent tree
fn lookup_extent(
    device: &dyn BlockDevice,
    sb: &Ext4Superblock,
    extent_data: &[u8],
    logical_block: u32,
) -> Result<Option<u64>, BlockDeviceError> {
    // Parse header
    let header = ExtentHeader::from_bytes(extent_data)
        .map_err(|_| BlockDeviceError::IoError)?;

    if header.eh_entries == 0 {
        // Empty extent tree = sparse block
        return Ok(None);
    }

    if header.eh_depth == 0 {
        // Leaf node - search for extent
        for i in 0..header.eh_entries {
            let offset = 12 + i as usize * 12;
            if offset + 12 > extent_data.len() {
                break;
            }

            let extent = Extent::from_bytes(&extent_data[offset..])
                .map_err(|_| BlockDeviceError::IoError)?;

            if let Some(physical) = extent.translate(logical_block) {
                return Ok(Some(physical));
            }
        }

        // Not found in any extent = sparse block
        Ok(None)
    } else {
        // Index node - find the right child
        let mut child_block: Option<u64> = None;

        for i in 0..header.eh_entries {
            let offset = 12 + i as usize * 12;
            if offset + 12 > extent_data.len() {
                break;
            }

            let index = ExtentIndex::from_bytes(&extent_data[offset..])
                .map_err(|_| BlockDeviceError::IoError)?;

            // Check if this index covers our logical block
            // (next index starts where this one should end)
            let covers = if i + 1 < header.eh_entries {
                let next_offset = 12 + (i + 1) as usize * 12;
                if next_offset + 12 <= extent_data.len() {
                    let next_index = ExtentIndex::from_bytes(&extent_data[next_offset..])
                        .map_err(|_| BlockDeviceError::IoError)?;
                    logical_block >= index.ei_block && logical_block < next_index.ei_block
                } else {
                    logical_block >= index.ei_block
                }
            } else {
                // Last index covers everything from ei_block onward
                logical_block >= index.ei_block
            };

            if covers {
                child_block = Some(index.physical_block());
                break;
            }
        }

        match child_block {
            Some(block) => {
                // Read child node and recurse
                let child_data = read_block(device, sb, block)?;
                lookup_extent(device, sb, &child_data, logical_block)
            }
            None => Ok(None),  // Sparse block
        }
    }
}

/// Read file data using extent tree
///
/// # Arguments
///
/// * `device` - Block device to read from
/// * `sb` - Superblock
/// * `inode` - Inode to read from
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - File contents
/// * `Err(FsError)` - Read error
pub fn read_file_data(
    device: &dyn BlockDevice,
    sb: &Ext4Superblock,
    inode: &Inode,
) -> Result<Vec<u8>, FsError> {
    if !inode.uses_extents() {
        // TODO: Implement indirect block reading for old-style inodes
        return Err(FsError::NotSupported);
    }

    let file_size = inode.size() as usize;
    let block_size = sb.block_size() as usize;
    let num_blocks = (file_size + block_size - 1) / block_size;

    let mut result = Vec::with_capacity(file_size);

    // Read each logical block
    for logical_block in 0..num_blocks as u32 {
        match lookup_extent(device, sb, &inode.i_block, logical_block) {
            Ok(Some(physical_block)) => {
                // Read the physical block
                let block_data = read_block(device, sb, physical_block)
                    .map_err(|_| FsError::IoError)?;

                // Determine how many bytes to take from this block
                let bytes_read = result.len();
                let bytes_remaining = file_size - bytes_read;
                let bytes_to_copy = core::cmp::min(block_size, bytes_remaining);

                result.extend_from_slice(&block_data[..bytes_to_copy]);
            }
            Ok(None) => {
                // Sparse block - fill with zeros
                let bytes_read = result.len();
                let bytes_remaining = file_size - bytes_read;
                let bytes_to_zero = core::cmp::min(block_size, bytes_remaining);

                result.resize(result.len() + bytes_to_zero, 0);
            }
            Err(_) => return Err(FsError::IoError),
        }
    }

    Ok(result)
}

/// Read specific blocks from file using extent tree
///
/// # Arguments
///
/// * `device` - Block device to read from
/// * `sb` - Superblock
/// * `inode` - Inode to read from
/// * `start_block` - First logical block to read
/// * `num_blocks` - Number of blocks to read
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - Block contents
/// * `Err(FsError)` - Read error
pub fn read_blocks(
    device: &dyn BlockDevice,
    sb: &Ext4Superblock,
    inode: &Inode,
    start_block: u32,
    num_blocks: u32,
) -> Result<Vec<u8>, FsError> {
    if !inode.uses_extents() {
        return Err(FsError::NotSupported);
    }

    let block_size = sb.block_size() as usize;
    let mut result = Vec::with_capacity(block_size * num_blocks as usize);

    for logical_block in start_block..start_block + num_blocks {
        match lookup_extent(device, sb, &inode.i_block, logical_block) {
            Ok(Some(physical_block)) => {
                let block_data = read_block(device, sb, physical_block)
                    .map_err(|_| FsError::IoError)?;
                result.extend_from_slice(&block_data);
            }
            Ok(None) => {
                // Sparse block - fill with zeros
                result.resize(result.len() + block_size, 0);
            }
            Err(_) => return Err(FsError::IoError),
        }
    }

    Ok(result)
}
