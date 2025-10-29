//! # The Ward of Sacred Boundaries
//!
//! *"The Heartwood must never be deceived into treating a mortal's idle thoughts*
//! *as a true spell, nor may it touch a cursed scroll without first sanctifying it."*
//!
//! ## Philosophy
//!
//! The Heartwood (kernel space) is a sacred, pure realm. The mortal lands (user space)
//! are chaotic and untrusted. The Ward of Sacred Boundaries enforces a strict separation:
//!
//! - **SMEP** (Supervisor Mode Execution Prevention): The Heartwood is forbidden from
//!   executing code located in the user's memory space. A mortal's idle thoughts cannot
//!   become true spells within the sacred realm.
//!
//! - **SMAP** (Supervisor Mode Access Prevention): When a mortal hands the Heartwood
//!   a scroll (a pointer), the Heartwood is forbidden from reading it directly. It must
//!   first create a perfect, sanctified copy within its own sacred space.
//!
//! - **UDEREF** (User Dereference Prevention): All user pointers must be validated and
//!   copied through sanctified functions. No kernel code may blindly trust a user pointer.
//!
//! ## Implementation
//!
//! This module provides:
//! - CPU feature detection and enablement (SMEP/SMAP in CR4)
//! - Safe copy functions (`sanctified_copy_from_mortal`, `sanctified_copy_to_mortal`)
//! - User pointer validation (`is_mortal_pointer`, `validate_mortal_region`)
//! - Compile-time enforcement via type system (`MortalPointer<T>`)

use core::arch::asm;
use core::mem::size_of;

/// CR4 bit for SMEP (Supervisor Mode Execution Prevention)
const CR4_SMEP: u64 = 1 << 20;

/// CR4 bit for SMAP (Supervisor Mode Access Prevention)
const CR4_SMAP: u64 = 1 << 21;

/// CPUID leaf for extended features
const CPUID_EXTENDED_FEATURES: u32 = 0x07;

/// CPUID EBX bit for SMEP support
const CPUID_SMEP: u32 = 1 << 7;

/// CPUID EBX bit for SMAP support
const CPUID_SMAP: u32 = 1 << 20;

/// User space address range (0x0000_0000_0000_0000 to 0x0000_7FFF_FFFF_FFFF)
/// Kernel space starts at 0xFFFF_8000_0000_0000
const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_FFFF;

/// Kernel space start (higher half)
const KERNEL_SPACE_START: u64 = 0xFFFF_8000_0000_0000;

/// Ward initialization status
static mut WARD_ENABLED: bool = false;

/// Error types for Ward operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WardError {
    /// Pointer is in kernel space (forbidden for user pointers)
    PointerInKernelSpace,
    /// Pointer is null
    NullPointer,
    /// Region overflows into kernel space
    RegionOverflow,
    /// CPU does not support required features
    UnsupportedCpu,
    /// Copy operation failed
    CopyFailed,
}

impl core::fmt::Display for WardError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WardError::PointerInKernelSpace => write!(f, "Mortal pointer trespasses in sacred realm"),
            WardError::NullPointer => write!(f, "Mortal offers void as truth"),
            WardError::RegionOverflow => write!(f, "Mortal's scroll breaches the boundary"),
            WardError::UnsupportedCpu => write!(f, "The ancient stones lack the Ward's wisdom"),
            WardError::CopyFailed => write!(f, "Sanctification ritual failed"),
        }
    }
}

/// A mortal pointer - guaranteed to point to user space only
///
/// This type cannot be created from arbitrary addresses; it must be validated
/// through `MortalPointer::new()` which ensures the pointer is in user space.
///
/// # Safety
///
/// Once created, a MortalPointer is guaranteed to point to user space (< 0x8000_0000_0000_0000).
/// However, the contents are still untrusted and must be copied via sanctified functions.
#[repr(transparent)]
pub struct MortalPointer<T> {
    addr: u64,
    _phantom: core::marker::PhantomData<*const T>,
}

impl<T> MortalPointer<T> {
    /// Create a new MortalPointer from a raw address
    ///
    /// # Arguments
    ///
    /// * `addr` - Raw address to validate
    ///
    /// # Returns
    ///
    /// * `Ok(MortalPointer<T>)` - Valid user space pointer
    /// * `Err(WardError)` - Invalid pointer (null or in kernel space)
    pub fn new(addr: u64) -> Result<Self, WardError> {
        validate_mortal_pointer(addr, size_of::<T>())?;
        Ok(Self {
            addr,
            _phantom: core::marker::PhantomData,
        })
    }

    /// Get the raw address (for internal use only)
    pub(crate) fn addr(&self) -> u64 {
        self.addr
    }
}

