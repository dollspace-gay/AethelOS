//! Virtual File System (VFS) Layer
//!
//! The VFS provides a unified interface for accessing different filesystem types.
//! This abstraction allows World-Tree and other components to work with any
//! underlying storage system (FAT32, ext4, NTFS, etc.) through a common API.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

pub mod manager;
pub mod mock;
pub mod block_device;
pub mod fat32;
pub mod ext4;
pub mod mock_fat32;
pub mod global;
pub mod debug_cmd;

#[cfg(test)]
mod tests;

/// Path type - represents a filesystem path
///
/// Internally stored as Unix-style paths (forward slashes) regardless of
/// the underlying filesystem's native format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    inner: String,
}

impl Path {
    /// Create a new path from a string
    ///
    /// Automatically normalizes backslashes to forward slashes.
    pub fn new(s: &str) -> Self {
        Self {
            inner: s.replace('\\', "/"),
        }
    }

    /// Get the path as a string slice
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Join this path with a component
    ///
    /// # Example
    /// ```
    /// let base = Path::new("/home/user");
    /// let full = base.join("documents");
    /// assert_eq!(full.as_str(), "/home/user/documents");
    /// ```
    pub fn join(&self, component: &str) -> Self {
        let base = self.inner.trim_end_matches('/');
        let comp = component.trim_start_matches('/');
        Self::new(&format!("{}/{}", base, comp))
    }

    /// Get the parent path
    ///
    /// Returns None if this is the root path.
    pub fn parent(&self) -> Option<Self> {
        let trimmed = self.inner.trim_end_matches('/');
        if trimmed.is_empty() || trimmed == "/" {
            return None;
        }

        trimmed.rfind('/')
            .map(|pos| Self::new(&trimmed[..pos]))
    }

    /// Get the file name (last component of path)
    pub fn file_name(&self) -> Option<&str> {
        let trimmed = self.inner.trim_end_matches('/');
        if trimmed.is_empty() || trimmed == "/" {
            return None;
        }

        trimmed.rfind('/')
            .map(|pos| &trimmed[pos + 1..])
            .or(Some(trimmed))
    }

    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.inner == "/" || self.inner.is_empty()
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Path {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

/// Directory entry information
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// File or directory name (not full path)
    pub name: String,
    /// Whether this entry is a directory
    pub is_dir: bool,
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileStat {
    /// File size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Creation time (Unix timestamp), if available
    pub created: Option<u64>,
    /// Last modification time (Unix timestamp), if available
    pub modified: Option<u64>,
}

/// Filesystem operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// File or directory not found
    NotFound,
    /// File or directory already exists
    AlreadyExists,
    /// Permission denied
    PermissionDenied,
    /// Path is not a directory
    NotADirectory,
    /// Path is a directory (when file was expected)
    IsADirectory,
    /// Invalid path format
    InvalidPath,
    /// I/O error occurred
    IoError,
    /// Filesystem is out of space
    OutOfSpace,
    /// Filesystem is mounted read-only
    ReadOnly,
    /// Operation not supported by this filesystem
    NotSupported,
}

impl core::fmt::Display for FsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FsError::NotFound => write!(f, "File or directory not found"),
            FsError::AlreadyExists => write!(f, "File or directory already exists"),
            FsError::PermissionDenied => write!(f, "Permission denied"),
            FsError::NotADirectory => write!(f, "Not a directory"),
            FsError::IsADirectory => write!(f, "Is a directory"),
            FsError::InvalidPath => write!(f, "Invalid path"),
            FsError::IoError => write!(f, "I/O error"),
            FsError::OutOfSpace => write!(f, "Filesystem out of space"),
            FsError::ReadOnly => write!(f, "Filesystem is read-only"),
            FsError::NotSupported => write!(f, "Operation not supported"),
        }
    }
}

/// Virtual File System trait
///
/// This trait provides a unified interface for different filesystem implementations.
/// All filesystems (FAT32, ext4, NTFS, etc.) must implement this trait.
///
/// # Thread Safety
///
/// Implementations must be Send + Sync as they may be accessed from multiple
/// threads (e.g., during file indexing or parallel operations).
///
/// # Example
///
/// ```
/// use vfs::{FileSystem, Path};
///
/// fn backup_file(fs: &dyn FileSystem, path: &Path) -> Result<(), FsError> {
///     let data = fs.read(path)?;
///     // ... save to backup location
///     Ok(())
/// }
/// ```
pub trait FileSystem: Send + Sync {
    /// Get the filesystem type name (for debugging/logging)
    ///
    /// Examples: "FAT32", "ext4", "NTFS", "MockFS"
    fn name(&self) -> &str;

    /// Read entire file into memory
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to read
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - File contents
    /// * `Err(FsError::NotFound)` - File doesn't exist
    /// * `Err(FsError::IsADirectory)` - Path is a directory
    /// * `Err(FsError::PermissionDenied)` - No read permission
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;

    /// Write entire file (create or overwrite)
    ///
    /// If the file exists, it will be overwritten. If it doesn't exist,
    /// it will be created along with any necessary parent directories.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to write
    /// * `data` - Data to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Write successful
    /// * `Err(FsError::ReadOnly)` - Filesystem is read-only
    /// * `Err(FsError::OutOfSpace)` - Not enough space
    /// * `Err(FsError::PermissionDenied)` - No write permission
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;

    /// Delete file or empty directory
    ///
    /// # Arguments
    ///
    /// * `path` - Path to delete
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Deletion successful
    /// * `Err(FsError::NotFound)` - File doesn't exist
    /// * `Err(FsError::ReadOnly)` - Filesystem is read-only
    /// * `Err(FsError::PermissionDenied)` - No delete permission
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn remove(&self, path: &Path) -> Result<(), FsError>;

    /// Create directory (including parent directories if needed)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to create
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Creation successful
    /// * `Err(FsError::AlreadyExists)` - Directory already exists
    /// * `Err(FsError::ReadOnly)` - Filesystem is read-only
    /// * `Err(FsError::PermissionDenied)` - No create permission
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn create_dir(&self, path: &Path) -> Result<(), FsError>;

    /// List directory contents
    ///
    /// # Arguments
    ///
    /// * `path` - Path to directory
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DirEntry>)` - List of entries in directory
    /// * `Err(FsError::NotFound)` - Directory doesn't exist
    /// * `Err(FsError::NotADirectory)` - Path is not a directory
    /// * `Err(FsError::PermissionDenied)` - No read permission
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError>;

    /// Get file/directory metadata
    ///
    /// # Arguments
    ///
    /// * `path` - Path to query
    ///
    /// # Returns
    ///
    /// * `Ok(FileStat)` - File metadata
    /// * `Err(FsError::NotFound)` - File doesn't exist
    /// * `Err(FsError::PermissionDenied)` - No access permission
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn stat(&self, path: &Path) -> Result<FileStat, FsError>;

    /// Check if path exists
    ///
    /// This is a convenience method that returns true if stat() succeeds.
    fn exists(&self, path: &Path) -> bool {
        self.stat(path).is_ok()
    }

    /// Sync all pending writes to disk
    ///
    /// Ensures all buffered writes are committed to persistent storage.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Sync successful
    /// * `Err(FsError::IoError)` - I/O error occurred
    fn sync(&self) -> Result<(), FsError>;
}
