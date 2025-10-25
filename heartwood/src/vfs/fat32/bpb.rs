//! FAT32 Boot Sector and BIOS Parameter Block (BPB) parsing
//!
//! The boot sector contains critical metadata about the FAT32 filesystem:
//! - Geometry: sectors per cluster, reserved sectors, etc.
//! - FAT location and size
//! - Root directory cluster
//! - Volume information
//! - FSInfo sector location (for free space tracking)
//! - Backup boot sector location (for redundancy)

use super::super::block_device::BlockDevice;
use alloc::string::{String, ToString};

/// FSInfo (File System Information) structure
///
/// FAT32-specific sector that tracks free space to speed up allocations.
/// Typically stored at sector 1 in the reserved region.
#[derive(Debug, Clone)]
pub struct FSInfo {
    /// Number of free clusters (0xFFFFFFFF if unknown)
    pub free_clusters: u32,
    /// Next free cluster hint (where to start searching)
    /// 0xFFFFFFFF if unknown, otherwise >= 2
    pub next_free: u32,
}

impl FSInfo {
    /// Parse FSInfo from sector data
    ///
    /// # Arguments
    ///
    /// * `data` - 512-byte FSInfo sector
    ///
    /// # Returns
    ///
    /// * `Ok(FSInfo)` - Successfully parsed FSInfo
    /// * `Err(&str)` - Invalid signatures or data
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 512 {
            return Err("FSInfo sector too small (need 512 bytes)");
        }

        // Check lead signature at offset 0 (0x41615252 = "RRaA")
        let lead_sig = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if lead_sig != 0x41615252 {
            return Err("Invalid FSInfo lead signature");
        }

        // Check structure signature at offset 484 (0x61417272 = "rrAa")
        let struct_sig = u32::from_le_bytes([data[484], data[485], data[486], data[487]]);
        if struct_sig != 0x61417272 {
            return Err("Invalid FSInfo structure signature");
        }

        // Check trail signature at offset 508 (0xAA550000)
        let trail_sig = u32::from_le_bytes([data[508], data[509], data[510], data[511]]);
        if trail_sig != 0xAA550000 {
            return Err("Invalid FSInfo trail signature");
        }

        // Free cluster count at offset 488
        let free_clusters = u32::from_le_bytes([data[488], data[489], data[490], data[491]]);

        // Next free cluster at offset 492
        let next_free = u32::from_le_bytes([data[492], data[493], data[494], data[495]]);

        Ok(FSInfo {
            free_clusters,
            next_free,
        })
    }

    /// Read and parse FSInfo from a block device
    pub fn from_device(device: &dyn BlockDevice, sector: u16) -> Result<Self, &'static str> {
        let data = device.read_sector(sector as u64)
            .map_err(|_| "Failed to read FSInfo sector")?;

        Self::parse(&data)
    }
}

/// FAT32 BIOS Parameter Block (BPB)
///
/// Contains filesystem metadata extracted from the boot sector.
#[derive(Debug, Clone)]
pub struct Fat32Bpb {
    /// Bytes per sector (usually 512, 1024, 2048, or 4096)
    pub bytes_per_sector: u16,
    /// Sectors per cluster (must be power of 2)
    pub sectors_per_cluster: u8,
    /// Number of reserved sectors (including boot sector)
    pub reserved_sectors: u16,
    /// Number of FAT tables (usually 2 for redundancy)
    pub num_fats: u8,
    /// Total sectors on the volume
    pub total_sectors: u32,
    /// Sectors per FAT table
    pub sectors_per_fat: u32,
    /// Cluster number of root directory (usually 2)
    pub root_cluster: u32,
    /// FSInfo sector number (usually 1, 0xFFFF if not present)
    pub fsinfo_sector: u16,
    /// Backup boot sector location (usually 6, 0xFFFF if not present)
    pub backup_boot_sector: u16,
    /// Volume label (11 characters, space-padded)
    pub volume_label: String,
    /// Filesystem type string (should be "FAT32   ")
    pub fs_type: String,
    /// Cached FSInfo data (if available)
    pub fsinfo: Option<FSInfo>,
}