/// Initialize the Ward of Sacred Boundaries
///
/// Detects CPU support for SMEP/SMAP and enables them in CR4.
///
/// # Returns
///
/// * `Ok(())` - Ward enabled successfully
/// * `Err(WardError::UnsupportedCpu)` - CPU lacks SMEP/SMAP support
///
/// # Safety
///
/// Must be called during kernel initialization, before any user space interaction.
pub unsafe fn init_ward() -> Result<(), WardError> {
    crate::serial_println!("[WARD] Initializing Ward of Sacred Boundaries...");

    // Check CPU support for SMEP and SMAP
    let (smep_supported, smap_supported) = check_cpu_features();

    if !smep_supported && !smap_supported {
        crate::serial_println!("[WARD] ✗ CPU lacks SMEP and SMAP support");
        return Err(WardError::UnsupportedCpu);
    }

    // Read current CR4
    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));

    // Enable SMEP if supported
    if smep_supported {
        cr4 |= CR4_SMEP;
        crate::serial_println!("[WARD] ✓ SMEP (Supervisor Mode Execution Prevention) enabled");
    } else {
        crate::serial_println!("[WARD] ⚠ SMEP not supported on this CPU");
    }

    // Enable SMAP if supported
    if smap_supported {
        cr4 |= CR4_SMAP;
        crate::serial_println!("[WARD] ✓ SMAP (Supervisor Mode Access Prevention) enabled");
    } else {
        crate::serial_println!("[WARD] ⚠ SMAP not supported on this CPU");
    }

    // Write back to CR4
    asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));

    WARD_ENABLED = true;
    crate::serial_println!("[WARD] ✓ The Ward of Sacred Boundaries stands vigilant");

    Ok(())
}

/// Check CPU support for SMEP and SMAP
///
/// Uses CPUID instruction to query CPU capabilities.
///
/// # Returns
///
/// * `(smep_supported, smap_supported)` - Tuple of feature availability
fn check_cpu_features() -> (bool, bool) {
    let ebx: u32;

    unsafe {
        // CPUID with EAX=0x07, ECX=0x00 for extended features
        // Note: We must save/restore RBX because LLVM reserves it
        asm!(
            "push rbx",             // Save rbx on stack
            "mov eax, 0x07",        // Set EAX to CPUID leaf 7
            "xor ecx, ecx",         // Set ECX to 0
            "cpuid",                // Execute CPUID
            "mov {0:e}, ebx",       // Save ebx output (32-bit)
            "pop rbx",              // Restore rbx from stack
            out(reg) ebx,
            out("rax") _,
            out("rcx") _,
            out("rdx") _,
        );
    }

    let smep_supported = (ebx & CPUID_SMEP) != 0;
    let smap_supported = (ebx & CPUID_SMAP) != 0;

    (smep_supported, smap_supported)
}

/// Check if the Ward is currently enabled
pub fn is_ward_enabled() -> bool {
    unsafe { WARD_ENABLED }
}

/// Validate a mortal (user space) pointer
///
/// Ensures the pointer and the region it points to are entirely in user space.
///
/// # Arguments
///
/// * `addr` - Address to validate
/// * `size` - Size of the region in bytes
///
/// # Returns
///
/// * `Ok(())` - Pointer is valid
/// * `Err(WardError)` - Pointer is invalid
pub fn validate_mortal_pointer(addr: u64, size: usize) -> Result<(), WardError> {
    // Check for null pointer
    if addr == 0 {
        return Err(WardError::NullPointer);
    }

    // Check if pointer is in kernel space
    if addr >= KERNEL_SPACE_START {
        return Err(WardError::PointerInKernelSpace);
    }

    // Check if region overflows into kernel space
    let end_addr = addr.checked_add(size as u64)
        .ok_or(WardError::RegionOverflow)?;

    if end_addr > USER_SPACE_END {
        return Err(WardError::RegionOverflow);
    }

    Ok(())
}

/// Check if an address is in mortal (user) space
///
/// # Arguments
///
/// * `addr` - Address to check
///
/// # Returns
///
/// * `true` - Address is in user space
/// * `false` - Address is in kernel space or invalid
pub fn is_mortal_pointer(addr: u64) -> bool {
    addr != 0 && addr < KERNEL_SPACE_START
}

/// Sanctified copy from mortal lands (copy_from_user)
///
/// Creates a pure, sanctified copy of data from user space into kernel space.
/// The Heartwood never directly touches the mortal's scroll - it creates a replica.
///
/// # Arguments
///
/// * `mortal_ptr` - Validated pointer to user space data
/// * `dest` - Kernel space destination buffer
///
/// # Returns
///
/// * `Ok(())` - Copy successful
/// * `Err(WardError)` - Copy failed
///
/// # Safety
///
/// - `mortal_ptr` must be a valid MortalPointer
/// - `dest` must be a valid kernel space buffer with sufficient space
/// - Caller must ensure proper type alignment and lifetime
pub unsafe fn sanctified_copy_from_mortal<T: Copy>(
    mortal_ptr: &MortalPointer<T>,
    dest: &mut T,
) -> Result<(), WardError> {
    // Temporarily disable SMAP to allow access (STAC instruction)
    // This is the "sanctification ritual" - controlled access with intent
    stac();

    // Perform the copy
    let src = mortal_ptr.addr() as *const T;
    let result = core::ptr::read_volatile(src);
    *dest = result;

    // Re-enable SMAP protection (CLAC instruction)
    clac();

    Ok(())
}

