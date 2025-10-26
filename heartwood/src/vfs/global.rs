//! Global VFS mount point
//!
//! Simple global filesystem for testing until proper VFS manager is implemented

use super::FileSystem;
use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;
use alloc::boxed::Box;

/// Global mounted filesystem
static mut GLOBAL_FS: MaybeUninit<InterruptSafeLock<Option<Box<dyn FileSystem>>>> = MaybeUninit::uninit();
static mut FS_INITIALIZED: bool = false;

/// Initialize the global filesystem
pub fn init() {
    unsafe {
        let fs_option: Option<Box<dyn FileSystem>> = None;
        let lock = InterruptSafeLock::new(fs_option);
        core::ptr::write(core::ptr::addr_of_mut!(GLOBAL_FS).cast(), lock);
        FS_INITIALIZED = true;
    }
}

/// Mount a filesystem as the global root
pub fn mount(fs: Box<dyn FileSystem>) {
    unsafe {
        if !FS_INITIALIZED {
            init();
        }
        let global_fs = &*core::ptr::addr_of!(GLOBAL_FS).cast::<InterruptSafeLock<Option<Box<dyn FileSystem>>>>();
        let mut fs_lock = global_fs.lock();
        *fs_lock = Some(fs);
    }
}

/// Get reference to global filesystem
pub fn get() -> Option<&'static InterruptSafeLock<Option<Box<dyn FileSystem>>>> {
    unsafe {
        if !FS_INITIALIZED {
            return None;
        }
        Some(&*core::ptr::addr_of!(GLOBAL_FS).cast::<InterruptSafeLock<Option<Box<dyn FileSystem>>>>())
    }
}