impl Fat32Bpb {
    /// Parse BPB from boot sector data
    ///
    /// # Arguments
    ///
    /// * `boot_sector` - 512-byte boot sector data
    ///
    /// # Returns
    ///
    /// * `Ok(Fat32Bpb)` - Successfully parsed BPB
    /// * `Err(&str)` - Parse error (invalid signature, not FAT32, etc.)
    pub fn parse(boot_sector: &[u8]) -> Result<Self, &'static str> {
        if boot_sector.len() < 512 {
            return Err("Boot sector too small (need 512 bytes)");
        }

        // Check boot signature (0x55AA at offset 510)
        if boot_sector[510] != 0x55 || boot_sector[511] != 0xAA {
            return Err("Invalid boot signature (not 0x55AA)");
        }

        // Parse common BPB fields (FAT12/16/32)
        let bytes_per_sector = u16::from_le_bytes([boot_sector[11], boot_sector[12]]);
        let sectors_per_cluster = boot_sector[13];
        let reserved_sectors = u16::from_le_bytes([boot_sector[14], boot_sector[15]]);
        let num_fats = boot_sector[16];

        // Root directory entries (0 for FAT32)
        let root_entries = u16::from_le_bytes([boot_sector[17], boot_sector[18]]);
        if root_entries != 0 {
            return Err("Not FAT32 (root entries != 0)");
        }

        // Total sectors (16-bit, 0 for FAT32)
        let total_sectors_16 = u16::from_le_bytes([boot_sector[19], boot_sector[20]]);
        if total_sectors_16 != 0 {
            return Err("Not FAT32 (total sectors uses 16-bit field)");
        }

        // Sectors per FAT (16-bit, 0 for FAT32)
        let sectors_per_fat_16 = u16::from_le_bytes([boot_sector[22], boot_sector[23]]);
        if sectors_per_fat_16 != 0 {
            return Err("Not FAT32 (sectors per FAT uses 16-bit field)");
        }

        // FAT32-specific fields start at offset 36
        let total_sectors = u32::from_le_bytes([
            boot_sector[32],
            boot_sector[33],
            boot_sector[34],
            boot_sector[35],
        ]);

        let sectors_per_fat = u32::from_le_bytes([
            boot_sector[36],
            boot_sector[37],
            boot_sector[38],
            boot_sector[39],
        ]);

        let root_cluster = u32::from_le_bytes([
            boot_sector[44],
            boot_sector[45],
            boot_sector[46],
            boot_sector[47],
        ]);

        // FSInfo sector number at offset 48 (usually 1)
        let fsinfo_sector = u16::from_le_bytes([boot_sector[48], boot_sector[49]]);

        // Backup boot sector at offset 50 (usually 6)
        let backup_boot_sector = u16::from_le_bytes([boot_sector[50], boot_sector[51]]);

        // Volume label at offset 71 (11 bytes)
        let volume_label = core::str::from_utf8(&boot_sector[71..82])
            .unwrap_or("INVALID    ")
            .trim_end()
            .to_string();

        // Filesystem type at offset 82 (8 bytes, should be "FAT32   ")
        let fs_type = core::str::from_utf8(&boot_sector[82..90])
            .unwrap_or("UNKNOWN ")
            .trim_end()
            .to_string();

        // Validate it's actually FAT32
        if !fs_type.starts_with("FAT32") {
            return Err("Filesystem type is not FAT32");
        }

        // Validate geometry
        if bytes_per_sector == 0 || (bytes_per_sector & (bytes_per_sector - 1)) != 0 {
            return Err("Invalid bytes per sector (not power of 2)");
        }

        if sectors_per_cluster == 0 || (sectors_per_cluster & (sectors_per_cluster - 1)) != 0 {
            return Err("Invalid sectors per cluster (not power of 2)");
        }

