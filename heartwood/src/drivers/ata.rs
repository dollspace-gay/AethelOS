//! ATA/IDE disk driver (PIO mode)
//!
//! This driver supports reading from ATA hard drives using PIO (Programmed I/O) mode.
//! It implements the BlockDevice trait for integration with the VFS layer.
//!
//! **Supported:**
//! - Primary bus (0x1F0-0x1F7)
//! - Master drive
//! - 28-bit LBA addressing
//! - Read operations
//!
//! **Not yet implemented:**
//! - Secondary bus (0x170)
//! - Slave drives
//! - 48-bit LBA
//! - DMA mode
//! - Write operations

use crate::vfs::block_device::{BlockDevice, BlockDeviceError};
use alloc::vec::Vec;
use core::arch::asm;

/// ATA command codes
const ATA_CMD_READ_SECTORS: u8 = 0x20;
const ATA_CMD_IDENTIFY: u8 = 0xEC;

/// ATA status register bits
const ATA_STATUS_BSY: u8 = 0x80;  // Busy
const ATA_STATUS_DRQ: u8 = 0x08;  // Data Request Ready
const ATA_STATUS_ERR: u8 = 0x01;  // Error

/// Primary ATA bus base I/O port
const ATA_PRIMARY_BASE: u16 = 0x1F0;

/// I/O port offsets from base
const ATA_REG_DATA: u16 = 0;       // 0x1F0
const ATA_REG_ERROR: u16 = 1;      // 0x1F1
const ATA_REG_SECTOR_COUNT: u16 = 2; // 0x1F2
const ATA_REG_LBA_LOW: u16 = 3;    // 0x1F3
const ATA_REG_LBA_MID: u16 = 4;    // 0x1F4
const ATA_REG_LBA_HIGH: u16 = 5;   // 0x1F5
const ATA_REG_DRIVE: u16 = 6;      // 0x1F6
const ATA_REG_STATUS: u16 = 7;     // 0x1F7
const ATA_REG_COMMAND: u16 = 7;    // 0x1F7 (write)

/// ATA disk drive
pub struct AtaDrive {
    bus: u16,
    drive: u8,        // 0 = master, 1 = slave
    sectors: u64,
    sector_size: u32,
}

impl AtaDrive {
    /// Get the total number of sectors on this drive
    pub fn sector_count(&self) -> u64 {
        self.sectors
    }

    /// Detect and initialize primary master drive (drive 0)
    /// Based on r3 kernel's ATA driver: https://github.com/Narasimha1997/r3
    pub fn detect_primary_master() -> Option<Self> {
        unsafe { Self::detect_drive(ATA_PRIMARY_BASE, 0) }
    }

    /// Detect and initialize primary slave drive (drive 1)
    pub fn detect_primary_slave() -> Option<Self> {
        unsafe { Self::detect_drive(ATA_PRIMARY_BASE, 1) }
    }

    /// Internal function to detect a drive on a specific bus and drive number
    ///
    /// SAFETY: Performs raw I/O port access to detect and initialize ATA drives.
    unsafe fn detect_drive(bus: u16, drive: u8) -> Option<Self> {
        // Serial marker: 'A' = Starting detection

        // Step 0: Disable interrupts on ATA controller
        // Write to Device Control Register (0x3F6): set nIEN bit (bit 1)
        outb(0x3F6, 0x02);  // Disable interrupts

        // Step 1: Select drive (0xA0 = master, 0xB0 = slave)
        let drive_select = 0xA0 | ((drive & 1) << 4);
        outb(bus + ATA_REG_DRIVE, drive_select);

            // Step 2: Wait 400ns for drive selection to settle
            for _ in 0..4 {
                inb(bus + ATA_REG_STATUS);
            }

            // Step 3: CRITICAL - Wait for BSY to clear BEFORE checking signature
            let mut timeout = 0;
            loop {
                let status = inb(bus + ATA_REG_STATUS);
                if (status & ATA_STATUS_BSY) == 0 {
                    break;  // BSY is clear, safe to proceed
                }
                timeout += 1;
                if timeout > 50000 {  // Reduced from 1000000 for faster timeout
                    return None;
                }
                core::hint::spin_loop();
            }

            // Step 4: Check device signature to determine ATA vs ATAPI
            // CRITICAL: Do this BEFORE sending any command!

            let lba_mid = inb(bus + ATA_REG_LBA_MID);
            let lba_high = inb(bus + ATA_REG_LBA_HIGH);

            // Debug: show signature bytes

            // ATAPI signature: 0x14 (LBA mid) / 0xEB (LBA high)
            if lba_mid == 0x14 && lba_high == 0xEB {
                return None;  // We don't support ATAPI yet
            }

            // ATA signature should be 0x00 / 0x00
            if lba_mid != 0x00 || lba_high != 0x00 {
                return None;
            }

            // Step 5: Send correct IDENTIFY command (0xEC for ATA)
            outb(bus + ATA_REG_COMMAND, ATA_CMD_IDENTIFY);


            // Step 6: Wait for drive to process command (BSY to clear, then DRQ to set)
            timeout = 0;
            loop {
                let status = inb(bus + ATA_REG_STATUS);

                // Check if status is 0 (no drive)
                if status == 0 {
                    return None;
                }

                // Check for error bit
                if (status & ATA_STATUS_ERR) != 0 {
                    let error = inb(bus + ATA_REG_ERROR);
                    return None;
                }

                // Check if BSY is clear AND DRQ is set (data ready)
                if (status & ATA_STATUS_BSY) == 0 && (status & ATA_STATUS_DRQ) != 0 {
                    break;
                }

                timeout += 1;
                if timeout > 50000 {  // Reduced from 1000000 for faster timeout
                    return None;
                }
                core::hint::spin_loop();
            }

            // Check if this is an ATA drive (not ATAPI)
            let lba_mid = inb(bus + ATA_REG_LBA_MID);
            let lba_high = inb(bus + ATA_REG_LBA_HIGH);
            if lba_mid != 0 || lba_high != 0 {
                return None;
            }

            // Step 7: Read IDENTIFY data (256 words = 512 bytes)

            // Read 256 words (512 bytes) of identify data
            let mut identify_data: [u16; 256] = [0; 256];
            for i in 0..256 {
                identify_data[i] = inw(bus + ATA_REG_DATA);
            }

            // Serial marker: 'H' = Parsing sector count

            // Parse sector count from words 60-61 (28-bit LBA)
            let sectors_low = identify_data[60] as u32;
            let sectors_high = identify_data[61] as u32;
            let sectors = ((sectors_high as u64) << 16) | (sectors_low as u64);

            // If sector count is 0, use a default
            let sectors = if sectors > 0 { sectors } else { 2048 };

        // Serial marker: 'Z' = Success!

        Some(AtaDrive {
            bus,
            drive,
            sectors,
            sector_size: 512,
        })
    }