/// Sanctified copy to mortal lands (copy_to_user)
///
/// Copies sanctified data from kernel space to user space.
///
/// # Arguments
///
/// * `src` - Kernel space source data
/// * `mortal_ptr` - Validated pointer to user space destination
///
/// # Returns
///
/// * `Ok(())` - Copy successful
/// * `Err(WardError)` - Copy failed
///
/// # Safety
///
/// - `src` must be a valid kernel space value
/// - `mortal_ptr` must be a valid MortalPointer with write permission
pub unsafe fn sanctified_copy_to_mortal<T: Copy>(
    src: &T,
    mortal_ptr: &MortalPointer<T>,
) -> Result<(), WardError> {
    // Temporarily disable SMAP (STAC instruction)
    stac();

    // Perform the copy
    let dest = mortal_ptr.addr() as *mut T;
    core::ptr::write_volatile(dest, *src);

    // Re-enable SMAP (CLAC instruction)
    clac();

    Ok(())
}

/// STAC (Set AC flag) - Temporarily allow access to user space
///
/// This is part of the sanctification ritual. The Heartwood explicitly declares
/// its intent to touch mortal lands, making the access controlled and logged.
///
/// # Safety
///
/// Must be paired with CLAC. Use only within sanctified_copy functions.
#[inline(always)]
unsafe fn stac() {
    // STAC instruction - sets AC flag in RFLAGS
    // This temporarily allows supervisor mode to access user pages (even with SMAP)
    asm!("stac", options(nomem, nostack));
}

/// CLAC (Clear AC flag) - Re-enable user space protection
///
/// Ends the sanctification ritual. The Heartwood withdraws from mortal lands
/// and re-establishes the sacred boundary.
///
/// # Safety
///
/// Must be called after STAC to restore protection.
#[inline(always)]
unsafe fn clac() {
    // CLAC instruction - clears AC flag in RFLAGS
    // This re-enables SMAP protection
    asm!("clac", options(nomem, nostack));
}

/// Copy a slice from mortal lands
///
/// Sanctifies an entire array from user space.
///
/// # Arguments
///
/// * `mortal_addr` - User space address of array start
/// * `dest` - Kernel space destination slice
///
/// # Returns
///
/// * `Ok(())` - All elements copied successfully
/// * `Err(WardError)` - Copy failed
pub unsafe fn sanctified_copy_slice_from_mortal<T: Copy>(
    mortal_addr: u64,
    dest: &mut [T],
) -> Result<(), WardError> {
    let size = dest.len() * size_of::<T>();
    validate_mortal_pointer(mortal_addr, size)?;

    stac();

    let src = mortal_addr as *const T;
    for (i, elem) in dest.iter_mut().enumerate() {
        *elem = core::ptr::read_volatile(src.add(i));
    }

    clac();

    Ok(())
}

/// Copy a slice to mortal lands
///
/// Sends sanctified data to user space.
///
/// # Arguments
///
/// * `src` - Kernel space source slice
/// * `mortal_addr` - User space destination address
///
/// # Returns
///
/// * `Ok(())` - All elements copied successfully
/// * `Err(WardError)` - Copy failed
pub unsafe fn sanctified_copy_slice_to_mortal<T: Copy>(
    src: &[T],
    mortal_addr: u64,
) -> Result<(), WardError> {
    let size = src.len() * size_of::<T>();
    validate_mortal_pointer(mortal_addr, size)?;

    stac();

    let dest = mortal_addr as *mut T;
    for (i, elem) in src.iter().enumerate() {
        core::ptr::write_volatile(dest.add(i), *elem);
    }

    clac();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mortal_pointer_validation() {
        // Valid user space pointer
        assert!(is_mortal_pointer(0x1000));
        assert!(is_mortal_pointer(0x0000_7FFF_0000_0000));

        // Invalid: kernel space
        assert!(!is_mortal_pointer(0xFFFF_8000_0000_0000));
        assert!(!is_mortal_pointer(0xFFFF_FFFF_FFFF_FFFF));

        // Invalid: null
        assert!(!is_mortal_pointer(0));
    }

    #[test]
    fn test_region_validation() {
        // Valid region entirely in user space
        assert!(validate_mortal_pointer(0x1000, 4096).is_ok());

        // Invalid: starts in user space but overflows
        assert!(validate_mortal_pointer(0x0000_7FFF_FFFF_F000, 0x2000).is_err());

        // Invalid: null pointer
        assert!(validate_mortal_pointer(0, 4096).is_err());

        // Invalid: kernel space
        assert!(validate_mortal_pointer(0xFFFF_8000_0000_0000, 4096).is_err());
    }
}
