//! Directory Entry Parsing
//!
//! FAT32 directories are sequences of 32-byte entries containing file metadata.
//!
//! Two types of entries:
//! 1. **Short entries (8.3)**: Standard DOS filenames like "README.TXT"
//! 2. **Long entries (LFN)**: Unicode filenames like "My Document.docx"
//!
//! Long filenames are stored as multiple LFN entries followed by a short entry.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Directory entry attributes (bitflags)
pub mod attr {
    pub const READ_ONLY: u8 = 0x01;
    pub const HIDDEN: u8 = 0x02;
    pub const SYSTEM: u8 = 0x04;
    pub const VOLUME_ID: u8 = 0x08;
    pub const DIRECTORY: u8 = 0x10;
    pub const ARCHIVE: u8 = 0x20;
    pub const LONG_NAME: u8 = 0x0F; // READ_ONLY | HIDDEN | SYSTEM | VOLUME_ID
}

/// A parsed directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Filename (short or long)
    pub name: String,
    /// Entry attributes
    pub attributes: u8,
    /// First cluster of file data
    pub first_cluster: u32,
    /// File size in bytes (0 for directories)
    pub size: u32,
    /// Is this a directory?
    pub is_dir: bool,
    /// Is this a hidden file?
    pub is_hidden: bool,
}

impl DirEntry {
    /// Parse a directory entry from 32 bytes
    ///
    /// # Arguments
    ///
    /// * `data` - 32-byte directory entry
    ///
    /// # Returns
    ///
    /// * `Some(DirEntry)` - Valid entry
    /// * `None` - Invalid, deleted, or end-of-directory marker
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }

        // Check for end of directory (first byte = 0x00)
        if data[0] == 0x00 {
            return None;
        }

        // Check for deleted entry (first byte = 0xE5)
        if data[0] == 0xE5 {
            return None;
        }

        let attributes = data[11];

        // Skip LFN entries (we'll handle them separately)
        if attributes == attr::LONG_NAME {
            return None;
        }

        // Skip volume ID entries
        if attributes & attr::VOLUME_ID != 0 {
            return None;
        }

        // Parse short filename (8.3 format)
        let name = Self::parse_short_name(&data[0..11]);

        // First cluster (high 16 bits at offset 20, low 16 bits at offset 26)
        let cluster_hi = u16::from_le_bytes([data[20], data[21]]) as u32;
        let cluster_lo = u16::from_le_bytes([data[26], data[27]]) as u32;
        let first_cluster = (cluster_hi << 16) | cluster_lo;

        // File size (32 bits at offset 28)
        let size = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);

        let is_dir = attributes & attr::DIRECTORY != 0;
        let is_hidden = attributes & attr::HIDDEN != 0;

        Some(DirEntry {
            name,
            attributes,
            first_cluster,
            size,
            is_dir,
            is_hidden,
        })
    }

    /// Parse an 8.3 short filename
    ///
    /// Format: "NAME    EXT" (8 chars for name, 3 for extension, space-padded)
    fn parse_short_name(name_bytes: &[u8]) -> String {
        // Extract base name (8 chars)
        let base = core::str::from_utf8(&name_bytes[0..8])
            .unwrap_or("????????")
            .trim_end();

        // Extract extension (3 chars)
        let ext = core::str::from_utf8(&name_bytes[8..11])
            .unwrap_or("???")
            .trim_end();

        if ext.is_empty() {
            base.to_string()
        } else {
            alloc::format!("{}.{}", base, ext)
        }
    }

    /// Parse a long filename (LFN) entry
    ///
    /// LFN entries are stored in reverse order before the short entry.
    /// Each LFN entry contains 13 Unicode characters.
    ///
    /// # Arguments
    ///
    /// * `data` - 32-byte LFN entry
    ///
    /// # Returns
    ///
    /// * `Some((sequence, chars))` - Sequence number and 13 characters
    /// * `None` - Not a valid LFN entry
    pub fn parse_lfn_entry(data: &[u8]) -> Option<(u8, String)> {
        if data.len() < 32 {
            return None;
        }

        let attributes = data[11];
        if attributes != attr::LONG_NAME {
            return None;
        }

        // Sequence number (0x40 bit means "last LFN entry")
        let sequence = data[0] & 0x3F;

        // Extract 13 Unicode characters (5 + 6 + 2)
        let mut chars = String::new();

        // Characters 1-5 at offset 1
        Self::append_lfn_chars(&mut chars, &data[1..11]);

        // Characters 6-11 at offset 14
        Self::append_lfn_chars(&mut chars, &data[14..26]);

        // Characters 12-13 at offset 28
        Self::append_lfn_chars(&mut chars, &data[28..32]);

        Some((sequence, chars))
    }

    /// Append LFN characters from a byte slice (UTF-16LE)
    fn append_lfn_chars(string: &mut String, data: &[u8]) {
        let mut i = 0;
        while i + 1 < data.len() {
            let ch = u16::from_le_bytes([data[i], data[i + 1]]);

            // Stop at null terminator or padding (0x0000 or 0xFFFF)
            if ch == 0x0000 || ch == 0xFFFF {
                break;
            }

            // Convert UTF-16 to char (simplified, doesn't handle surrogates)
            if let Some(c) = char::from_u32(ch as u32) {
                string.push(c);
            }

            i += 2;
        }
    }
}

