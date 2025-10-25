//! Mock FAT32 Block Device for Testing
//!
//! This module provides a pre-built FAT32 filesystem image in memory
//! for testing the FAT32 driver without real hardware.

use super::block_device::{BlockDevice, BlockDeviceError};
use alloc::vec::Vec;

/// Mock block device containing a minimal FAT32 filesystem
///
/// This creates a small FAT32 volume with:
/// - Volume label: "AETHELOS"
/// - 2 files in root: "README.TXT", "TEST.TXT"
/// - Total size: 64KB (128 sectors × 512 bytes)
/// - Limited to 64KB due to buddy allocator's 64KB max allocation
pub struct MockFat32Device {
    data: Vec<u8>,
}

impl MockFat32Device {
    /// Create a new mock FAT32 device with test data
    pub fn new() -> Self {
        // Create a minimal FAT32 filesystem (128 sectors = 64KB)
        // NOTE: Limited to 64KB because the buddy allocator's MAX_ORDER = 10
        // allows maximum allocation of 64 * 2^10 = 64KB
        const TOTAL_SECTORS: usize = 128;
        const SECTOR_SIZE: usize = 512;

        let mut data = Vec::with_capacity(TOTAL_SECTORS * SECTOR_SIZE);
        data.resize(TOTAL_SECTORS * SECTOR_SIZE, 0u8);

        Self::write_boot_sector(&mut data);
        Self::write_fsinfo(&mut data);
        Self::write_backup_boot_sector(&mut data);
        Self::write_fat_tables(&mut data);
        Self::write_root_directory(&mut data);
        Self::write_file_data(&mut data);

        Self { data }
    }

    fn write_boot_sector(data: &mut [u8]) {
        let boot = &mut data[0..512];

        // Jump instruction (3 bytes)
        boot[0..3].copy_from_slice(&[0xEB, 0x58, 0x90]);

        // OEM Name: "AETHELOS" (8 bytes)
        boot[3..11].copy_from_slice(b"AETHELOS");

        // Bytes per sector: 512
        boot[11..13].copy_from_slice(&512u16.to_le_bytes());

        // Sectors per cluster: 1
        boot[13] = 1;

        // Reserved sectors: 32
        boot[14..16].copy_from_slice(&32u16.to_le_bytes());

        // Number of FATs: 2
        boot[16] = 2;

        // Root entries: 0 (FAT32)
        boot[17..19].copy_from_slice(&0u16.to_le_bytes());

        // Total sectors (16-bit): 0 (use 32-bit field)
        boot[19..21].copy_from_slice(&0u16.to_le_bytes());

        // Media descriptor: 0xF8 (hard disk)
        boot[21] = 0xF8;

        // Sectors per FAT (16-bit): 0 (use 32-bit field)
        boot[22..24].copy_from_slice(&0u16.to_le_bytes());

        // Sectors per track: 63
        boot[24..26].copy_from_slice(&63u16.to_le_bytes());

        // Number of heads: 255
        boot[26..28].copy_from_slice(&255u16.to_le_bytes());

        // Hidden sectors: 0
        boot[28..32].copy_from_slice(&0u32.to_le_bytes());

        // Total sectors (32-bit): 128
        boot[32..36].copy_from_slice(&128u32.to_le_bytes());

        // Sectors per FAT (32-bit): 8 (reduced for smaller volume)
        boot[36..40].copy_from_slice(&8u32.to_le_bytes());

        // Flags: 0
        boot[40..42].copy_from_slice(&0u16.to_le_bytes());

        // Version: 0.0
        boot[42..44].copy_from_slice(&0u16.to_le_bytes());

        // Root cluster: 2
        boot[44..48].copy_from_slice(&2u32.to_le_bytes());

        // FSInfo sector: 1
        boot[48..50].copy_from_slice(&1u16.to_le_bytes());

        // Backup boot sector: 6
        boot[50..52].copy_from_slice(&6u16.to_le_bytes());

        // Reserved (12 bytes): zeros

        // Drive number: 0x80 (hard disk)
        boot[64] = 0x80;

        // Reserved: 0
        boot[65] = 0;

        // Boot signature: 0x29
        boot[66] = 0x29;

        // Volume ID: 0x12345678
        boot[67..71].copy_from_slice(&0x12345678u32.to_le_bytes());

        // Volume label: "AETHELOS   " (11 bytes)
        boot[71..82].copy_from_slice(b"AETHELOS   ");

        // Filesystem type: "FAT32   " (8 bytes)
        boot[82..90].copy_from_slice(b"FAT32   ");

        // Boot signature: 0x55AA
        boot[510..512].copy_from_slice(&[0x55, 0xAA]);
    }

