//! VFS Tests

use super::*;
use super::manager::{VfsManager, VfsError};
use super::mock::MockFs;
use alloc::boxed::Box;

#[test]
fn test_path_creation() {
    let path = Path::new("/home/user/file.txt");
    assert_eq!(path.as_str(), "/home/user/file.txt");

    // Test backslash normalization
    let path = Path::new("C:\\Users\\file.txt");
    assert_eq!(path.as_str(), "C:/Users/file.txt");
}

#[test]
fn test_path_join() {
    let base = Path::new("/home/user");
    let joined = base.join("documents");
    assert_eq!(joined.as_str(), "/home/user/documents");

    // Test with trailing slash
    let base = Path::new("/home/user/");
    let joined = base.join("documents");
    assert_eq!(joined.as_str(), "/home/user/documents");

    // Test with leading slash in component
    let base = Path::new("/home/user");
    let joined = base.join("/documents");
    assert_eq!(joined.as_str(), "/home/user/documents");
}

#[test]
fn test_path_parent() {
    let path = Path::new("/home/user/file.txt");
    let parent = path.parent().unwrap();
    assert_eq!(parent.as_str(), "/home/user");

    let parent2 = parent.parent().unwrap();
    assert_eq!(parent2.as_str(), "/home");

    let parent3 = parent2.parent().unwrap();
    assert_eq!(parent3.as_str(), "");

    // Root has no parent
    let root = Path::new("/");
    assert!(root.parent().is_none());
}

#[test]
fn test_path_file_name() {
    let path = Path::new("/home/user/file.txt");
    assert_eq!(path.file_name(), Some("file.txt"));

    let path = Path::new("/home/user/");
    assert_eq!(path.file_name(), Some("user"));

    let path = Path::new("/");
    assert!(path.file_name().is_none());
}

#[test]
fn test_vfs_manager_creation() {
    let vfs = VfsManager::new();
    assert_eq!(vfs.count(), 0);
    assert!(vfs.mounts().is_empty());
}

#[test]
fn test_vfs_mount() {
    let mut vfs = VfsManager::new();

    let mock_fs = Box::new(MockFs::new());
    vfs.mount("test", mock_fs).unwrap();

    assert_eq!(vfs.count(), 1);
    assert!(vfs.is_mounted("test"));
    assert!(vfs.get("test").is_some());
}

#[test]
fn test_vfs_mount_duplicate() {
    let mut vfs = VfsManager::new();

    vfs.mount("test", Box::new(MockFs::new())).unwrap();

    let result = vfs.mount("test", Box::new(MockFs::new()));
    assert_eq!(result, Err(VfsError::AlreadyMounted));
}

#[test]
fn test_vfs_mount_invalid_name() {
    let mut vfs = VfsManager::new();

    // Empty name
    let result = vfs.mount("", Box::new(MockFs::new()));
    assert_eq!(result, Err(VfsError::InvalidMountPoint));

    // Name with slash
    let result = vfs.mount("test/bad", Box::new(MockFs::new()));
    assert_eq!(result, Err(VfsError::InvalidMountPoint));

    // Name with backslash
    let result = vfs.mount("test\\bad", Box::new(MockFs::new()));
    assert_eq!(result, Err(VfsError::InvalidMountPoint));
}

#[test]
fn test_vfs_unmount() {
    let mut vfs = VfsManager::new();

    vfs.mount("test", Box::new(MockFs::new())).unwrap();
    assert!(vfs.is_mounted("test"));

    vfs.unmount("test").unwrap();
    assert!(!vfs.is_mounted("test"));
    assert_eq!(vfs.count(), 0);
}

#[test]
fn test_vfs_unmount_not_mounted() {
    let mut vfs = VfsManager::new();

    let result = vfs.unmount("nonexistent");
    assert_eq!(result, Err(VfsError::NotMounted));
}