    /// Send IDENTIFY command to drive
    fn identify(bus: u16, drive: u8) -> bool {
        // Select drive
        outb(bus + ATA_REG_DRIVE, 0xA0 | (drive << 4));
        Self::wait_400ns(bus);

        // Set sector count and LBA to 0
        outb(bus + ATA_REG_SECTOR_COUNT, 0);
        outb(bus + ATA_REG_LBA_LOW, 0);
        outb(bus + ATA_REG_LBA_MID, 0);
        outb(bus + ATA_REG_LBA_HIGH, 0);

        // Send IDENTIFY command
        outb(bus + ATA_REG_COMMAND, ATA_CMD_IDENTIFY);

        // Read status
        let status = inb(bus + ATA_REG_STATUS);
        if status == 0 {
            // Drive does not exist
            return false;
        }

        // Poll until BSY clears
        if !Self::wait_not_busy(bus) {
            return false;
        }

        // Check if drive is ready
        let lba_mid = inb(bus + ATA_REG_LBA_MID);
        let lba_high = inb(bus + ATA_REG_LBA_HIGH);

        if lba_mid != 0 || lba_high != 0 {
            // Not ATA drive (might be ATAPI)
            return false;
        }

        // Wait for DRQ or ERR (with timeout)
        for _ in 0..1000 {
            let status = inb(bus + ATA_REG_STATUS);
            if status & ATA_STATUS_ERR != 0 {
                return false;
            }
            if status & ATA_STATUS_DRQ != 0 {
                break;
            }
        }

        // Drive exists and responded to IDENTIFY
        // Discard the 256 words of IDENTIFY data for now
        for _ in 0..256 {
            let _ = inw(bus + ATA_REG_DATA);
        }

        true
    }

    /// Read sector count from IDENTIFY data
    fn read_sector_count(bus: u16, drive: u8) -> u64 {
        // Re-send IDENTIFY to get data
        outb(bus + ATA_REG_DRIVE, 0xA0 | (drive << 4));
        Self::wait_400ns(bus);
        outb(bus + ATA_REG_COMMAND, ATA_CMD_IDENTIFY);
        Self::wait_not_busy(bus);

        // Wait for DRQ (with timeout)
        let mut drq_ready = false;
        for _ in 0..1000 {
            let status = inb(bus + ATA_REG_STATUS);
            if status & ATA_STATUS_DRQ != 0 {
                drq_ready = true;
                break;
            }
        }

        if !drq_ready {
            // Timeout - return default
            return 2048; // 1MB default
        }

        // Read IDENTIFY data
        let mut identify_data = [0u16; 256];
        for i in 0..256 {
            identify_data[i] = inw(bus + ATA_REG_DATA);
        }

        // Words 60-61 contain total 28-bit LBA sectors
        let sectors_low = identify_data[60] as u32;
        let sectors_high = identify_data[61] as u32;
        let sectors = (sectors_high as u64) << 16 | sectors_low as u64;

        if sectors > 0 {
            sectors
        } else {
            // Default to small size if IDENTIFY failed
            2048 // 1MB
        }
    }

