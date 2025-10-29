//! # Interrupt-Safe Locking
//!
//! Locks that can safely be held even when interrupts occur.
//! These locks disable interrupts while held to prevent deadlocks.

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;

/// A spinlock that disables interrupts while held
///
/// This prevents deadlocks that can occur when:
/// 1. Thread acquires lock
/// 2. Interrupt fires
/// 3. Interrupt handler tries to acquire same lock
/// 4. Deadlock!
///
/// By disabling interrupts while the lock is held, we ensure
/// interrupts cannot fire during critical sections.
pub struct InterruptSafeLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Sync for InterruptSafeLock<T> {}
unsafe impl<T: Send> Send for InterruptSafeLock<T> {}

impl<T> InterruptSafeLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock, returning a guard that restores interrupt state on drop
    pub fn lock(&self) -> InterruptSafeLockGuard<'_, T> {
        let interrupts_enabled = are_interrupts_enabled();
        disable_interrupts();

        while self.locked.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }

        InterruptSafeLockGuard {
            lock: self,
            restore_interrupts: interrupts_enabled,
        }
    }

    /// Force unlock (unsafe - only use if you know the lock is held)
    ///
    /// # Safety
    /// Only call this if you know for certain the lock is currently held
    pub unsafe fn force_unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// DIAGNOSTIC: Check if the lock is currently held
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }
}

pub struct InterruptSafeLockGuard<'a, T> {
    lock: &'a InterruptSafeLock<T>,
    restore_interrupts: bool,
}

impl<'a, T> Drop for InterruptSafeLockGuard<'a, T> {
    fn drop(&mut self) {
        // Release the lock
        self.lock.locked.store(false, Ordering::Release);

        // Restore interrupt state
        if self.restore_interrupts {
            enable_interrupts();
        }
    }
}

impl<'a, T> core::ops::Deref for InterruptSafeLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for InterruptSafeLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

/// Check if interrupts are currently enabled
#[inline]
fn are_interrupts_enabled() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            out(reg) flags,
            options(nomem, preserves_flags)
        );
    }
    (flags & 0x200) != 0 // IF flag is bit 9
}

/// Disable interrupts
#[inline]
fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Enable interrupts
#[inline]
fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_basic() {
        let lock = InterruptSafeLock::new(42);
        {
            let guard = lock.lock();
            assert_eq!(*guard, 42);
        }
        // Lock should be released after guard drops
    }
}
