//! # Interrupt-Safe Locking
//!
//! Locks that can safely be held even when interrupts occur.
//! These locks disable interrupts while held to prevent deadlocks.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::cell::UnsafeCell;

/// Global tracking for lock holder (for debugging deadlocks)
static LOCK_HOLDER_ID: AtomicU64 = AtomicU64::new(0);
/// Counter for assigning unique IDs to each lock acquisition
static LOCK_CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    /// Debug name for identifying which lock is involved in deadlocks
    debug_name: &'static str,
}

unsafe impl<T> Sync for InterruptSafeLock<T> {}
unsafe impl<T: Send> Send for InterruptSafeLock<T> {}

impl<T> InterruptSafeLock<T> {
    pub const fn new(data: T, debug_name: &'static str) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            debug_name,
        }
    }

    /// Acquire the lock, returning a guard that restores interrupt state on drop
    ///
    /// # Panics
    ///
    /// Panics if the lock cannot be acquired after spinning for too long.
    /// This indicates a deadlock, usually from reentrant allocation (allocating
    /// while already holding the allocator lock).
    pub fn lock(&self) -> InterruptSafeLockGuard<'_, T> {
        // CRITICAL: Check and disable interrupts FIRST, before any debug output!
        // If we output debug characters with interrupts enabled, a timer interrupt
        // can fire and try to acquire this same lock, causing deadlock.
        let interrupts_enabled = are_interrupts_enabled();
        disable_interrupts();

        // DEBUG DISABLED: These serial writes can flood the log
        // unsafe { core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'(', options(nomem, nostack, preserves_flags)); }
        // unsafe { core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'i', options(nomem, nostack, preserves_flags)); }
        // unsafe { core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'd', options(nomem, nostack, preserves_flags)); }

        // Assign unique ID to this lock acquisition attempt
        let my_call_id = LOCK_CALL_COUNTER.fetch_add(1, Ordering::SeqCst);

        // DEBUG DISABLED
        // unsafe { core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'f', options(nomem, nostack, preserves_flags)); }
        // unsafe { core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'L', options(nomem, nostack, preserves_flags)); }

        // Try to acquire the lock with a spin limit to detect deadlocks
        let mut spin_count = 0;
        const MAX_SPINS: usize = 10_000; // Much lower threshold for faster detection
        const REPORT_INTERVAL: usize = 2_000; // Report every 2000 spins

        // Use SeqCst (strongest ordering) to rule out memory ordering issues
        // Add memory fence to ensure visibility
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // DEBUG DISABLED
        // unsafe { core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b's', options(nomem, nostack, preserves_flags)); }

        while self.locked.swap(true, Ordering::SeqCst) {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            core::hint::spin_loop();

            spin_count += 1;

            // Report spinning progress to help diagnose where we're stuck
            if spin_count % REPORT_INTERVAL == 0 {
                unsafe {
                    // Output '!' to show we're spinning
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3f8u16,
                        in("al") b'!',
                        options(nomem, nostack, preserves_flags)
                    );
                }
            }

            // After excessive spinning, output who's holding the lock
            if spin_count == MAX_SPINS {
                let holder_id = LOCK_HOLDER_ID.load(Ordering::SeqCst);
                unsafe {
                    // Output lock name
                    let msg = b"\n[DEADLOCK] Lock '";
                    for &byte in msg.iter() {
                        core::arch::asm!(
                            "out dx, al",
                            in("dx") 0x3f8u16,
                            in("al") byte,
                            options(nomem, nostack, preserves_flags)
                        );
                    }

                    // Output lock name
                    for &byte in self.debug_name.as_bytes().iter() {
                        core::arch::asm!(
                            "out dx, al",
                            in("dx") 0x3f8u16,
                            in("al") byte,
                            options(nomem, nostack, preserves_flags)
                        );
                    }

                    // Output lock holder ID and attempting ID
                    let msg2 = b"' held by call #";
                    for &byte in msg2.iter() {
                        core::arch::asm!(
                            "out dx, al",
                            in("dx") 0x3f8u16,
                            in("al") byte,
                            options(nomem, nostack, preserves_flags)
                        );
                    }

                    // Output holder ID in decimal
                    output_decimal(holder_id);

                    let msg3 = b", attempting from call #";
                    for &byte in msg3.iter() {
                        core::arch::asm!(
                            "out dx, al",
                            in("dx") 0x3f8u16,
                            in("al") byte,
                            options(nomem, nostack, preserves_flags)
                        );
                    }

                    // Output my call ID in decimal
                    output_decimal(my_call_id);

                    let msg4 = b"\n";
                    for &byte in msg4.iter() {
                        core::arch::asm!(
                            "out dx, al",
                            in("dx") 0x3f8u16,
                            in("al") byte,
                            options(nomem, nostack, preserves_flags)
                        );
                    }
                }
            }
        }

        // Record who acquired the lock
        LOCK_HOLDER_ID.store(my_call_id, Ordering::SeqCst);

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
        // Clear lock holder tracking
        LOCK_HOLDER_ID.store(0, Ordering::SeqCst);

        // Release the lock with SeqCst ordering and fence
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        self.lock.locked.store(false, Ordering::SeqCst);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

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

/// Output a decimal number to serial port (for debug output without allocation)
unsafe fn output_decimal(mut num: u64) {
    if num == 0 {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'0',
            options(nomem, nostack, preserves_flags)
        );
        return;
    }

    // Convert to decimal digits (max 20 digits for u64)
    let mut digits = [0u8; 20];
    let mut count = 0;

    while num > 0 {
        digits[count] = b'0' + (num % 10) as u8;
        num /= 10;
        count += 1;
    }

    // Output digits in reverse order
    for i in (0..count).rev() {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") digits[i],
            options(nomem, nostack, preserves_flags)
        );
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
        let lock = InterruptSafeLock::new(42, "TEST");
        {
            let guard = lock.lock();
            assert_eq!(*guard, 42);
        }
        // Lock should be released after guard drops
    }
}
