# ext4 Filesystem Driver for AethelOS

## Overview

This directory contains a **read-only ext4 filesystem driver** for AethelOS. It implements the VFS `FileSystem` trait, allowing AethelOS to read files and directories from ext4 volumes.

## Architecture

The ext4 driver is structured into several modules:

```
ext4/
├── mod.rs          - Main Ext4 struct and FileSystem trait implementation
├── superblock.rs   - Superblock and group descriptor parsing
├── inode.rs        - Inode reading and parsing
├── extent.rs       - Extent tree navigation for file data access
└── dir.rs          - Directory entry parsing
```

## Features

### ✅ Implemented

- **Superblock Parsing**: Reads and validates ext4 superblock metadata
- **64-bit Support**: Handles filesystems larger than 2TB
- **Extent Tree Navigation**: Efficiently maps logical file blocks to physical disk blocks
- **Inode Reading**: Supports both 128-byte and 256-byte inode formats
- **Directory Traversal**: Parses linear directory entries (dir_entry_2 format)
- **File Reading**: Reads complete files using extent trees
- **Sparse Files**: Handles sparse blocks (unallocated regions) correctly
- **VFS Integration**: Fully implements the `FileSystem` trait

### ⚠️ Limitations (Read-Only)

- **No Write Support**: Cannot create, modify, or delete files
- **No Journal Replay**: Does not replay the journal on mount
- **No Extended Attributes**: Does not parse extended attributes (xattrs)
- **No HTree Support**: Linear directory parsing only (no hash tree optimization)
- **No Indirect Blocks**: Only supports extent-based files (not legacy indirect block maps)
- **No Inline Data**: Does not handle files with inline data flag

## Usage

### Mounting an ext4 Volume

```rust
use vfs::ext4::Ext4;
use vfs::FileSystem;

// Assuming you have a block device (e.g., ATA disk)
let block_device = Box::new(ata_device);

// Mount the ext4 filesystem
let fs = Ext4::new(block_device)?;

// Read a file
let data = fs.read(&Path::new("/home/user/document.txt"))?;

// List directory
let entries = fs.read_dir(&Path::new("/home/user"))?;
for entry in entries {
    println!("{} ({})", entry.name, if entry.is_dir { "dir" } else { "file" });
}
```

### Integration with VFS Manager

```rust
use vfs::manager::VfsManager;
use vfs::ext4::Ext4;

let mut vfs = VfsManager::new();

// Mount ext4 volume at /mnt/disk
let ext4_fs = Ext4::new(block_device)?;
vfs.mount("/mnt/disk", Box::new(ext4_fs))?;

// Access files through VFS
let data = vfs.read(&Path::new("/mnt/disk/file.txt"))?;
```

## Technical Details

### Superblock (superblock.rs)

- Located at byte offset 1024 from partition start
- Contains filesystem metadata: block size, inode count, feature flags
- Validates magic number (0xEF53)
- Supports 64-bit block addressing

### Inodes (inode.rs)

- Inode numbers are 1-indexed (as per ext4 spec)
- Root directory is always inode 2
- Inodes are located using block group descriptors
- Supports variable inode sizes (128 or 256 bytes)

### Extent Trees (extent.rs)

- Efficient mapping of logical blocks → physical blocks
- Extent header at offset 0 (12 bytes)
- Extent entries (leaves) or index entries (internal nodes)
- Recursive tree traversal for multi-level extents
- Handles sparse files (unmapped blocks return zeros)

### Directory Entries (dir.rs)

- Variable-length entries (minimum 8 bytes)
- Format: inode (4) + rec_len (2) + name_len (1) + file_type (1) + name (variable)
- Skips "." and ".." entries when listing directories
- Case-sensitive name matching

## Constants

### ext4 Feature Flags

- `INCOMPAT_EXTENTS` (0x0040): Extent tree support
- `INCOMPAT_64BIT` (0x0080): 64-bit block addressing
- `INCOMPAT_FLEX_BG` (0x0200): Flexible block groups

### File Type Constants

- `S_IFREG` (0x8000): Regular file
- `S_IFDIR` (0x4000): Directory
- `S_IFLNK` (0xA000): Symbolic link

### Inode Flags

- `EXT4_EXTENTS_FL` (0x00080000): Uses extent tree
- `EXT4_INLINE_DATA_FL` (0x10000000): Has inline data (not supported)

## Future Enhancements

### Planned Features

1. **Write Support**: Implement file/directory creation and modification
2. **Journal Replay**: Ensure filesystem consistency by replaying journal on mount
3. **HTree Directories**: Optimize large directory access with hash trees
4. **Extended Attributes**: Parse and expose xattrs
5. **Indirect Block Support**: Handle legacy filesystems without extents
6. **Inline Data**: Support small files stored directly in inodes
7. **Symbolic Link Resolution**: Follow symlinks transparently

### Performance Optimizations

- Cache block group descriptors
- Cache extent tree nodes
- Buffer directory entries
- Implement read-ahead for sequential access

## Testing

### Unit Tests

Tests are located in `mod.rs` under `#[cfg(test)]`:

```rust
// TODO: Implement unit tests with mock block device
```

### Integration Testing

To test with a real ext4 image:

1. Create a test ext4 disk image:
   ```bash
   dd if=/dev/zero of=test.img bs=1M count=10
   mkfs.ext4 test.img
   mkdir /mnt/test
   mount test.img /mnt/test
   echo "Hello AethelOS" > /mnt/test/test.txt
   umount /mnt/test
   ```

2. Attach the image to QEMU as a second disk:
   ```bash
   qemu-system-x86_64 -cdrom aethelos.iso -drive file=test.img,format=raw
   ```

3. Mount and read from AethelOS shell:
   ```
   eldarin> mount /dev/sdb1 /mnt/disk ext4
   eldarin> cat /mnt/disk/test.txt
   ```

## References

- [ext4 Disk Layout (kernel.org)](https://www.kernel.org/doc/html/latest/filesystems/ext4/index.html)
- [The Second Extended Filesystem (MIT)](https://www.nongnu.org/ext2-doc/ext2.html) - Many concepts apply to ext4
- [ext4 On-Disk Format Specification](https://ext4.wiki.kernel.org/index.php/Ext4_Disk_Layout)

## Contributing

When modifying the ext4 driver:

1. **Follow AethelOS Conventions**: Use poetic naming where appropriate
2. **Document Safety Invariants**: All `unsafe` blocks need `/// SAFETY:` comments
3. **Handle Errors Gracefully**: Return `FsError` variants, never panic
4. **Test Thoroughly**: Verify with real ext4 images, not just mocks
5. **Track with bd**: Create issues with `bd create` for all tasks

## License

This driver is part of AethelOS and follows the same license as the main project.

---

*Last updated: January 2025*
*Driver version: 0.1.0 (read-only)*
