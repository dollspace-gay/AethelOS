//! # Vessel - Process Abstraction
//!
//! In AethelOS, a Vessel is the container for threads - what other systems
//! call a "process". Each Vessel has its own address space (CR3), capabilities,
//! and role-based permissions from the Concordance of Fates.
//!
//! ## Philosophy
//! A Vessel is not just a process - it's a sacred container that holds threads
//! and their shared destiny. Each Vessel has a Beacon (unique ID) that identifies
//! it across the Harbor (process table).
//!
//! ## Architecture
//! - One Vessel can contain multiple Threads
//! - Each Vessel has isolated address space (separate page tables)
//! - Each Vessel has a kernel stack for system call handling
//! - Vessels inherit their parent's Fate (RBAC role) or are assigned one
//!
//! ## Lifecycle
//! Nascent → Weaving → Resting/Fading → Vanished

use super::thread::ThreadId;
use alloc::string::String;
use alloc::alloc::{alloc, Layout};
use crate::mana_pool::{UserAddressSpace, create_address_space_from_elf};

/// Size of kernel stack for syscall handling (16 KB)
///
/// This is smaller than thread stacks because it's only used during
/// syscall execution, not for the entire thread lifetime.
pub const KERNEL_STACK_SIZE: usize = 16 * 1024;

/// Allocate a kernel stack for syscall handling
///
/// Returns the top of the stack (stack grows downward).
///
/// # Returns
///
/// * `Ok(u64)` - Stack top address (RSP value for TSS)
/// * `Err(&str)` - Allocation failed
///
/// # Safety
///
/// The allocated stack is never freed (leaked). This is acceptable for Phase 2;
/// proper cleanup will be implemented in Phase 3+ when Vessels are destroyed.
fn allocate_kernel_stack() -> Result<u64, &'static str> {
    // Align to 16 bytes (required by x86-64 calling convention)
    let size = (KERNEL_STACK_SIZE + 15) & !15;

    // Create layout for allocation
    let layout = Layout::from_size_align(size, 16)
        .map_err(|_| "Invalid layout for kernel stack")?;

    // Allocate the stack
    let ptr = unsafe { alloc(layout) };

    if ptr.is_null() {
        return Err("Failed to allocate kernel stack");
    }

    // Stack grows downward, so return the top address
    let stack_top = ptr as u64 + size as u64;

    crate::serial_println!("[VESSEL] Allocated kernel stack: {:#x} - {:#x} (size: {:#x})",
        ptr as u64, stack_top, size);

    Ok(stack_top)
}

/// A unique identifier for a Vessel (process)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VesselId(pub u64);

/// The state of a Vessel in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VesselState {
    /// Vessel is being created (initial state)
    Nascent,

    /// Vessel is actively running (has at least one Weaving thread)
    Weaving,

    /// Vessel is blocked/idle (all threads are Resting)
    Resting,

    /// Vessel is in the process of exiting (cleaning up)
    Fading,

    /// Vessel has exited but not yet reaped (zombie state)
    Vanished,
}

/// A Vessel - The process abstraction of AethelOS
///
/// Each Vessel contains one or more threads and has:
/// - Isolated address space (own page tables)
/// - Kernel stack for system call handling
/// - RBAC role from Concordance of Fates
/// - Parent-child relationship with other Vessels
pub struct Vessel {
    /// The Beacon - unique identifier for this Vessel
    pub beacon: VesselId,

    /// Parent Vessel that spawned this one (None for kernel Vessel)
    pub parent: Option<VesselId>,

    /// The user address space - manages virtual memory regions
    pub address_space: UserAddressSpace,

    /// Physical address of this Vessel's PML4 page table (CR3 value)
    pub page_table_phys: u64,

    /// Entry point address (from ELF file)
    pub entry_point: u64,

    /// Kernel stack top for TSS.rsp[0] during ring 3→0 transitions
    pub kernel_stack: u64,

    /// The main thread of this Vessel (first thread spawned)
    pub main_thread: ThreadId,

    /// The Fate assigned to this Vessel (RBAC role from Concordance)
    pub fate: String,

    /// Current state of the Vessel
    pub state: VesselState,
}

