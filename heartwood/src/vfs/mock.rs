//! Mock Filesystem - In-memory filesystem for testing

use super::{FileSystem, Path, FsError, DirEntry, FileStat};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use core::cell::RefCell;

/// Mock filesystem implementation
///
/// This is an in-memory filesystem used for testing. It stores files as
/// key-value pairs in a HashMap and supports all VFS operations.
///
/// # Thread Safety
///
/// Uses RefCell for interior mutability to satisfy the FileSystem trait's
/// const methods while allowing modification. This is safe because the kernel
/// is single-threaded in test mode.
///
/// # Example
///
/// ```
/// let fs = MockFs::new();
/// fs.write(&Path::new("/test.txt"), b"Hello, World!")?;
/// let data = fs.read(&Path::new("/test.txt"))?;
/// assert_eq!(data, b"Hello, World!");
/// ```
pub struct MockFs {
    /// Files stored as path -> data
    files: RefCell<BTreeMap<String, Vec<u8>>>,
    /// Directories stored as path -> ()
    dirs: RefCell<BTreeMap<String, ()>>,
    /// Whether this filesystem is read-only
    read_only: bool,
}

impl MockFs {
    /// Create a new empty mock filesystem
    pub fn new() -> Self {
        let fs = Self {
            files: RefCell::new(BTreeMap::new()),
            dirs: RefCell::new(BTreeMap::new()),
            read_only: false,
        };

        // Create root directory
        fs.dirs.borrow_mut().insert("/".to_string(), ());

        fs
    }

    /// Create a read-only mock filesystem
    ///
    /// All write operations will return FsError::ReadOnly
    pub fn new_read_only() -> Self {
        let mut fs = Self::new();
        fs.read_only = true;
        fs
    }

    /// Check if filesystem is read-only
    fn check_read_only(&self) -> Result<(), FsError> {
        if self.read_only {
            Err(FsError::ReadOnly)
        } else {
            Ok(())
        }
    }

    /// Normalize path (remove trailing slashes, handle empty paths)
    fn normalize_path(&self, path: &Path) -> String {
        let s = path.as_str();
        if s.is_empty() || s == "/" {
            "/".to_string()
        } else {
            s.trim_end_matches('/').to_string()
        }
    }

    /// Ensure parent directories exist
    fn ensure_parent_dirs(&self, path: &Path) -> Result<(), FsError> {
        if let Some(parent) = path.parent() {
            let parent_str = self.normalize_path(&parent);
            if parent_str != "/" && !self.dirs.borrow().contains_key(&parent_str) {
                // Recursively create parent
                self.ensure_parent_dirs(&parent)?;
                self.dirs.borrow_mut().insert(parent_str, ());
            }
        }
        Ok(())
    }
}

impl Default for MockFs {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: MockFs is only used in tests, which are single-threaded.
// RefCell is not Sync, but this is acceptable for testing purposes.
// In production, real filesystems will use proper thread-safe primitives.
unsafe impl Sync for MockFs {}

impl FileSystem for MockFs {
    fn name(&self) -> &str {
        "MockFS"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let path_str = self.normalize_path(path);

        // Check if it's a directory
        if self.dirs.borrow().contains_key(&path_str) {
            return Err(FsError::IsADirectory);
        }

        // Try to read file
        self.files
            .borrow()
            .get(&path_str)
            .cloned()
            .ok_or(FsError::NotFound)
    }

    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.check_read_only()?;

        let path_str = self.normalize_path(path);

        // Check if path is a directory
        if self.dirs.borrow().contains_key(&path_str) {
            return Err(FsError::IsADirectory);
        }

        // Ensure parent directories exist
        self.ensure_parent_dirs(path)?;

