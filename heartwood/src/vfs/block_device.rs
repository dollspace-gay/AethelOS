//! Block Device Abstraction
//!
//! Provides a generic interface for reading/writing fixed-size blocks (sectors)
//! from storage devices like hard disks, SSDs, USB drives, etc.

use alloc::vec::Vec;

/// Block device errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceError {
    /// Device not ready or not present
    NotReady,
    /// I/O error occurred
    IoError,
    /// Invalid sector number (out of bounds)
    InvalidSector,
    /// Device is write-protected
    WriteProtected,
    /// Operation not supported by this device
    NotSupported,
}

impl core::fmt::Display for BlockDeviceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BlockDeviceError::NotReady => write!(f, "Device not ready"),
            BlockDeviceError::IoError => write!(f, "I/O error"),
            BlockDeviceError::InvalidSector => write!(f, "Invalid sector number"),
            BlockDeviceError::WriteProtected => write!(f, "Device is write-protected"),
            BlockDeviceError::NotSupported => write!(f, "Operation not supported"),
        }
    }
}

/// Block Device trait
///
/// Represents a storage device that can be read/written in fixed-size blocks.
/// Most storage devices use 512-byte sectors, but this can vary.
///
/// # Thread Safety
///
/// Implementations must be Send + Sync as they may be accessed from multiple
/// threads during concurrent filesystem operations.
///
/// # Example
///
/// ```
/// use block_device::{BlockDevice, BlockDeviceError};
///
/// fn read_boot_sector(device: &dyn BlockDevice) -> Result<Vec<u8>, BlockDeviceError> {
///     device.read_sector(0) // Read first sector
/// }
/// ```
pub trait BlockDevice: Send + Sync {
    /// Get the size of a single sector/block in bytes
    ///
    /// Common values: 512, 1024, 2048, 4096
    fn sector_size(&self) -> u32;

    /// Get the total number of sectors on this device
    fn sector_count(&self) -> u64;

    /// Read a single sector
    ///
    /// # Arguments
    ///
    /// * `sector` - Sector number (0-indexed)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - Sector data (length = sector_size())
    /// * `Err(InvalidSector)` - Sector number >= sector_count()
    /// * `Err(IoError)` - Read failed
    fn read_sector(&self, sector: u64) -> Result<Vec<u8>, BlockDeviceError>;

    /// Read multiple consecutive sectors
    ///
    /// Default implementation reads sectors one at a time, but devices
    /// can override for better performance.
    ///
    /// # Arguments
    ///
    /// * `start_sector` - First sector to read
    /// * `count` - Number of sectors to read
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - All sector data concatenated (length = count * sector_size())
    /// * `Err(InvalidSector)` - start_sector + count > sector_count()
    /// * `Err(IoError)` - Read failed
    fn read_sectors(&self, start_sector: u64, count: u32) -> Result<Vec<u8>, BlockDeviceError> {
        if start_sector + count as u64 > self.sector_count() {
            return Err(BlockDeviceError::InvalidSector);
        }

        let sector_size = self.sector_size() as usize;
        let mut buffer = Vec::with_capacity(sector_size * count as usize);

        for i in 0..count {
            let sector_data = self.read_sector(start_sector + i as u64)?;
            buffer.extend_from_slice(&sector_data);
        }

        Ok(buffer)
    }

    /// Write a single sector
    ///
    /// # Arguments
    ///
    /// * `sector` - Sector number (0-indexed)
    /// * `data` - Data to write (length must equal sector_size())
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Write successful
    /// * `Err(InvalidSector)` - Sector number >= sector_count()
    /// * `Err(WriteProtected)` - Device is read-only
    /// * `Err(IoError)` - Write failed
    fn write_sector(&self, sector: u64, data: &[u8]) -> Result<(), BlockDeviceError>;

    /// Write multiple consecutive sectors
    ///
    /// Default implementation writes sectors one at a time.
    ///
    /// # Arguments
    ///
    /// * `start_sector` - First sector to write
    /// * `data` - Data to write (length must be multiple of sector_size())
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Write successful
    /// * `Err(InvalidSector)` - Invalid sector range
    /// * `Err(WriteProtected)` - Device is read-only
    /// * `Err(IoError)` - Write failed
    fn write_sectors(&self, start_sector: u64, data: &[u8]) -> Result<(), BlockDeviceError> {
        let sector_size = self.sector_size() as usize;
        if data.len() % sector_size != 0 {
            return Err(BlockDeviceError::IoError);
        }

        let count = data.len() / sector_size;
        if start_sector + count as u64 > self.sector_count() {
            return Err(BlockDeviceError::InvalidSector);
        }

        for i in 0..count {
            let offset = i * sector_size;
            let sector_data = &data[offset..offset + sector_size];
            self.write_sector(start_sector + i as u64, sector_data)?;
        }

        Ok(())
    }

    /// Flush any cached writes to the device
    ///
    /// Ensures all pending writes are committed to persistent storage.
    fn sync(&self) -> Result<(), BlockDeviceError>;

    /// Check if the device is read-only
    fn is_read_only(&self) -> bool {
        false
    }
}
