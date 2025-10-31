//! # Per-CPU Data Structures
//!
//! Fast access to CPU-local data via the GS register.
//!
//! ## Philosophy
//!
//! Each CPU is a unique consciousness in the Heartwood, maintaining its own
//! thread of execution, kernel stack, and context. The GS register provides
//! instant access to this per-CPU data without locks or atomic operations.
//!
//! ## Architecture
//!
//! - **GS Register**: Points to PerCpuData structure for current CPU
//! - **GSBASE MSR (0xC0000101)**: Kernel GS base (used in kernel mode)
//! - **KERNEL_GSBASE MSR (0xC0000102)**: User GS base (swapped with swapgs)
//!
//! ## Usage
//!
//! ```rust
//! // Access current CPU's data
//! let cpu_data = per_cpu::get();
//! let kernel_stack = cpu_data.kernel_stack_top;
//! let current_thread = cpu_data.current_thread_id;
//! ```

use x86_64::registers::model_specific::{GsBase, KernelGsBase};
use x86_64::VirtAddr;
use crate::loom_of_fate::ThreadId;

/// Per-CPU data structure
///
/// This structure is accessed via the GS register for fast, lock-free
/// access to CPU-local data. Critical for syscall/sysret performance.
///
/// # Memory Layout
///
/// The structure is #[repr(C)] to ensure stable layout for assembly access.
/// Offsets are documented for use in naked functions.
///
/// **CRITICAL**: self_ptr MUST be at offset 0 because get()/get_mut() read gs:[0]
#[repr(C)]
pub struct PerCpuData {
    /// Pointer to this PerCpuData structure (used by get/get_mut)
    ///
    /// **Offset: 0 bytes** (for assembly: `gs:[0]`) - MUST BE FIRST!
    pub self_ptr: *const PerCpuData,

    /// Top of kernel stack for this CPU (used during syscall entry)
    ///
    /// **Offset: 8 bytes** (for assembly: `gs:[8]`)
    pub kernel_stack_top: u64,

    /// Saved user stack pointer during syscall
    ///
    /// **Offset: 16 bytes** (for assembly: `gs:[16]`)
    pub user_stack_saved: u64,

    /// Currently executing thread on this CPU
    ///
    /// **Offset: 24 bytes** (for assembly: `gs:[24]`)
    pub current_thread_id: Option<ThreadId>,

    /// The CPU ID (0-based index)
    pub cpu_id: u32,

    /// Interrupt nesting depth (for debugging)
    pub interrupt_depth: u32,

    /// Syscall count for this CPU (for profiling)
    pub syscall_count: u64,
}

impl PerCpuData {
    /// Create a new PerCpuData structure for a CPU
    ///
    /// # Arguments
    ///
    /// * `cpu_id` - The CPU ID (0-based)
    /// * `kernel_stack_top` - Top of the kernel stack for this CPU
    pub const fn new(cpu_id: u32, kernel_stack_top: u64) -> Self {
        Self {
            self_ptr: core::ptr::null(),
            kernel_stack_top,
            user_stack_saved: 0,
            current_thread_id: None,
            cpu_id,
            interrupt_depth: 0,
            syscall_count: 0,
        }
    }

    /// Initialize the self-pointer (must be called after allocation)
    pub fn init_self_ptr(&mut self) {
        self.self_ptr = self as *const _;
    }

    /// Validate that the per-CPU data is sane
    pub fn validate(&self) -> bool {
        // Check that self_ptr points to this structure
        self.self_ptr == (self as *const _)
    }
}

/// Static storage for per-CPU data (single CPU for now)
///
/// TODO: When SMP support is added, this will be an array indexed by CPU ID
static mut BSP_CPU_DATA: PerCpuData = PerCpuData::new(0, 0);

/// Dedicated kernel stack for syscall handling on BSP (64KB, statically allocated)
///
/// This stack is used when transitioning from user mode to kernel mode via syscall.
/// It's separate from the boot/thread stacks to prevent corruption.
/// Using a static array avoids heap allocation during early init.
#[repr(align(16))]
struct KernelStack {
    data: [u8; 65536], // 64KB stack
}

static mut BSP_KERNEL_STACK: KernelStack = KernelStack {
    data: [0; 65536],
};

