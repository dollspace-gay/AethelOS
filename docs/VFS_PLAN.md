# The Path-Keepers: AethelOS Virtual File System Architecture

**Status:** Planned
**Priority:** High
**Dependencies:** Block device driver, World-Tree design
**Estimated Timeline:** 4-6 weeks
**Version:** 1.0
**Last Updated:** January 2025

---

## Table of Contents

1. [Vision](#vision)
2. [The Fundamental Insight](#the-fundamental-insight)
3. [Architecture Overview](#architecture-overview)
4. [VFS Abstraction Layer](#vfs-abstraction-layer)
5. [Filesystem Implementations](#filesystem-implementations)
6. [Integration with World-Tree](#integration-with-world-tree)
7. [Implementation Phases](#implementation-phases)
8. [Multi-Boot Support](#multi-boot-support)
9. [Testing Strategy](#testing-strategy)
10. [Timeline and Milestones](#timeline-and-milestones)

---

## Vision

> *"The World-Tree does not care whether its roots touch ext4, FAT32, or NTFS. It draws nourishment from all soils, transforming base storage into living knowledge."*

### The Problem

World-Tree needs persistent storage for:
- Content-addressed objects (Git-like blobs)
- Metadata indices (queryable attributes)
- Commit graphs (version history)
- Pruning logs (space management)

**Bad approach:** Build a custom filesystem from scratch
- 6-12 months of development
- High risk of data corruption bugs
- Reinventing solved problems
- No interoperability with existing systems

**Good approach:** Abstract storage behind a clean interface
- Support multiple proven filesystems
- Reuse battle-tested code
- Enable multi-boot configurations
- Focus on World-Tree's unique features

### The Philosophy

AethelOS embraces **pragmatic abstraction**:
- Use existing filesystems as storage layers
- Build unique features on top (queries, versioning, metadata)
- Enable interoperability (install on Linux/Windows partitions)
- Keep options open (can add custom FS later if needed)

**Git proves this works:** Git doesn't have its own filesystem. It works brilliantly on ext4, NTFS, APFS, FAT32, and even network filesystems. Git's innovation is its *model* (content-addressing, versioning), not its storage layer.

**AethelOS follows the same pattern:** World-Tree's innovation is *queries and metadata*, not block allocation.

---

## The Fundamental Insight

### What Makes World-Tree Unique (Build This)

- ✅ **Query-based interface** - `seek scrolls where creator is "Elara"`
- ✅ **Rich metadata** - Essence, creator, genesis time, connections
- ✅ **Content versioning** - Every version preserved, queryable by time
- ✅ **Temporal queries** - See file as it existed 3 days ago
- ✅ **Pruning policies** - Intelligent space management
- ✅ **No path hierarchy** - Files are database objects

### What's Commodity (Reuse This)

- ❌ Block allocation algorithms
- ❌ Directory structures and B-trees
- ❌ Journal recovery and crash consistency
- ❌ Disk I/O optimization
- ❌ File permissions and ownership
- ❌ Inode management

**ALL of World-Tree's unique features work on top of ANY filesystem.**

---

## Architecture Overview

### The Stack

```
┌─────────────────────────────────────────────────────┐
│  Eldarin Shell / User Applications                  │
│  > seek scrolls where essence="Scroll"              │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  World-Tree Grove (Query & Versioning Layer)        │
│  - Query language parser and executor               │
│  - Metadata index (SQLite or custom)                │
│  - Version graph management                         │
│  - Pruning policy engine                            │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  Object Store (Git-like Content Addressing)         │
│  - SHA-256 content addressing                       │
│  - Blob storage and deduplication                   │
│  - Reference management                             │
│  - Commit graph                                     │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  VFS Layer (Filesystem Abstraction) ← YOU BUILD THIS│
│  trait FileSystem {                                 │
│    fn read(path) -> Vec<u8>;                        │
│    fn write(path, data);                            │
│    fn list_dir(path) -> Vec<Entry>;                 │
│  }                                                  │
└─────────────────────────────────────────────────────┘
                       ↓
┌──────────────┬──────────────┬──────────────┬────────┐
│  FAT32       │  ext4        │  NTFS        │ Custom │
│  Driver      │  Driver      │  Driver      │  (future)|
│  (Week 1)    │  (Week 3)    │  (Week 5)    │        │
└──────────────┴──────────────┴──────────────┴────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  Block Device Layer                                 │
│  - Read/write sectors                               │
│  - Partition table parsing                          │
│  - IDE/SATA/NVMe drivers                            │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  Physical Disk Hardware                             │
└─────────────────────────────────────────────────────┘
```

### Key Insight

**World-Tree doesn't call FAT32/ext4 directly.** It calls the VFS layer, which dispatches to the appropriate driver. This means:

- ✅ World-Tree code is filesystem-agnostic
- ✅ Easy to add new filesystem support
- ✅ Can test with mock filesystems
- ✅ Can switch filesystems at runtime
- ✅ Can use multiple filesystems simultaneously

---

## VFS Abstraction Layer

### Core Trait

```rust
// heartwood/src/vfs/mod.rs

use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;

/// Virtual File System trait - the abstraction that makes everything work
pub trait FileSystem: Send + Sync {
    /// Filesystem type name (for debugging/logging)
    fn name(&self) -> &str;

    /// Read entire file into memory
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;

    /// Write entire file (create or overwrite)
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;

    /// Delete file or empty directory
    fn remove(&self, path: &Path) -> Result<(), FsError>;

    /// Create directory (and parents if needed)
    fn create_dir(&self, path: &Path) -> Result<(), FsError>;

    /// List directory contents
    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError>;

    /// Get file metadata
    fn stat(&self, path: &Path) -> Result<FileStat, FsError>;

    /// Check if path exists
    fn exists(&self, path: &Path) -> bool {
        self.stat(path).is_ok()
    }

    /// Sync all pending writes to disk
    fn sync(&self) -> Result<(), FsError>;
}

/// Path type (Unix-style, even on filesystems that use backslashes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    inner: String,
}

impl Path {
    pub fn new(s: &str) -> Self {
        // Normalize to forward slashes
        Self { inner: s.replace('\\', "/") }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn join(&self, component: &str) -> Self {
        Self::new(&format!("{}/{}", self.inner.trim_end_matches('/'), component))
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileStat {
    pub size: u64,
    pub is_dir: bool,
    pub created: Option<u64>,  // Unix timestamp
    pub modified: Option<u64>,
}

/// Filesystem errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    PermissionDenied,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    IoError,
    OutOfSpace,
    ReadOnly,
}
```

### VFS Manager

```rust
// heartwood/src/vfs/manager.rs

use alloc::collections::BTreeMap;
use alloc::boxed::Box;

/// Manages multiple mounted filesystems
pub struct VfsManager {
    filesystems: BTreeMap<String, Box<dyn FileSystem>>,
}

impl VfsManager {
    pub fn new() -> Self {
        Self {
            filesystems: BTreeMap::new(),
        }
    }

    /// Mount a filesystem at a mount point
    pub fn mount(&mut self, mount_point: &str, fs: Box<dyn FileSystem>) -> Result<(), VfsError> {
        if self.filesystems.contains_key(mount_point) {
            return Err(VfsError::AlreadyMounted);
        }

        crate::println!("◈ Mounting {} at /{}", fs.name(), mount_point);
        self.filesystems.insert(mount_point.to_string(), fs);
        Ok(())
    }

    /// Unmount a filesystem
    pub fn unmount(&mut self, mount_point: &str) -> Result<(), VfsError> {
        self.filesystems.remove(mount_point)
            .map(|_| ())
            .ok_or(VfsError::NotMounted)
    }

    /// Get filesystem by mount point
    pub fn get(&self, mount_point: &str) -> Option<&dyn FileSystem> {
        self.filesystems.get(mount_point).map(|b| &**b)
    }

    /// Get mutable filesystem reference
    pub fn get_mut(&mut self, mount_point: &str) -> Option<&mut dyn FileSystem> {
        self.filesystems.get_mut(mount_point).map(|b| &mut **b)
    }

    /// List all mount points
    pub fn mounts(&self) -> Vec<&str> {
        self.filesystems.keys().map(|s| s.as_str()).collect()
    }

    /// Resolve a path to (mount_point, filesystem, relative_path)
    pub fn resolve(&self, path: &Path) -> Option<(&str, &dyn FileSystem, Path)> {
        let path_str = path.as_str().trim_start_matches('/');

        // Find longest matching mount point
        for (mount_point, fs) in self.filesystems.iter().rev() {
            if path_str.starts_with(mount_point) {
                let relative = path_str.strip_prefix(mount_point)
                    .unwrap_or("")
                    .trim_start_matches('/');
                return Some((mount_point, &**fs, Path::new(relative)));
            }
        }

        None
    }
}

/// Global VFS instance (initialized during boot)
static VFS: InterruptSafeLock<Option<VfsManager>> = InterruptSafeLock::new(None);

pub fn init() {
    *VFS.lock() = Some(VfsManager::new());
    crate::println!("◈ VFS layer initialized");
}

pub fn vfs() -> &'static InterruptSafeLock<Option<VfsManager>> {
    &VFS
}
```

---

## Filesystem Implementations

### 1. FAT32 - The Foundation (Week 1)

**Why start here:**
- ✅ Simple specification (easy to understand)
- ✅ Universal compatibility (every OS can read it)
- ✅ Pure Rust crate available (`fatfs`)
- ✅ Good for boot media (USB sticks)
- ✅ No complex features (no journaling, permissions, etc.)

**Implementation:**

```rust
// heartwood/src/vfs/fat32.rs

use fatfs::{FileSystem as FatFileSystem, FsOptions};

pub struct Fat32 {
    fs: FatFileSystem<BlockDevice>,
    read_only: bool,
}

impl Fat32 {
    pub fn new(device: BlockDevice) -> Result<Self, FsError> {
        let fs = FatFileSystem::new(device, FsOptions::new())
            .map_err(|_| FsError::IoError)?;

        Ok(Self {
            fs,
            read_only: false,
        })
    }
}

impl FileSystem for Fat32 {
    fn name(&self) -> &str {
        "FAT32"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let root_dir = self.fs.root_dir();
        let mut file = root_dir.open_file(path.as_str())
            .map_err(|_| FsError::NotFound)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|_| FsError::IoError)?;

        Ok(buf)
    }

    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        if self.read_only {
            return Err(FsError::ReadOnly);
        }

        let root_dir = self.fs.root_dir();
        let mut file = root_dir.create_file(path.as_str())
            .map_err(|_| FsError::IoError)?;

        file.write_all(data)
            .map_err(|_| FsError::IoError)?;

        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError> {
        let root_dir = self.fs.root_dir();
        let dir = if path.as_str().is_empty() || path.as_str() == "/" {
            root_dir
        } else {
            root_dir.open_dir(path.as_str())
                .map_err(|_| FsError::NotFound)?
        };

        let mut entries = Vec::new();
        for entry in dir.iter() {
            let entry = entry.map_err(|_| FsError::IoError)?;
            entries.push(DirEntry {
                name: entry.file_name(),
                is_dir: entry.is_dir(),
            });
        }

        Ok(entries)
    }

    fn stat(&self, path: &Path) -> Result<FileStat, FsError> {
        let root_dir = self.fs.root_dir();
        let file = root_dir.open_file(path.as_str())
            .map_err(|_| FsError::NotFound)?;

        Ok(FileStat {
            size: file.len(),
            is_dir: false,
            created: None,  // FAT32 has timestamps but we'll skip for now
            modified: None,
        })
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        if self.read_only {
            return Err(FsError::ReadOnly);
        }

        let root_dir = self.fs.root_dir();
        root_dir.create_dir(path.as_str())
            .map_err(|_| FsError::IoError)?;

        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<(), FsError> {
        if self.read_only {
            return Err(FsError::ReadOnly);
        }

        let root_dir = self.fs.root_dir();
        root_dir.remove(path.as_str())
            .map_err(|_| FsError::NotFound)?;

        Ok(())
    }

    fn sync(&self) -> Result<(), FsError> {
        // FAT32 crate handles sync automatically
        Ok(())
    }
}
```

**Dependencies:**
```toml
[dependencies]
fatfs = { version = "0.4", default-features = false }
```

**Time estimate:** 1 week (including block device integration)

---

### 2. ext4 - Linux Compatibility (Week 3-4)

**Why ext4:**
- ✅ Most common Linux filesystem
- ✅ Journaling (crash-safe)
- ✅ Good performance
- ✅ Can install AethelOS on existing Linux partitions

**Implementation options:**

**Option A: Port lwext4 (Recommended)**
- Small C library (~15,000 lines)
- BSD license
- Supports ext2/ext3/ext4
- Well-tested

**Option B: Use ext4-rs (If mature enough)**
- Pure Rust implementation
- Still in development
- May not support all ext4 features yet

**Skeleton:**

```rust
// heartwood/src/vfs/ext4.rs

// Using lwext4 via FFI (to be implemented)
pub struct Ext4 {
    // lwext4 file descriptor
    fs: ext4_sys::Ext4FileSystem,
}

impl FileSystem for Ext4 {
    fn name(&self) -> &str {
        "ext4"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        // Call lwext4 functions via FFI
        todo!("Implement ext4 read")
    }

    // ... similar pattern to FAT32
}
```

**Time estimate:** 2-4 weeks (including lwext4 integration and testing)

---

### 3. NTFS - Windows Interop (Week 5-6)

**Why NTFS:**
- ✅ Read Windows partitions
- ✅ Share data with Windows dual-boot
- ✅ Access Documents, Pictures, etc. from AethelOS

**Implementation:**

```rust
// heartwood/src/vfs/ntfs.rs

use ntfs::{Ntfs, NtfsFile};

pub struct NtfsFs {
    ntfs: Ntfs,
    read_only: bool,  // Write support is complex, start read-only
}

impl FileSystem for NtfsFs {
    fn name(&self) -> &str {
        "NTFS"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        // Use ntfs crate to read file
        todo!("Implement NTFS read")
    }

    fn write(&self, _path: &Path, _data: &[u8]) -> Result<(), FsError> {
        // Read-only for now
        Err(FsError::ReadOnly)
    }

    // ... implement other methods
}
```

**Dependencies:**
```toml
[dependencies]
ntfs = { version = "0.4", default-features = false }
```

**Time estimate:** 1 week for read-only, 4+ weeks for write support

---

### 4. Mock Filesystem - Testing (Immediate)

```rust
// heartwood/src/vfs/mock.rs

use alloc::collections::BTreeMap;

/// In-memory filesystem for testing
pub struct MockFs {
    files: BTreeMap<String, Vec<u8>>,
    dirs: BTreeMap<String, ()>,
}

impl MockFs {
    pub fn new() -> Self {
        let mut fs = Self {
            files: BTreeMap::new(),
            dirs: BTreeMap::new(),
        };
        fs.dirs.insert("/".to_string(), ());
        fs
    }
}

impl FileSystem for MockFs {
    fn name(&self) -> &str {
        "MockFS"
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.files.get(path.as_str())
            .cloned()
            .ok_or(FsError::NotFound)
    }

    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.files.insert(path.as_str().to_string(), data.to_vec());
        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError> {
        let prefix = format!("{}/", path.as_str().trim_end_matches('/'));
        let mut entries = Vec::new();

        for key in self.files.keys() {
            if key.starts_with(&prefix) {
                let name = key.strip_prefix(&prefix).unwrap();
                if !name.contains('/') {
                    entries.push(DirEntry {
                        name: name.to_string(),
                        is_dir: false,
                    });
                }
            }
        }

        Ok(entries)
    }

    fn stat(&self, path: &Path) -> Result<FileStat, FsError> {
        if let Some(data) = self.files.get(path.as_str()) {
            Ok(FileStat {
                size: data.len() as u64,
                is_dir: false,
                created: None,
                modified: None,
            })
        } else if self.dirs.contains_key(path.as_str()) {
            Ok(FileStat {
                size: 0,
                is_dir: true,
                created: None,
                modified: None,
            })
        } else {
            Err(FsError::NotFound)
        }
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        self.dirs.insert(path.as_str().to_string(), ());
        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<(), FsError> {
        self.files.remove(path.as_str())
            .ok_or(FsError::NotFound)?;
        Ok(())
    }

    fn sync(&self) -> Result<(), FsError> {
        Ok(())
    }
}
```

**Usage in tests:**

```rust
#[test]
fn test_world_tree_storage() {
    let mock_fs = MockFs::new();
    let tree = WorldTree::new(Box::new(mock_fs));

    let hash = tree.store_scroll(b"Test content", "TestUser");
    let data = tree.retrieve(&hash).unwrap();

    assert_eq!(data, b"Test content");
}
```

---

## Integration with World-Tree

### Object Store on VFS

```rust
// groves/world-tree_grove/src/object_store.rs

use sha2::{Sha256, Digest};
use crate::vfs::FileSystem;

pub type Hash = [u8; 32];

pub struct ObjectStore {
    fs: Box<dyn FileSystem>,
    root: String,  // e.g., "/world-tree/objects"
}

impl ObjectStore {
    pub fn new(fs: Box<dyn FileSystem>, root: &str) -> Self {
        Self {
            fs,
            root: root.to_string(),
        }
    }

    /// Store a blob (Git-like)
    pub fn write_blob(&mut self, data: &[u8]) -> Result<Hash, StoreError> {
        // Compute SHA-256
        let hash: Hash = Sha256::digest(data).into();

        // Git-like path: objects/ab/cd1234...
        let hash_hex = hex::encode(hash);
        let path = format!("{}/{}/{}",
            self.root,
            &hash_hex[0..2],
            &hash_hex[2..]
        );

        // Ensure directory exists
        let dir = format!("{}/{}", self.root, &hash_hex[0..2]);
        let _ = self.fs.create_dir(&Path::new(&dir));

        // Write file
        self.fs.write(&Path::new(&path), data)?;

        Ok(hash)
    }

    /// Read a blob
    pub fn read_blob(&self, hash: &Hash) -> Result<Vec<u8>, StoreError> {
        let hash_hex = hex::encode(hash);
        let path = format!("{}/{}/{}",
            self.root,
            &hash_hex[0..2],
            &hash_hex[2..]
        );

        self.fs.read(&Path::new(&path))
            .map_err(|_| StoreError::NotFound)
    }

    /// Check if blob exists
    pub fn has_blob(&self, hash: &Hash) -> bool {
        let hash_hex = hex::encode(hash);
        let path = format!("{}/{}/{}",
            self.root,
            &hash_hex[0..2],
            &hash_hex[2..]
        );

        self.fs.exists(&Path::new(&path))
    }
}
```

### World-Tree on VFS

```rust
// groves/world-tree_grove/src/lib.rs

pub struct WorldTree {
    object_store: ObjectStore,
    metadata_db: MetadataDb,  // Could be SQLite or custom
}

impl WorldTree {
    /// Create World-Tree on any filesystem
    pub fn new(fs: Box<dyn FileSystem>) -> Result<Self, TreeError> {
        // Initialize object store
        let object_store = ObjectStore::new(fs.clone(), "/world-tree/objects");

        // Initialize metadata database
        let metadata_db = MetadataDb::new(fs, "/world-tree/metadata.db")?;

        Ok(Self {
            object_store,
            metadata_db,
        })
    }

    /// Store a scroll (file with metadata)
    pub fn store_scroll(&mut self, content: &[u8], creator: &str) -> Result<Hash, TreeError> {
        // Store content (Git-like)
        let hash = self.object_store.write_blob(content)?;

        // Store metadata
        self.metadata_db.insert(hash, Metadata {
            essence: "Scroll".to_string(),
            creator: creator.to_string(),
            genesis_time: now(),
            connections: vec![],
        })?;

        Ok(hash)
    }

    /// Query interface
    pub fn seek(&self) -> QueryBuilder {
        QueryBuilder::new(&self.metadata_db)
    }
}
```

---

## Implementation Phases

### Phase 1: VFS Abstraction (Week 1)

**Goals:**
- ✅ Define VFS trait
- ✅ Implement VfsManager
- ✅ Create MockFs for testing
- ✅ Write comprehensive tests

**Deliverables:**
- `heartwood/src/vfs/mod.rs` - Core trait
- `heartwood/src/vfs/manager.rs` - VFS manager
- `heartwood/src/vfs/mock.rs` - Mock filesystem
- `heartwood/src/vfs/tests.rs` - Test suite

**Success criteria:**
- All VFS operations work with MockFs
- Can mount/unmount filesystems
- Path resolution works correctly

---

### Phase 2: FAT32 Support (Week 2)

**Goals:**
- ✅ Integrate `fatfs` crate
- ✅ Implement FileSystem trait for FAT32
- ✅ Add block device abstraction
- ✅ Test with real FAT32 image

**Deliverables:**
- `heartwood/src/vfs/fat32.rs` - FAT32 driver
- `heartwood/src/block_device.rs` - Block device interface
- FAT32 test image for QEMU

**Success criteria:**
- Can read files from FAT32 partition
- Can write files to FAT32 partition
- Can list directories
- Works in QEMU with test disk image

---

### Phase 3: World-Tree Integration (Week 2-3)

**Goals:**
- ✅ Port Object Store to use VFS
- ✅ Create metadata database on VFS
- ✅ Implement Git-like storage layout
- ✅ Add Eldarin commands for testing

**Deliverables:**
- `groves/world-tree_grove/src/object_store.rs`
- `groves/world-tree_grove/src/metadata.rs`
- Eldarin commands: `wt-store`, `wt-read`, `wt-seek`

**Success criteria:**
- Can store blobs on FAT32
- Can retrieve blobs by hash
- Metadata persists across reboots
- Simple queries work

---

### Phase 4: ext4 Support (Week 4-5)

**Goals:**
- ✅ Port lwext4 or integrate ext4-rs
- ✅ Implement FileSystem trait for ext4
- ✅ Test journaling and crash recovery
- ✅ Benchmark performance vs FAT32

**Deliverables:**
- `heartwood/src/vfs/ext4.rs` - ext4 driver
- ext4 test images
- Performance benchmarks

**Success criteria:**
- Can read/write ext4 partitions
- Journaling works (test with forced crashes)
- Performance is acceptable
- Can install AethelOS on Linux partition

---

### Phase 5: NTFS Support (Week 6)

**Goals:**
- ✅ Integrate `ntfs` crate
- ✅ Implement read-only NTFS support
- ✅ Test with Windows partition
- ✅ Document limitations

**Deliverables:**
- `heartwood/src/vfs/ntfs.rs` - NTFS driver (read-only)
- NTFS test images
- Interop guide

**Success criteria:**
- Can read files from Windows partition
- Can list Windows directories
- Can access Documents, Pictures, etc.
- Clearly documented that write is not yet supported

---

### Phase 6: Multi-Mount Setup (Week 7)

**Goals:**
- ✅ Support multiple mounted filesystems
- ✅ Partition table parsing (MBR/GPT)
- ✅ Auto-detect filesystem types
- ✅ Create multi-boot examples

**Deliverables:**
- `heartwood/src/partition.rs` - Partition parsing
- Auto-mount logic
- Multi-boot guide
- Example disk layouts

**Success criteria:**
- Can mount multiple partitions
- Filesystem auto-detection works
- Can dual-boot with Linux/Windows
- World-Tree can span multiple filesystems

---

## Multi-Boot Support

### Example Disk Layout

```
/dev/sda (1TB SSD)
├─ sda1: EFI System Partition (FAT32, 512MB)
│  └─ /EFI/BOOT/BOOTX64.EFI    ← GRUB bootloader
│  └─ /aethelos/heartwood.bin   ← AethelOS kernel
│
├─ sda2: AethelOS Root (ext4, 50GB)
│  └─ /world-tree/              ← World-Tree storage
│     ├─ objects/ab/cd1234...   ← Content blobs
│     └─ metadata.db            ← Query index
│
├─ sda3: Linux Root (ext4, 50GB)
│  └─ /home/user/...            ← Linux files
│
└─ sda4: Shared Data (NTFS, 800GB)
   └─ /Documents/               ← Shared with Windows
   └─ /Pictures/
   └─ /Projects/
```

### Boot Configuration

```bash
# /boot/grub/grub.cfg

menuentry "AethelOS" {
    insmod gzio
    insmod part_gpt
    insmod ext4

    set root='hd0,gpt2'  # AethelOS partition

    multiboot2 /boot/aethelos/heartwood.bin
    boot
}

menuentry "Linux" {
    set root='hd0,gpt3'
    linux /boot/vmlinuz root=/dev/sda3
    initrd /boot/initrd.img
}
```

### Mount Configuration in AethelOS

```rust
// During boot
fn init_filesystems() -> Result<(), FsError> {
    let mut vfs = VfsManager::new();

    // Parse partition table
    let disk = BlockDevice::new(0)?;  // Primary disk
    let partitions = parse_gpt(&disk)?;

    // Mount EFI partition (FAT32)
    let efi_part = partitions.get(0).unwrap();
    let efi_fs = Fat32::new(efi_part.clone())?;
    vfs.mount("boot", Box::new(efi_fs))?;

    // Mount AethelOS root (ext4)
    let root_part = partitions.get(1).unwrap();
    let root_fs = Ext4::new(root_part.clone())?;
    vfs.mount("root", Box::new(root_fs))?;

    // Mount shared data (NTFS, read-only for now)
    let data_part = partitions.get(3).unwrap();
    let data_fs = NtfsFs::new(data_part.clone())?;
    vfs.mount("data", Box::new(data_fs))?;

    // Initialize World-Tree on root filesystem
    let root_fs = vfs.get("root").unwrap();
    let tree = WorldTree::new(Box::new(root_fs))?;

    crate::println!("◈ Filesystems mounted:");
    crate::println!("  /boot  → FAT32 (EFI partition)");
    crate::println!("  /root  → ext4 (AethelOS root)");
    crate::println!("  /data  → NTFS (Shared data, read-only)");

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_fs_read_write() {
        let mut fs = MockFs::new();
        let path = Path::new("/test.txt");

        fs.write(&path, b"Hello, World!").unwrap();
        let data = fs.read(&path).unwrap();

        assert_eq!(data, b"Hello, World!");
    }

    #[test]
    fn test_vfs_mounting() {
        let mut vfs = VfsManager::new();
        let mock_fs = Box::new(MockFs::new());

        vfs.mount("test", mock_fs).unwrap();

        assert!(vfs.get("test").is_some());
        assert_eq!(vfs.mounts().len(), 1);
    }

    #[test]
    fn test_path_resolution() {
        let mut vfs = VfsManager::new();
        vfs.mount("root", Box::new(MockFs::new())).unwrap();

        let (mount, fs, rel_path) = vfs.resolve(&Path::new("/root/foo/bar.txt")).unwrap();

        assert_eq!(mount, "root");
        assert_eq!(rel_path.as_str(), "foo/bar.txt");
    }
}
```

### Integration Tests

```rust
#[test]
fn test_world_tree_on_fat32() {
    // Create FAT32 image
    let img_path = create_test_fat32_image();
    let device = BlockDevice::from_file(&img_path).unwrap();
    let fs = Fat32::new(device).unwrap();

    // Create World-Tree
    let mut tree = WorldTree::new(Box::new(fs)).unwrap();

    // Store a scroll
    let hash = tree.store_scroll(b"Test content", "Tester").unwrap();

    // Retrieve it
    let data = tree.read_blob(&hash).unwrap();
    assert_eq!(data, b"Test content");

    // Query by metadata
    let results = tree.seek()
        .where_creator("Tester")
        .execute()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], hash);
}
```

### Performance Tests

```rust
#[test]
fn bench_object_store_throughput() {
    let mock_fs = Box::new(MockFs::new());
    let mut store = ObjectStore::new(mock_fs, "/objects");

    let start = now();

    // Write 1000 blobs
    for i in 0..1000 {
        let data = format!("Blob {}", i).into_bytes();
        store.write_blob(&data).unwrap();
    }

    let elapsed = now() - start;
    let throughput = 1000.0 / elapsed.as_secs_f64();

    crate::println!("Object store throughput: {:.2} blobs/sec", throughput);
}
```

---

## Timeline and Milestones

### Week 1: VFS Foundation
- **Day 1-2:** Design and implement VFS trait
- **Day 3-4:** Implement VfsManager
- **Day 5:** Create MockFs and write tests
- **Milestone:** VFS abstraction complete, all tests pass

### Week 2: FAT32 Support
- **Day 1-2:** Block device abstraction
- **Day 3-4:** Integrate fatfs crate
- **Day 5:** Testing with FAT32 images
- **Milestone:** Can read/write FAT32 in QEMU

### Week 3: World-Tree Integration
- **Day 1-2:** Port Object Store to VFS
- **Day 3-4:** Implement metadata storage
- **Day 5:** Add Eldarin commands
- **Milestone:** World-Tree works on FAT32

### Week 4-5: ext4 Support
- **Week 4:** Port lwext4, implement trait
- **Week 5:** Testing, journaling, crash recovery
- **Milestone:** Can use ext4 partitions

### Week 6: NTFS Support
- **Day 1-3:** Integrate ntfs crate
- **Day 4-5:** Testing with Windows partitions
- **Milestone:** Read-only NTFS works

### Week 7: Multi-Boot
- **Day 1-2:** Partition table parsing
- **Day 3-4:** Auto-mount logic
- **Day 5:** Documentation and examples
- **Milestone:** Can dual-boot with Linux/Windows

---

## Success Criteria

### Functional Requirements

- ✅ VFS trait is clean and extensible
- ✅ FAT32 read/write works reliably
- ✅ ext4 read/write works with journaling
- ✅ NTFS read-only works
- ✅ World-Tree stores objects on any filesystem
- ✅ Can mount multiple filesystems simultaneously
- ✅ Filesystem auto-detection works
- ✅ Can dual-boot with other OSes

### Performance Requirements

- ✅ Object store: >100 blobs/sec write throughput
- ✅ Metadata queries: <100ms for typical queries
- ✅ File read: Within 2x of Linux performance
- ✅ File write: Within 2x of Linux performance

### Quality Requirements

- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ No data corruption under normal use
- ✅ Graceful handling of disk full
- ✅ Proper error messages
- ✅ Comprehensive documentation

---

## Future Enhancements

### Phase 8: Advanced Features (Post v1.0)

**Write support for NTFS:**
- Complex but valuable for Windows interop
- Requires careful implementation (easy to corrupt)

**Network filesystems:**
- NFS client (access Linux network shares)
- SMB/CIFS client (access Windows network shares)

**Copy-on-Write filesystem:**
- Custom AethelFS optimized for World-Tree
- Native snapshots and versioning
- Compression and deduplication

**FUSE support:**
- Let userspace implement filesystems
- Enable experimentation without kernel changes

---

## Philosophical Notes

### Why This Approach Works

**Git proves it:** Git doesn't have a filesystem. It uses whatever is available. Yet it's the most successful version control system ever.

**Databases prove it:** PostgreSQL, MySQL, SQLite—none make their own filesystems. They focus on database features.

**Docker proves it:** Uses overlay filesystems on top of ext4/btrfs/whatever. Focuses on containers, not storage.

**AethelOS follows this pattern:** Focus on what's unique (queries, versioning, metadata). Reuse proven infrastructure (filesystems).

### The Abstraction Principle

> *"The World-Tree does not care whether it grows in ext4 soil or FAT32 sand. It adapts to its environment while maintaining its essential nature."*

Good abstractions enable:
- **Flexibility** - Switch storage backends easily
- **Testing** - Use mocks for fast unit tests
- **Compatibility** - Work with existing systems
- **Evolution** - Can optimize later without breaking API

### Engineering Pragmatism

**Build what differentiates you. Reuse what doesn't.**

World-Tree's differentiation:
- Query-based interface ← Build this
- Rich metadata ← Build this
- Versioning model ← Build this

Commodity infrastructure:
- Block allocation ← Reuse ext4
- Crash recovery ← Reuse journaling
- I/O optimization ← Reuse proven filesystems

**Result:** Ship in 6 weeks instead of 6 months.

---

## Conclusion

The VFS layer is **critical infrastructure** that unlocks:
- ✅ Multi-filesystem support (FAT32, ext4, NTFS)
- ✅ Interoperability (dual-boot, shared partitions)
- ✅ Flexibility (can optimize later)
- ✅ Testing (mock filesystems)
- ✅ Focus (build World-Tree features, not filesystem internals)

**This is the right architecture.** It's not a compromise—it's **smart engineering.**

---

*"The strongest trees don't resist the wind; they bend with it. So too does AethelOS adapt to the storage landscape, drawing strength from proven foundations."*

**Status:** Ready to implement
**First step:** Phase 1 (VFS Abstraction, Week 1)
**Next review:** After FAT32 integration (Week 2)

---

## References

- **Git Object Model:** https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
- **fatfs crate:** https://crates.io/crates/fatfs
- **ntfs crate:** https://crates.io/crates/ntfs
- **lwext4:** https://github.com/gkostka/lwext4
- **VFS in Linux:** https://www.kernel.org/doc/html/latest/filesystems/vfs.html
- **World-Tree Plan:** [WORLD_TREE_PLAN.md](WORLD_TREE_PLAN.md)
