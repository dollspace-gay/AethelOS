//! VFS Debug Command - shows mount status

use crate::vfs::global as vfs_global;

pub fn show_mount_status() {
    crate::println!("◈ VFS Mount Status");
    crate::println!();

    match vfs_global::get() {
        None => {
            crate::println!("  ✗ Global VFS not initialized");
            crate::println!("  (This is a bug - VFS should initialize at boot)");
        }
        Some(global_fs) => {
            crate::println!("  ✓ Global VFS initialized");

            let fs_lock = global_fs.lock();
            match &*fs_lock {
                None => {
                    crate::println!("  ✗ No filesystem mounted");
                    crate::println!();
                    crate::println!("  Possible reasons:");
                    crate::println!("  - No ATA drive detected (check boot messages)");
                    crate::println!("  - FAT32 mount failed (disk not formatted?)");
                    crate::println!("  - QEMU not started with -hda disk.img");
                }
                Some(fs) => {
                    crate::println!("  ✓ Filesystem mounted!");
                    crate::println!("  Type: {}", fs.name());
                    crate::println!();
                    crate::println!("  Try: vfs-ls / to list root directory");
                }
            }
        }
    }
}