    fn write_fsinfo(data: &mut [u8]) {
        let fsinfo = &mut data[512..1024]; // Sector 1

        // Lead signature: 0x41615252 ("RRaA")
        fsinfo[0..4].copy_from_slice(&0x41615252u32.to_le_bytes());

        // Reserved (480 bytes): zeros

        // Structure signature: 0x61417272 ("rrAa")
        fsinfo[484..488].copy_from_slice(&0x61417272u32.to_le_bytes());

        // Free clusters: 77 (128 total - 48 reserved/FAT - 3 used = 77)
        fsinfo[488..492].copy_from_slice(&77u32.to_le_bytes());

        // Next free cluster: 5 (after root, readme, test)
        fsinfo[492..496].copy_from_slice(&5u32.to_le_bytes());

        // Reserved (12 bytes): zeros

        // Trail signature: 0xAA550000
        fsinfo[508..512].copy_from_slice(&0xAA550000u32.to_le_bytes());
    }

    fn write_backup_boot_sector(data: &mut [u8]) {
        // Copy boot sector to sector 6
        let boot_copy = data[0..512].to_vec();
        data[6 * 512..7 * 512].copy_from_slice(&boot_copy);
    }

    fn write_fat_tables(data: &mut [u8]) {
        // FAT1 starts at sector 32 (8 sectors)
        let fat1 = &mut data[32 * 512..(32 + 8) * 512];

        // FAT entry 0: Media descriptor + end-of-chain marker
        fat1[0..4].copy_from_slice(&0x0FFFFFF8u32.to_le_bytes());

        // FAT entry 1: End-of-chain marker (reserved)
        fat1[4..8].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        // FAT entry 2: End-of-chain (root directory, 1 cluster)
        fat1[8..12].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        // FAT entry 3: End-of-chain (README.TXT, 1 cluster)
        fat1[12..16].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        // FAT entry 4: End-of-chain (TEST.TXT, 1 cluster)
        fat1[16..20].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        // FAT2: Copy FAT1 to sector 40 (8 sectors)
        let fat1_copy = fat1.to_vec();
        let fat2 = &mut data[40 * 512..(40 + 8) * 512];
        fat2[..fat1_copy.len()].copy_from_slice(&fat1_copy);
    }

    fn write_root_directory(data: &mut [u8]) {
        // Root directory is at cluster 2
        // Data region starts at sector 48 (32 reserved + 16 FAT sectors)
        // Cluster 2 = first cluster = sector 48
        let root = &mut data[48 * 512..49 * 512];

        // Entry 1: README.TXT
        root[0..11].copy_from_slice(b"README  TXT");
        root[11] = 0x20; // Archive attribute
        root[26..28].copy_from_slice(&3u16.to_le_bytes()); // First cluster (low)
        root[20..22].copy_from_slice(&0u16.to_le_bytes()); // First cluster (high)
        root[28..32].copy_from_slice(&56u32.to_le_bytes()); // File size

        // Entry 2: TEST.TXT
        root[32..43].copy_from_slice(b"TEST    TXT");
        root[32 + 11] = 0x20; // Archive attribute
        root[32 + 26..32 + 28].copy_from_slice(&4u16.to_le_bytes()); // First cluster
        root[32 + 20..32 + 22].copy_from_slice(&0u16.to_le_bytes());
        root[32 + 28..32 + 32].copy_from_slice(&44u32.to_le_bytes()); // File size
    }

    fn write_file_data(data: &mut [u8]) {
        // README.TXT at cluster 3 = sector 49
        let readme = b"Welcome to AethelOS!\nThe symbiotic operating system.\n";
        data[49 * 512..49 * 512 + readme.len()].copy_from_slice(readme);

        // TEST.TXT at cluster 4 = sector 50
        let test = b"FAT32 driver is working!\nFSInfo loaded.\n";
        data[50 * 512..50 * 512 + test.len()].copy_from_slice(test);
    }
}

impl BlockDevice for MockFat32Device {
    fn sector_size(&self) -> u32 {
        512
    }

    fn sector_count(&self) -> u64 {
        (self.data.len() / 512) as u64
    }

    fn read_sector(&self, sector: u64) -> Result<Vec<u8>, BlockDeviceError> {
        let offset = sector as usize * 512;
        if offset + 512 > self.data.len() {
            return Err(BlockDeviceError::InvalidSector);
        }

        Ok(self.data[offset..offset + 512].to_vec())
    }

    fn write_sector(&self, _sector: u64, _data: &[u8]) -> Result<(), BlockDeviceError> {
        Err(BlockDeviceError::WriteProtected)
    }

    fn sync(&self) -> Result<(), BlockDeviceError> {
        Ok(())
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