        // Write file
        self.files
            .borrow_mut()
            .insert(path_str, data.to_vec());

        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<(), FsError> {
        self.check_read_only()?;

        let path_str = self.normalize_path(path);

        // Try to remove file first
        if self.files.borrow_mut().remove(&path_str).is_some() {
            return Ok(());
        }

        // Try to remove directory
        if path_str == "/" {
            return Err(FsError::PermissionDenied);  // Can't delete root
        }

        if self.dirs.borrow_mut().remove(&path_str).is_some() {
            return Ok(());
        }

        Err(FsError::NotFound)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        self.check_read_only()?;

        let path_str = self.normalize_path(path);

        // Check if already exists
        if self.dirs.borrow().contains_key(&path_str) {
            return Err(FsError::AlreadyExists);
        }

        if self.files.borrow().contains_key(&path_str) {
            return Err(FsError::AlreadyExists);
        }

        // Ensure parent directories exist
        self.ensure_parent_dirs(path)?;

        // Create directory
        self.dirs.borrow_mut().insert(path_str, ());

        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError> {
        let path_str = self.normalize_path(path);

        // Check if path is a directory
        if !self.dirs.borrow().contains_key(&path_str) {
            if self.files.borrow().contains_key(&path_str) {
                return Err(FsError::NotADirectory);
            } else {
                return Err(FsError::NotFound);
            }
        }

        let mut entries = Vec::new();
        let prefix = if path_str == "/" {
            "/".to_string()
        } else {
            format!("{}/", path_str)
        };

        // Find all files in this directory
        for key in self.files.borrow().keys() {
            if key.starts_with(&prefix) {
                let name = &key[prefix.len()..];
                // Only include direct children (not subdirectories)
                if !name.contains('/') && !name.is_empty() {
                    entries.push(DirEntry {
                        name: name.to_string(),
                        is_dir: false,
                    });
                }
            }
        }

        // Find all subdirectories
        for key in self.dirs.borrow().keys() {
            if key != &path_str && key.starts_with(&prefix) {
                let name = &key[prefix.len()..];
                // Only include direct children
                if !name.contains('/') && !name.is_empty() {
                    entries.push(DirEntry {
                        name: name.to_string(),
                        is_dir: true,
                    });
                }
            }
        }

        Ok(entries)
    }

    fn stat(&self, path: &Path) -> Result<FileStat, FsError> {
        let path_str = self.normalize_path(path);

        // Check if it's a file
        if let Some(data) = self.files.borrow().get(&path_str) {
            return Ok(FileStat {
                size: data.len() as u64,
                is_dir: false,
                created: None,
                modified: None,
            });
        }

        // Check if it's a directory
        if self.dirs.borrow().contains_key(&path_str) {
            return Ok(FileStat {
                size: 0,
                is_dir: true,
                created: None,
                modified: None,
            });
        }

        Err(FsError::NotFound)
    }

    fn sync(&self) -> Result<(), FsError> {
        // MockFS is in-memory, so sync is a no-op
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mockfs() {
        let fs = MockFs::new();
        assert_eq!(fs.name(), "MockFS");

        // Root should exist
        assert!(fs.exists(&Path::new("/")));
    }

    #[test]
    fn test_write_and_read() {
        let fs = MockFs::new();

        let path = Path::new("/test.txt");
        let data = b"Hello, World!";

        fs.write(&path, data).unwrap();
        let read_data = fs.read(&path).unwrap();

        assert_eq!(read_data, data);
    }

    #[test]
    fn test_read_nonexistent() {
        let fs = MockFs::new();
        let path = Path::new("/nonexistent.txt");

        let result = fs.read(&path);
        assert_eq!(result, Err(FsError::NotFound));
    }

    #[test]
    fn test_create_nested_file() {
        let fs = MockFs::new();

        let path = Path::new("/dir1/dir2/file.txt");
        let data = b"Nested file";

        fs.write(&path, data).unwrap();
        let read_data = fs.read(&path).unwrap();

        assert_eq!(read_data, data);

        // Parent directories should exist
        assert!(fs.exists(&Path::new("/dir1")));
        assert!(fs.exists(&Path::new("/dir1/dir2")));
    }

    #[test]
    fn test_read_dir() {
        let fs = MockFs::new();

        fs.write(&Path::new("/file1.txt"), b"data1").unwrap();
        fs.write(&Path::new("/file2.txt"), b"data2").unwrap();
        fs.create_dir(&Path::new("/subdir")).unwrap();

        let entries = fs.read_dir(&Path::new("/")).unwrap();

        assert_eq!(entries.len(), 3);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"file1.txt"));
        assert!(names.contains(&"file2.txt"));
        assert!(names.contains(&"subdir"));
    }

    #[test]
    fn test_remove_file() {
        let fs = MockFs::new();

        let path = Path::new("/test.txt");
        fs.write(&path, b"data").unwrap();

        assert!(fs.exists(&path));

        fs.remove(&path).unwrap();

        assert!(!fs.exists(&path));
    }

    #[test]
    fn test_read_only_filesystem() {
        let fs = MockFs::new_read_only();

        let result = fs.write(&Path::new("/test.txt"), b"data");
        assert_eq!(result, Err(FsError::ReadOnly));
    }

    #[test]
    fn test_stat() {
        let fs = MockFs::new();

        fs.write(&Path::new("/file.txt"), b"12345").unwrap();
        fs.create_dir(&Path::new("/dir")).unwrap();

        let file_stat = fs.stat(&Path::new("/file.txt")).unwrap();
        assert_eq!(file_stat.size, 5);
        assert!(!file_stat.is_dir);

        let dir_stat = fs.stat(&Path::new("/dir")).unwrap();
        assert_eq!(dir_stat.size, 0);
        assert!(dir_stat.is_dir);
    }
}
