//! # IRQ-Safe Mutex
//!
//! A mutex that automatically disables interrupts when locked,
//! preventing deadlocks from interrupt handlers trying to acquire
//! the same lock.

use spin::{Mutex, MutexGuard};
use core::ops::{Deref, DerefMut};

pub struct IrqSafeMutex<T> {
    pub(crate) inner: Mutex<T>,  // Accessible within the crate for write_char_unlocked
}

impl<T> IrqSafeMutex<T> {
    pub const fn new(data: T) -> Self {
        IrqSafeMutex {
            inner: Mutex::new(data),
        }
    }

    pub fn lock(&self) -> IrqSafeMutexGuard<T> {
        // Check if interrupts are currently enabled
        let were_enabled: bool;
        unsafe {
            let flags: u64;
            core::arch::asm!(
                "pushfq",
                "pop {0}",
                out(reg) flags,
                options(nomem, preserves_flags)
            );
            were_enabled = (flags & 0x200) != 0;
        }

        // Disable interrupts BEFORE acquiring the lock
        if were_enabled {
            unsafe {
                core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
            }
        }

        IrqSafeMutexGuard {
            guard: self.inner.lock(),
            irq_enabled_on_entry: were_enabled,
        }
    }

    /// Force unlock the mutex (dangerous - only use in special cases like before context switch)
    pub unsafe fn force_unlock(&self) {
        self.inner.force_unlock();
    }
}

// Custom guard that re-enables interrupts when dropped
pub struct IrqSafeMutexGuard<'a, T> {
    guard: MutexGuard<'a, T>,
    irq_enabled_on_entry: bool,
}

impl<'a, T> Drop for IrqSafeMutexGuard<'a, T> {
    fn drop(&mut self) {
        // Debug: Mark that we're dropping the guard (releasing the lock)
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") b'~' as u8,  // ~ = Lock being released
                options(nomem, nostack, preserves_flags)
            );
        }

        // The inner guard drops here automatically, releasing the spinlock

        // Re-enable interrupts ONLY if they were enabled when we entered
        if self.irq_enabled_on_entry {
            unsafe {
                core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
                core::arch::asm!(
                    "out dx, al",
                    in("dx") 0x3f8u16,
                    in("al") b'^' as u8,  // ^ = Interrupts re-enabled
                    options(nomem, nostack, preserves_flags)
                );
            }
        }
    }
}

// Allow using the guard like a normal MutexGuard
impl<'a, T> Deref for IrqSafeMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.guard
    }
}

impl<'a, T> DerefMut for IrqSafeMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.guard
    }
}