        Ok(Fat32Bpb {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            total_sectors,
            sectors_per_fat,
            root_cluster,
            fsinfo_sector,
            backup_boot_sector,
            volume_label,
            fs_type,
            fsinfo: None, // Will be loaded separately
        })
    }

    /// Get the byte offset of the first FAT table
    pub fn fat_offset(&self) -> u64 {
        self.reserved_sectors as u64 * self.bytes_per_sector as u64
    }

    /// Get the byte offset of the data region (where clusters start)
    pub fn data_offset(&self) -> u64 {
        let fat_size = self.sectors_per_fat as u64 * self.bytes_per_sector as u64;
        self.fat_offset() + (self.num_fats as u64 * fat_size)
    }

    /// Get the sector number for a given cluster
    ///
    /// # Arguments
    ///
    /// * `cluster` - Cluster number (2 = first cluster in data region)
    ///
    /// # Returns
    ///
    /// Sector number of the first sector in the cluster
    pub fn cluster_to_sector(&self, cluster: u32) -> u64 {
        // Clusters start at 2 (0 and 1 are reserved)
        let data_sector = self.reserved_sectors as u64
            + (self.num_fats as u64 * self.sectors_per_fat as u64);
        data_sector + ((cluster as u64 - 2) * self.sectors_per_cluster as u64)
    }

    /// Get the byte offset for a given cluster
    pub fn cluster_to_offset(&self, cluster: u32) -> u64 {
        self.cluster_to_sector(cluster) * self.bytes_per_sector as u64
    }

    /// Get the size of one cluster in bytes
    pub fn cluster_size(&self) -> u32 {
        self.bytes_per_sector as u32 * self.sectors_per_cluster as u32
    }

    /// Get number of free clusters (if FSInfo is available)
    pub fn free_clusters(&self) -> Option<u32> {
        self.fsinfo.as_ref().map(|fs| {
            if fs.free_clusters == 0xFFFFFFFF {
                None
            } else {
                Some(fs.free_clusters)
            }
        }).flatten()
    }

    /// Get free space in bytes (if FSInfo is available)
    pub fn free_space(&self) -> Option<u64> {
        self.free_clusters().map(|count| count as u64 * self.cluster_size() as u64)
    }

    /// Read the boot sector from a block device and parse it
    ///
    /// This method will:
    /// 1. Try to read the primary boot sector (sector 0)
    /// 2. If that fails, try the backup boot sector (if specified)
    /// 3. Parse FSInfo (if present and valid)
    pub fn from_device(device: &dyn BlockDevice) -> Result<Self, &'static str> {
        // Try primary boot sector first
        let boot_sector_result = device.read_sector(0);

        let (boot_sector, used_backup) = match boot_sector_result {
            Ok(data) => {
                // Primary boot sector read successfully
                match Self::parse(&data) {
                    Ok(_bpb) => (data, false),
                    Err(primary_err) => {
                        // Primary boot sector is corrupted, try backup
                        // We need to at least get backup_boot_sector field (offset 50)
                        if data.len() >= 52 {
                            let backup_sector = u16::from_le_bytes([data[50], data[51]]);
                            if backup_sector != 0 && backup_sector != 0xFFFF {
                                // Try reading backup boot sector
                                match device.read_sector(backup_sector as u64) {
                                    Ok(backup_data) => {
                                        match Self::parse(&backup_data) {
                                            Ok(_) => {
                                                // Backup is valid!
                                                (backup_data, true)
                                            }
                                            Err(_) => {
                                                // Both primary and backup are corrupted
                                                return Err(primary_err);
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Can't read backup
                                        return Err(primary_err);
                                    }
                                }
                            } else {
                                // No backup sector specified
                                return Err(primary_err);
                            }
                        } else {
                            return Err(primary_err);
                        }
                    }
                }
            }
            Err(_) => {
                // Can't even read sector 0 - device error
                return Err("Failed to read boot sector");
            }
        };

        // Parse BPB
        let mut bpb = Self::parse(&boot_sector)?;

        // Note: If used_backup is true, the primary boot sector was corrupted
        // and we successfully fell back to the backup at bpb.backup_boot_sector.
        // In a production system, this should be logged for diagnostics.
        let _ = used_backup; // Suppress unused variable warning

        // Try to read FSInfo if present
        if bpb.fsinfo_sector != 0 && bpb.fsinfo_sector != 0xFFFF {
            match FSInfo::from_device(device, bpb.fsinfo_sector) {
                Ok(fsinfo) => {
                    bpb.fsinfo = Some(fsinfo);
                }
                Err(_) => {
                    // FSInfo is corrupted or not present, continue without it
                    // This is not fatal - we can still use the filesystem
                }
            }
        }

        Ok(bpb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpb_validation() {
        // Create minimal valid FAT32 boot sector
        let mut boot_sector = [0u8; 512];

        // Boot signature
        boot_sector[510] = 0x55;
        boot_sector[511] = 0xAA;

        // Bytes per sector = 512
        boot_sector[11] = 0x00;
        boot_sector[12] = 0x02;

        // Sectors per cluster = 8
        boot_sector[13] = 8;

        // Reserved sectors = 32
        boot_sector[14] = 32;
        boot_sector[15] = 0;

        // Number of FATs = 2
        boot_sector[16] = 2;

        // Root entries = 0 (FAT32)
        boot_sector[17] = 0;
        boot_sector[18] = 0;

        // Total sectors 16-bit = 0 (FAT32)
        boot_sector[19] = 0;
        boot_sector[20] = 0;

        // Sectors per FAT 16-bit = 0 (FAT32)
        boot_sector[22] = 0;
        boot_sector[23] = 0;

        // Total sectors 32-bit = 1000000
        let total = 1000000u32.to_le_bytes();
        boot_sector[32..36].copy_from_slice(&total);

        // Sectors per FAT 32-bit = 1000
        let spf = 1000u32.to_le_bytes();
        boot_sector[36..40].copy_from_slice(&spf);

        // Root cluster = 2
        boot_sector[44] = 2;
        boot_sector[45] = 0;
        boot_sector[46] = 0;
        boot_sector[47] = 0;

        // FSInfo sector = 1
        boot_sector[48] = 1;
        boot_sector[49] = 0;

        // Backup boot sector = 6
        boot_sector[50] = 6;
        boot_sector[51] = 0;

        // Filesystem type = "FAT32   "
        boot_sector[82..90].copy_from_slice(b"FAT32   ");

        let bpb = Fat32Bpb::parse(&boot_sector).unwrap();
        assert_eq!(bpb.bytes_per_sector, 512);
        assert_eq!(bpb.sectors_per_cluster, 8);
        assert_eq!(bpb.root_cluster, 2);
        assert_eq!(bpb.fsinfo_sector, 1);
        assert_eq!(bpb.backup_boot_sector, 6);
    }

    #[test]
    fn test_fsinfo_parsing() {
        let mut fsinfo = [0u8; 512];

        // Lead signature (0x41615252)
        fsinfo[0..4].copy_from_slice(&0x41615252u32.to_le_bytes());

        // Structure signature (0x61417272)
        fsinfo[484..488].copy_from_slice(&0x61417272u32.to_le_bytes());

        // Free clusters = 500000
        fsinfo[488..492].copy_from_slice(&500000u32.to_le_bytes());

        // Next free = 100
        fsinfo[492..496].copy_from_slice(&100u32.to_le_bytes());

        // Trail signature (0xAA550000)
        fsinfo[508..512].copy_from_slice(&0xAA550000u32.to_le_bytes());

        let fs = FSInfo::parse(&fsinfo).unwrap();
        assert_eq!(fs.free_clusters, 500000);
        assert_eq!(fs.next_free, 100);
    }
}