#[test]
fn test_vfs_resolve_simple() {
    let mut vfs = VfsManager::new();
    let fs = Box::new(MockFs::new());
    vfs.mount("boot", fs).unwrap();

    let (mount, _fs, rel_path) = vfs.resolve(&Path::new("/boot/kernel.bin")).unwrap();

    assert_eq!(mount, "boot");
    assert_eq!(rel_path.as_str(), "kernel.bin");
}

#[test]
fn test_vfs_resolve_nested() {
    let mut vfs = VfsManager::new();
    vfs.mount("root", Box::new(MockFs::new())).unwrap();

    let (mount, _fs, rel_path) = vfs.resolve(&Path::new("/root/home/user/file.txt")).unwrap();

    assert_eq!(mount, "root");
    assert_eq!(rel_path.as_str(), "home/user/file.txt");
}

#[test]
fn test_vfs_resolve_mount_point_only() {
    let mut vfs = VfsManager::new();
    vfs.mount("boot", Box::new(MockFs::new())).unwrap();

    let (mount, _fs, rel_path) = vfs.resolve(&Path::new("/boot")).unwrap();

    assert_eq!(mount, "boot");
    assert_eq!(rel_path.as_str(), "");
}

#[test]
fn test_vfs_resolve_not_mounted() {
    let vfs = VfsManager::new();

    let result = vfs.resolve(&Path::new("/nonexistent/file.txt"));
    assert!(result.is_none());
}

#[test]
fn test_vfs_file_operations_through_manager() {
    let mut vfs = VfsManager::new();
    let fs = Box::new(MockFs::new());
    vfs.mount("test", fs).unwrap();

    // Get filesystem reference
    let test_fs = vfs.get("test").unwrap();

    // Write a file
    test_fs.write(&Path::new("/hello.txt"), b"Hello, World!").unwrap();

    // Read it back
    let data = test_fs.read(&Path::new("/hello.txt")).unwrap();
    assert_eq!(data, b"Hello, World!");

    // Check it exists
    assert!(test_fs.exists(&Path::new("/hello.txt")));

    // Get stats
    let stat = test_fs.stat(&Path::new("/hello.txt")).unwrap();
    assert_eq!(stat.size, 13);
    assert!(!stat.is_dir);
}

#[test]
fn test_vfs_multiple_filesystems() {
    let mut vfs = VfsManager::new();

    vfs.mount("boot", Box::new(MockFs::new())).unwrap();
    vfs.mount("root", Box::new(MockFs::new())).unwrap();
    vfs.mount("data", Box::new(MockFs::new())).unwrap();

    assert_eq!(vfs.count(), 3);

    let mounts = vfs.mounts();
    assert!(mounts.contains(&"boot"));
    assert!(mounts.contains(&"root"));
    assert!(mounts.contains(&"data"));

    // Write to different filesystems
    vfs.get("boot").unwrap().write(&Path::new("/kernel.bin"), b"boot").unwrap();
    vfs.get("root").unwrap().write(&Path::new("/etc/config"), b"root").unwrap();
    vfs.get("data").unwrap().write(&Path::new("/file.txt"), b"data").unwrap();

    // Verify each filesystem has its own data
    let boot_data = vfs.get("boot").unwrap().read(&Path::new("/kernel.bin")).unwrap();
    assert_eq!(boot_data, b"boot");

    let root_data = vfs.get("root").unwrap().read(&Path::new("/etc/config")).unwrap();
    assert_eq!(root_data, b"root");

    let data_data = vfs.get("data").unwrap().read(&Path::new("/file.txt")).unwrap();
    assert_eq!(data_data, b"data");
}

#[test]
fn test_vfs_unmount_all() {
    let mut vfs = VfsManager::new();

    vfs.mount("boot", Box::new(MockFs::new())).unwrap();
    vfs.mount("root", Box::new(MockFs::new())).unwrap();
    vfs.mount("data", Box::new(MockFs::new())).unwrap();

    assert_eq!(vfs.count(), 3);

    vfs.unmount_all().unwrap();

    assert_eq!(vfs.count(), 0);
    assert!(vfs.mounts().is_empty());
}
