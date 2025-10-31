//! # The Harbor - Vessel Management
//!
//! The Harbor is where all Vessels (processes) are moored. It maintains
//! the registry of all active Vessels and provides lookup capabilities.
//!
//! ## Philosophy
//! The Harbor is not just a process table - it's the sacred dockyard where
//! Vessels rest between voyages. Each Vessel has its own berth (slot) and
//! Beacon (unique identifier) that allows other parts of the system to find it.
//!
//! ## Architecture
//! - Centralized registry of all Vessels
//! - Fast lookup by VesselId
//! - Thread-to-Vessel mapping
//! - Vessel lifecycle management

use super::vessel::{Vessel, VesselId, VesselState};
use super::thread::ThreadId;
use alloc::vec::Vec;
use alloc::string::String;

/// The Harbor - Registry of all Vessels in the system
pub struct Harbor {
    /// All Vessels currently in the system
    vessels: Vec<Vessel>,

    /// Next available Beacon (VesselId) to assign
    next_beacon_id: u64,
}

impl Harbor {
    /// Create a new empty Harbor
    pub const fn new() -> Self {
        Self {
            vessels: Vec::new(),
            next_beacon_id: 1, // Start at 1 (0 reserved for kernel)
        }
    }

    /// Moor a new Vessel in the Harbor
    ///
    /// This creates a kernel-mode Vessel without user space.
    /// For user space Vessels, use `moor_user_vessel()` instead.
    ///
    /// # Arguments
    /// * `parent` - Parent VesselId (None for kernel Vessel)
    /// * `page_table_phys` - Physical address of PML4 page table
    /// * `kernel_stack` - Top of kernel stack for syscalls
    /// * `main_thread` - ThreadId of the main thread
    /// * `fate` - RBAC role from Concordance
    ///
    /// # Returns
    /// The VesselId of the newly created Vessel
    pub fn moor_vessel(
        &mut self,
        parent: Option<VesselId>,
        page_table_phys: u64,
        kernel_stack: u64,
        main_thread: ThreadId,
        fate: String,
    ) -> VesselId {
        let beacon = VesselId(self.next_beacon_id);
        self.next_beacon_id += 1;

        // Create empty address space for kernel Vessels
        let address_space = crate::mana_pool::UserAddressSpace::new()
            .expect("Failed to create kernel Vessel address space");

        let vessel = Vessel::new(
            beacon,
            parent,
            address_space,
            page_table_phys,
            0, // entry_point = 0 for kernel vessels
            kernel_stack,
            main_thread,
            fate,
        );

        self.vessels.push(vessel);
        beacon
    }

    /// Moor a new user-mode Vessel from an ELF binary
    ///
    /// This creates a user-mode Vessel with proper address space isolation.
    /// Use this for spawning user space processes.
    ///
    /// # Arguments
    /// * `parent` - Parent VesselId (None for init process)
    /// * `elf_data` - Raw ELF binary data
    /// * `fate` - RBAC role from Concordance
    /// * `main_thread` - ThreadId of the main thread
    ///
    /// # Returns
    /// * `Ok(VesselId)` - The VesselId of the newly created Vessel
    /// * `Err(&str)` - Error message if ELF loading fails
    pub fn moor_user_vessel(
        &mut self,
        parent: Option<VesselId>,
        elf_data: &[u8],
        fate: String,
        main_thread: ThreadId,
    ) -> Result<VesselId, &'static str> {
        let beacon = VesselId(self.next_beacon_id);
        self.next_beacon_id += 1;

        // Create Vessel from ELF
        let mut vessel = Vessel::from_elf(beacon, parent, elf_data, fate, main_thread)?;

        // The Vessel ID is already set by from_elf
        self.vessels.push(vessel);

        Ok(beacon)
    }

    /// Find a Vessel by its Beacon (VesselId)
    ///
    /// # Returns
    /// Some(&Vessel) if found, None otherwise
    pub fn find_vessel(&self, beacon: VesselId) -> Option<&Vessel> {
        self.vessels.iter().find(|v| v.beacon == beacon)
    }

    /// Find a Vessel mutably by its Beacon (VesselId)
    ///
    /// # Returns
    /// Some(&mut Vessel) if found, None otherwise
    pub fn find_vessel_mut(&mut self, beacon: VesselId) -> Option<&mut Vessel> {
        self.vessels.iter_mut().find(|v| v.beacon == beacon)
    }

    /// Find which Vessel owns a specific Thread
    ///
    /// # Arguments
    /// * `thread_id` - The ThreadId to search for
    ///
    /// # Returns
    /// Some(VesselId) if the thread's Vessel is found, None otherwise
    ///
    /// # Note
    /// Currently only checks the main_thread field. In Phase 3+,
    /// this will need to check all threads in the Vessel.
    pub fn find_vessel_by_thread(&self, thread_id: ThreadId) -> Option<VesselId> {
        self.vessels
            .iter()
            .find(|v| v.main_thread == thread_id)
            .map(|v| v.beacon)
    }

    /// Unmoor a Vessel from the Harbor (remove it)
    ///
    /// # Arguments
    /// * `beacon` - The VesselId to remove
    ///
    /// # Returns
    /// true if the Vessel was found and removed, false otherwise
    ///
    /// # Note
    /// This should only be called after all threads have been cleaned up
    /// and the Vessel is in Vanished state.
    pub fn unmoor_vessel(&mut self, beacon: VesselId) -> bool {
        if let Some(pos) = self.vessels.iter().position(|v| v.beacon == beacon) {
            self.vessels.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get the number of Vessels currently in the Harbor
    pub fn vessel_count(&self) -> usize {
        self.vessels.len()
    }

    /// Get all VesselIds currently in the Harbor
    pub fn all_beacons(&self) -> Vec<VesselId> {
        self.vessels.iter().map(|v| v.beacon).collect()
    }

    /// Count Vessels in a specific state
    pub fn count_by_state(&self, state: VesselState) -> usize {
        self.vessels.iter().filter(|v| v.state == state).count()
    }

    /// Get statistics about the Harbor
    pub fn stats(&self) -> HarborStats {
        HarborStats {
            total_vessels: self.vessel_count(),
            nascent: self.count_by_state(VesselState::Nascent),
            weaving: self.count_by_state(VesselState::Weaving),
            resting: self.count_by_state(VesselState::Resting),
            fading: self.count_by_state(VesselState::Fading),
            vanished: self.count_by_state(VesselState::Vanished),
        }
    }
}

/// Statistics about the Harbor
#[derive(Debug, Clone, Copy)]
pub struct HarborStats {
    pub total_vessels: usize,
    pub nascent: usize,
    pub weaving: usize,
    pub resting: usize,
    pub fading: usize,
    pub vanished: usize,
}