/// Initialize per-CPU data for the bootstrap processor (BSP)
///
/// This sets up the GS register to point to the BSP's per-CPU data
/// and configures the dedicated kernel stack for syscall handling.
///
/// # Safety
///
/// Must be called exactly once during kernel initialization, on the BSP,
/// before any code that accesses per-CPU data or syscalls.
pub unsafe fn init_bsp() {
    // Calculate the top of the static kernel stack (stack grows down)
    let kernel_stack_bottom = core::ptr::addr_of!(BSP_KERNEL_STACK) as u64;
    let kernel_stack_size = core::mem::size_of::<KernelStack>();
    let kernel_stack_top = kernel_stack_bottom + kernel_stack_size as u64;

    // Initialize BSP per-CPU data with the dedicated kernel stack
    BSP_CPU_DATA = PerCpuData::new(0, kernel_stack_top);
    BSP_CPU_DATA.init_self_ptr();

    // Set GSBASE to point to BSP data
    let addr = VirtAddr::new(&BSP_CPU_DATA as *const _ as u64);
    GsBase::write(addr);

    // Set KERNEL_GSBASE to 0 initially (will be set to user GS during context switch)
    KernelGsBase::write(VirtAddr::new(0));

    crate::serial_println!("[PER_CPU] ✓ Bootstrap processor per-CPU data initialized");
    crate::serial_println!("[PER_CPU]   GS base: {:#x}", addr.as_u64());
    crate::serial_println!("[PER_CPU]   Kernel stack: {:#x} - {:#x} ({} KB)",
                          kernel_stack_bottom, kernel_stack_top, kernel_stack_size / 1024);
    crate::serial_println!("[PER_CPU]   ✓ Dedicated syscall stack configured (static allocation)");
}

/// Get a reference to the current CPU's data
///
/// # Safety
///
/// The GS register must be properly initialized via `init_bsp()` before
/// calling this function.
///
/// # Returns
///
/// A reference to the current CPU's PerCpuData structure
pub unsafe fn get() -> &'static PerCpuData {
    let ptr: *const PerCpuData;

    // Read GS:0 to get the address of the PerCpuData structure
    core::arch::asm!(
        "mov {}, gs:[0]",
        out(reg) ptr,
        options(nostack, preserves_flags, readonly),
    );

    &*ptr
}

/// Get a mutable reference to the current CPU's data
///
/// # Safety
///
/// The GS register must be properly initialized via `init_bsp()` before
/// calling this function. The caller must ensure exclusive access.
pub unsafe fn get_mut() -> &'static mut PerCpuData {
    let ptr: *mut PerCpuData;

    // Read GS:0 to get the address of the PerCpuData structure
    core::arch::asm!(
        "mov {}, gs:[0]",
        out(reg) ptr,
        options(nostack, preserves_flags),
    );

    &mut *ptr
}

/// Get the current CPU's kernel stack top
///
/// This is used during syscall entry to switch to the kernel stack.
///
/// # Safety
///
/// The GS register must be properly initialized.
#[inline(always)]
pub unsafe fn kernel_stack_top() -> u64 {
    let stack_top: u64;

    // Read GS:8 directly (faster than get().kernel_stack_top)
    core::arch::asm!(
        "mov {}, gs:[8]",
        out(reg) stack_top,
        options(nostack, preserves_flags, readonly),
    );

    stack_top
}

/// Get the current thread ID
///
/// # Safety
///
/// The GS register must be properly initialized.
#[inline(always)]
pub unsafe fn current_thread() -> Option<ThreadId> {
    get().current_thread_id
}

/// Set the current thread ID
///
/// # Safety
///
/// The GS register must be properly initialized.
#[inline(always)]
pub unsafe fn set_current_thread(thread_id: Option<ThreadId>) {
    get_mut().current_thread_id = thread_id;
}

/// Increment the syscall counter for profiling
///
/// # Safety
///
/// The GS register must be properly initialized.
#[inline(always)]
pub unsafe fn increment_syscall_count() {
    get_mut().syscall_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_cpu_data_layout() {
        use core::mem::{offset_of, size_of};

        // Verify struct layout for assembly access
        assert_eq!(offset_of!(PerCpuData, self_ptr), 0, "self_ptr MUST be at offset 0 for gs:[0] access");
        assert_eq!(offset_of!(PerCpuData, kernel_stack_top), 8);
        assert_eq!(offset_of!(PerCpuData, user_stack_saved), 16);
        assert_eq!(offset_of!(PerCpuData, current_thread_id), 24);

        // Verify alignment
        assert_eq!(size_of::<PerCpuData>() % 8, 0, "PerCpuData must be 8-byte aligned");
    }

    #[test]
    fn test_per_cpu_data_creation() {
        let data = PerCpuData::new(0, 0x1000);
        assert_eq!(data.cpu_id, 0);
        assert_eq!(data.kernel_stack_top, 0x1000);
        assert_eq!(data.user_stack_saved, 0);
        assert!(data.current_thread_id.is_none());
    }
}