    /// Read a single sector (28-bit LBA)
    /// Read a single sector using PIO mode
    /// Based on r3 kernel's read_sectors_lba28()
    ///
    /// SAFETY: Performs raw I/O port access to read sectors from ATA drive.
    unsafe fn read_sector_pio(&self, lba: u64) -> Result<Vec<u8>, BlockDeviceError> {
        if lba >= self.sectors {
            return Err(BlockDeviceError::InvalidSector);
        }

        // Wait for drive to not be busy
        let mut timeout = 0;
            loop {
                let status = inb(self.bus + ATA_REG_STATUS);
                if status & ATA_STATUS_BSY == 0 {
                    break;
                }
                timeout += 1;
                if timeout > 50000 {  // Reduced from 1000000 for faster timeout
                    return Err(BlockDeviceError::IoError);
                }
            }

            // Select drive (LBA mode) + high 4 bits of LBA
            let drive_byte = 0xE0 | (self.drive << 4) | ((lba >> 24) & 0x0F) as u8;
            outb(self.bus + ATA_REG_DRIVE, drive_byte);

            // Wait 400ns
            for _ in 0..4 {
                inb(self.bus + ATA_REG_STATUS);
            }

            // Send sector count (1 sector)
            outb(self.bus + ATA_REG_SECTOR_COUNT, 1);

            // Send LBA (lower 24 bits)
            outb(self.bus + ATA_REG_LBA_LOW, (lba & 0xFF) as u8);
            outb(self.bus + ATA_REG_LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(self.bus + ATA_REG_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);

            // Send READ SECTORS command (0x20)
            outb(self.bus + ATA_REG_COMMAND, ATA_CMD_READ_SECTORS);

            // Wait for BSY to clear
            timeout = 0;
            loop {
                let status = inb(self.bus + ATA_REG_STATUS);
                if status & ATA_STATUS_BSY == 0 {
                    break;
                }
                timeout += 1;
                if timeout > 50000 {  // Reduced from 1000000 for faster timeout
                    return Err(BlockDeviceError::IoError);
                }
            }

            // Wait for DRQ to be set
            timeout = 0;
            loop {
                let status = inb(self.bus + ATA_REG_STATUS);
                if status & ATA_STATUS_ERR != 0 {
                    return Err(BlockDeviceError::IoError);
                }
                if status & ATA_STATUS_DRQ != 0 {
                    break;
                }
                timeout += 1;
                if timeout > 50000 {  // Reduced from 1000000 for faster timeout
                    return Err(BlockDeviceError::IoError);
                }
            }

            // Read 256 words (512 bytes) into fixed-size array
            let mut data: [u16; 256] = [0; 256];
            for i in 0..256 {
                data[i] = inw(self.bus + ATA_REG_DATA);
            }

            // Convert to Vec<u8>
        let mut buffer = Vec::with_capacity(512);
        for word in data.iter() {
            buffer.push((word & 0xFF) as u8);
            buffer.push((word >> 8) as u8);
        }

        Ok(buffer)
    }

    /// Wait for drive to not be busy (with timeout)
    fn wait_not_busy(bus: u16) -> bool {
        for _ in 0..1000 {  // Much shorter timeout
            let status = inb(bus + ATA_REG_STATUS);
            if status & ATA_STATUS_BSY == 0 {
                return true;
            }
            // No delay - inb() itself provides enough time
        }
        false
    }

    /// Wait 400ns by reading status register 4 times
    fn wait_400ns(bus: u16) {
        for _ in 0..4 {
            let _ = inb(bus + ATA_REG_STATUS);
        }
    }

    /// Tiny delay for polling loops
    fn tiny_delay() {
        for _ in 0..100 {
            core::hint::spin_loop();
        }
    }
}

impl BlockDevice for AtaDrive {
    fn sector_size(&self) -> u32 {
        self.sector_size
    }

    fn sector_count(&self) -> u64 {
        self.sectors
    }

    fn read_sector(&self, sector: u64) -> Result<Vec<u8>, BlockDeviceError> {
        unsafe { self.read_sector_pio(sector) }
    }

    fn read_sectors(&self, start_sector: u64, count: u32) -> Result<Vec<u8>, BlockDeviceError> {
        let mut result = Vec::with_capacity((count as usize) * 512);

        for i in 0..count {
            let sector_data = self.read_sector(start_sector + i as u64)?;
            result.extend_from_slice(&sector_data);
        }

        Ok(result)
    }

    fn write_sector(&self, _sector: u64, _data: &[u8]) -> Result<(), BlockDeviceError> {
        // Write not yet implemented
        Err(BlockDeviceError::WriteProtected)
    }

    fn write_sectors(&self, _start_sector: u64, _data: &[u8]) -> Result<(), BlockDeviceError> {
        // Write not yet implemented
        Err(BlockDeviceError::WriteProtected)
    }

    fn sync(&self) -> Result<(), BlockDeviceError> {
        // No cache to flush in PIO mode
        Ok(())
    }

    fn is_read_only(&self) -> bool {
        true  // For now, until write support is added
    }
}

/// Read byte from I/O port
fn inb(port: u16) -> u8 {
    let result: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") result,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    result
}

/// Write byte to I/O port
fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read word (16-bit) from I/O port
fn inw(port: u16) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "in ax, dx",
            out("ax") result,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    result
}
