//! VFS Manager - manages multiple mounted filesystems

use super::{FileSystem, Path, FsError};
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// VFS Manager error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    /// Filesystem already mounted at this mount point
    AlreadyMounted,
    /// Mount point not found
    NotMounted,
    /// Invalid mount point name
    InvalidMountPoint,
    /// No filesystem mounted at path
    NoFilesystem,
}

impl core::fmt::Display for VfsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VfsError::AlreadyMounted => write!(f, "Filesystem already mounted"),
            VfsError::NotMounted => write!(f, "Mount point not found"),
            VfsError::InvalidMountPoint => write!(f, "Invalid mount point name"),
            VfsError::NoFilesystem => write!(f, "No filesystem mounted at path"),
        }
    }
}

/// VFS Manager - manages multiple mounted filesystems
///
/// The VfsManager allows multiple filesystems to be mounted at different
/// mount points and routes operations to the appropriate filesystem.
///
/// # Example
///
/// ```
/// let mut vfs = VfsManager::new();
///
/// // Mount a FAT32 filesystem
/// vfs.mount("boot", Box::new(fat32_fs))?;
///
/// // Mount an ext4 filesystem
/// vfs.mount("root", Box::new(ext4_fs))?;
///
/// // Access files through mount points
/// let boot_fs = vfs.get("boot").unwrap();
/// let data = boot_fs.read(&Path::new("/kernel.bin"))?;
/// ```
pub struct VfsManager {
    /// Map of mount point names to filesystems
    filesystems: BTreeMap<String, Box<dyn FileSystem>>,
}

impl VfsManager {
    /// Create a new VFS manager
    pub fn new() -> Self {
        Self {
            filesystems: BTreeMap::new(),
        }
    }

    /// Mount a filesystem at a mount point
    ///
    /// # Arguments
    ///
    /// * `mount_point` - Name of the mount point (e.g., "boot", "root", "data")
    /// * `fs` - Filesystem to mount
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Mount successful
    /// * `Err(VfsError::AlreadyMounted)` - A filesystem is already mounted here
    /// * `Err(VfsError::InvalidMountPoint)` - Invalid mount point name
    ///
    /// # Example
    ///
    /// ```
    /// vfs.mount("boot", Box::new(fat32_fs))?;
    /// ```
    pub fn mount(&mut self, mount_point: &str, fs: Box<dyn FileSystem>) -> Result<(), VfsError> {
        // Validate mount point name
        if mount_point.is_empty() || mount_point.contains('/') || mount_point.contains('\\') {
            return Err(VfsError::InvalidMountPoint);
        }

        if self.filesystems.contains_key(mount_point) {
            return Err(VfsError::AlreadyMounted);
        }

        crate::println!("◈ Mounting {} at /{}", fs.name(), mount_point);
        self.filesystems.insert(mount_point.to_string(), fs);
        Ok(())
    }

    /// Unmount a filesystem
    ///
    /// # Arguments
    ///
    /// * `mount_point` - Name of the mount point to unmount
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Unmount successful
    /// * `Err(VfsError::NotMounted)` - No filesystem mounted at this point
    pub fn unmount(&mut self, mount_point: &str) -> Result<(), VfsError> {
        self.filesystems
            .remove(mount_point)
            .map(|fs| {
                crate::println!("◈ Unmounted {} from /{}", fs.name(), mount_point);
            })
            .ok_or(VfsError::NotMounted)
    }

    /// Get a reference to a mounted filesystem
    ///
    /// # Arguments
    ///
    /// * `mount_point` - Name of the mount point
    ///
    /// # Returns
    ///
    /// * `Some(&dyn FileSystem)` - Reference to the filesystem
    /// * `None` - No filesystem mounted at this point
    pub fn get(&self, mount_point: &str) -> Option<&dyn FileSystem> {
        self.filesystems.get(mount_point).map(|b| &**b)
    }

    /// List all mount points
    ///
    /// # Returns
    ///
    /// Vector of mount point names
    pub fn mounts(&self) -> Vec<&str> {
        self.filesystems.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of mounted filesystems
    pub fn count(&self) -> usize {
        self.filesystems.len()
    }

    /// Check if a mount point exists
    pub fn is_mounted(&self, mount_point: &str) -> bool {
        self.filesystems.contains_key(mount_point)
    }

    /// Resolve a path to (mount_point, filesystem, relative_path)
    ///
    /// This takes a path like "/boot/kernel.bin" and resolves it to:
    /// - mount_point: "boot"
    /// - filesystem: reference to the FAT32 filesystem
    /// - relative_path: "/kernel.bin" (or "kernel.bin")
    ///
    /// # Arguments
    ///
    /// * `path` - Path to resolve (should start with /)
    ///
    /// # Returns
    ///
    /// * `Some((mount_point, filesystem, relative_path))` - Resolution successful
    /// * `None` - No filesystem mounted for this path
    ///
    /// # Example
    ///
    /// ```
    /// let (mount, fs, rel_path) = vfs.resolve(&Path::new("/boot/kernel.bin")).unwrap();
    /// assert_eq!(mount, "boot");
    /// assert_eq!(rel_path.as_str(), "kernel.bin");
    /// ```
    pub fn resolve(&self, path: &Path) -> Option<(&str, &dyn FileSystem, Path)> {
        let path_str = path.as_str().trim_start_matches('/');

        // Handle root path
        if path_str.is_empty() {
            return None;
        }

        // Find the first path component (mount point)
        let mount_point_str = if let Some(slash_pos) = path_str.find('/') {
            &path_str[..slash_pos]
        } else {
            // Entire path is the mount point (e.g., "/boot")
            path_str
        };

        // Look up the filesystem - we iterate to get the actual key reference
        for (mount_key, fs) in self.filesystems.iter() {
            if mount_key.as_str() == mount_point_str {
                // Calculate relative path
                let relative = if mount_point_str.len() < path_str.len() {
                    &path_str[mount_point_str.len() + 1..]  // Skip mount point and slash
                } else {
                    ""  // Path was exactly the mount point
                };

                return Some((mount_key.as_str(), &**fs, Path::new(relative)));
            }
        }

        None
    }

    /// Unmount all filesystems and sync
    ///
    /// This is useful during shutdown to ensure all data is written.
    pub fn unmount_all(&mut self) -> Result<(), FsError> {
        crate::println!("◈ Unmounting all filesystems...");

        for (mount_point, fs) in self.filesystems.iter() {
            crate::println!("  Syncing {}...", mount_point);
            fs.sync()?;
        }

        let count = self.filesystems.len();
        self.filesystems.clear();

        crate::println!("  ✓ Unmounted {} filesystems", count);
        Ok(())
    }
}

impl Default for VfsManager {
    fn default() -> Self {
        Self::new()
    }
}

// Note: We don't use a global VFS singleton here.
// Instead, the kernel will create and manage the VFS instance.
// This makes testing easier and avoids global mutable state issues.