impl Vessel {
    /// Create a new Vessel
    ///
    /// # Arguments
    /// * `beacon` - Unique VesselId
    /// * `parent` - Parent VesselId (None for kernel Vessel)
    /// * `address_space` - The user address space
    /// * `page_table_phys` - Physical address of PML4 page table
    /// * `entry_point` - Program entry point address
    /// * `kernel_stack` - Top of kernel stack for syscalls
    /// * `main_thread` - ThreadId of the main thread
    /// * `fate` - RBAC role from Concordance
    ///
    /// # Returns
    /// A new Vessel in Nascent state
    pub fn new(
        beacon: VesselId,
        parent: Option<VesselId>,
        address_space: UserAddressSpace,
        page_table_phys: u64,
        entry_point: u64,
        kernel_stack: u64,
        main_thread: ThreadId,
        fate: String,
    ) -> Self {
        Self {
            beacon,
            parent,
            address_space,
            page_table_phys,
            entry_point,
            kernel_stack,
            main_thread,
            fate,
            state: VesselState::Nascent,
        }
    }

    /// Get the Vessel's unique identifier
    pub fn id(&self) -> VesselId {
        self.beacon
    }

    /// Get the Vessel's current state
    pub fn state(&self) -> VesselState {
        self.state
    }

    /// Set the Vessel's state
    pub fn set_state(&mut self, state: VesselState) {
        self.state = state;
    }

    /// Check if this Vessel is in user mode (ring 3)
    ///
    /// A Vessel is in user mode if it has a non-kernel address space.
    /// The kernel Vessel always has page_table_phys == 0 (uses kernel page tables).
    pub fn is_user_mode(&self) -> bool {
        self.page_table_phys != 0
    }

    /// Get the entry point address
    pub fn entry_point(&self) -> u64 {
        self.entry_point
    }

    /// Get a reference to the address space
    pub fn address_space(&self) -> &UserAddressSpace {
        &self.address_space
    }

    /// Get a mutable reference to the address space
    pub fn address_space_mut(&mut self) -> &mut UserAddressSpace {
        &mut self.address_space
    }

    /// Get the parent VesselId if it exists
    pub fn parent(&self) -> Option<VesselId> {
        self.parent
    }

    /// Get the Vessel's Fate (RBAC role)
    pub fn fate(&self) -> &str {
        &self.fate
    }

    /// Get the main thread's ID
    pub fn main_thread(&self) -> ThreadId {
        self.main_thread
    }

    /// Get the kernel stack top (for TSS.rsp[0])
    pub fn kernel_stack(&self) -> u64 {
        self.kernel_stack
    }

    /// Get the page table physical address (CR3 value)
    pub fn page_table_phys(&self) -> u64 {
        self.page_table_phys
    }

    /// Create a Vessel from an ELF binary
    ///
    /// This is a factory method that:
    /// 1. Parses the ELF file
    /// 2. Creates an isolated address space with all segments mapped
    /// 3. Allocates a user stack
    /// 4. Allocates a kernel stack for syscall handling
    /// 5. Creates the Vessel structure
    ///
    /// # Arguments
    ///
    /// * `beacon` - Unique VesselId for this Vessel
    /// * `parent` - Parent VesselId (None for init process)
    /// * `elf_data` - Raw ELF binary data
    /// * `fate` - RBAC role from Concordance
    /// * `main_thread` - ThreadId of the main thread (must be created separately)
    ///
    /// # Returns
    ///
    /// * `Ok(Vessel)` - A new Vessel ready to execute
    /// * `Err(&str)` - Error message if ELF loading fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let vessel = Vessel::from_elf(
    ///     VesselId(1),
    ///     None,
    ///     elf_binary_data,
    ///     "user".to_string(),
    ///     ThreadId(42),
    /// )?;
    /// ```
    pub fn from_elf(
        beacon: VesselId,
        parent: Option<VesselId>,
        elf_data: &[u8],
        fate: String,
        main_thread: ThreadId,
    ) -> Result<Self, &'static str> {
        crate::serial_println!("[VESSEL] Creating Vessel {} from ELF", beacon.0);

        // Parse ELF and create address space
        let (address_space, entry_point) = create_address_space_from_elf(elf_data)?;

        // Get the PML4 physical address (CR3 value)
        let page_table_phys = address_space.pml4_phys.as_u64();

        // Allocate kernel stack for syscall handling
        let kernel_stack = allocate_kernel_stack()?;

        crate::serial_println!(
            "[VESSEL] ✓ Vessel {} created: entry={:#x}, CR3={:#x}, kernel_stack={:#x}",
            beacon.0,
            entry_point,
            page_table_phys,
            kernel_stack
        );

        Ok(Self::new(
            beacon,
            parent,
            address_space,
            page_table_phys,
            entry_point,
            kernel_stack,
            main_thread,
            fate,
        ))
    }
}