/// Directory entry iterator
///
/// Parses a directory's cluster chain and yields directory entries.
pub struct DirEntryIter {
    data: Vec<u8>,
    offset: usize,
    lfn_buffer: Vec<String>, // Buffer for building long filenames
}

impl DirEntryIter {
    /// Create a new directory entry iterator
    ///
    /// # Arguments
    ///
    /// * `data` - Raw directory data (one or more clusters)
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            offset: 0,
            lfn_buffer: Vec::new(),
        }
    }

    /// Get the next directory entry
    pub fn next(&mut self) -> Option<DirEntry> {
        while self.offset + 32 <= self.data.len() {
            let entry_data = &self.data[self.offset..self.offset + 32];
            self.offset += 32;

            // Check for end of directory
            if entry_data[0] == 0x00 {
                return None;
            }

            // Try parsing as LFN entry
            if let Some((sequence, chars)) = DirEntry::parse_lfn_entry(entry_data) {
                // LFN entries are in reverse order, so insert at front
                if sequence == 1 {
                    // This is the last LFN entry, prepend to buffer
                    self.lfn_buffer.insert(0, chars);
                } else {
                    self.lfn_buffer.insert(0, chars);
                }
                continue;
            }

            // Try parsing as regular entry
            if let Some(mut entry) = DirEntry::parse(entry_data) {
                // If we have LFN entries buffered, use them as the name
                if !self.lfn_buffer.is_empty() {
                    entry.name = self.lfn_buffer.concat();
                    self.lfn_buffer.clear();
                }
                return Some(entry);
            }

            // Skip invalid/deleted entries
            self.lfn_buffer.clear();
        }

        None
    }

    /// Collect all entries into a Vec
    pub fn collect(mut self) -> Vec<DirEntry> {
        let mut entries = Vec::new();
        while let Some(entry) = self.next() {
            entries.push(entry);
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_name_parsing() {
        // "README  TXT" -> "README.TXT"
        let name = DirEntry::parse_short_name(b"README  TXT");
        assert_eq!(name, "README.TXT");

        // "KERNEL  BIN" -> "KERNEL.BIN"
        let name = DirEntry::parse_short_name(b"KERNEL  BIN");
        assert_eq!(name, "KERNEL.BIN");

        // "NOEXT       " -> "NOEXT"
        let name = DirEntry::parse_short_name(b"NOEXT      ");
        assert_eq!(name, "NOEXT");
    }

    #[test]
    fn test_attribute_checking() {
        let mut data = [0u8; 32];
        data[0] = b'T'; // Valid filename start
        data[11] = attr::DIRECTORY;
        data[26] = 2; // First cluster = 2

        let entry = DirEntry::parse(&data).unwrap();
        assert!(entry.is_dir);
        assert!(!entry.is_hidden);
    }
}
