//! # x86_64 Page Table Management
//!
//! This module provides low-level access to x86_64 page tables for modifying
//! page permissions. Used by The Rune of Permanence to mark critical kernel
//! structures as read-only at the hardware level.
//!
//! ## x86_64 4-Level Paging Structure
//!
//! ```text
//! Virtual Address (48-bit):
//! ┌─────┬─────┬─────┬─────┬──────────┐
//! │ PML4│ PDPT│ PD  │ PT  │  Offset  │
//! │ 9bit│ 9bit│ 9bit│ 9bit│  12bit   │
//! │47-39│38-30│29-21│20-12│   11-0   │
//! └─────┴─────┴─────┴─────┴──────────┘
//!
//! CR3 → PML4[511:0] → PDPT[511:0] → PD[511:0] → PT[511:0] → 4KB Page
//! ```
//!
//! ## Page Table Entry Format (64-bit)
//!
//! ```text
//! ┌───────────────────────────────────────────────┐
//! │ 63  │ 52-12                │ 11-0              │
//! │ NX  │ Physical Address     │ Flags             │
//! └───────────────────────────────────────────────┘
//!
//! Flags:
//!   Bit 0: P   (Present)
//!   Bit 1: RW  (Read/Write) ← We clear this for read-only!
//!   Bit 2: US  (User/Supervisor)
//!   Bit 3: PWT (Page-level Write-Through)
//!   Bit 4: PCD (Page-level Cache Disable)
//!   Bit 5: A   (Accessed)
//!   Bit 6: D   (Dirty)
//!   Bit 7: PS  (Page Size) - 1 = huge page (2MB or 1GB)
//!   Bit 8: G   (Global)
//! ```

/// Page table entry flags
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum PageFlag {
    Present = 1 << 0,
    ReadWrite = 1 << 1,
    UserSupervisor = 1 << 2,
    WriteThrough = 1 << 3,
    CacheDisable = 1 << 4,
    Accessed = 1 << 5,
    Dirty = 1 << 6,
    HugePage = 1 << 7,  // 2MB page in PD, 1GB page in PDPT
    Global = 1 << 8,
}

/// Page table entry (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    /// Check if the entry is present
    #[inline]
    pub fn is_present(&self) -> bool {
        self.entry & PageFlag::Present as u64 != 0
    }

    /// Check if this is a huge page (2MB or 1GB)
    #[inline]
    pub fn is_huge(&self) -> bool {
        self.entry & PageFlag::HugePage as u64 != 0
    }

    /// Check if the page is writable
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.entry & PageFlag::ReadWrite as u64 != 0
    }

    /// Get the physical address this entry points to (page-aligned)
    #[inline]
    pub fn address(&self) -> u64 {
        self.entry & 0x000F_FFFF_FFFF_F000
    }

    /// Set the read/write flag
    #[inline]
    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.entry |= PageFlag::ReadWrite as u64;
        } else {
            self.entry &= !(PageFlag::ReadWrite as u64);
        }
    }

    /// Get the raw entry value
    #[inline]
    pub fn raw(&self) -> u64 {
        self.entry
    }

    /// Set the raw entry value
    #[inline]
    pub fn set_raw(&mut self, value: u64) {
        self.entry = value;
    }
}

/// A page table (512 entries)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Get an entry by index
    #[inline]
    pub fn entry(&self, index: usize) -> PageTableEntry {
        self.entries[index]
    }

    /// Get a mutable entry by index
    #[inline]
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}

/// Extract the page table index for a given level from a virtual address
#[inline]
fn page_table_index(addr: u64, level: u8) -> usize {
    match level {
        4 => ((addr >> 39) & 0x1FF) as usize,  // PML4 index (bits 47-39)
        3 => ((addr >> 30) & 0x1FF) as usize,  // PDPT index (bits 38-30)
        2 => ((addr >> 21) & 0x1FF) as usize,  // PD index (bits 29-21)
        1 => ((addr >> 12) & 0x1FF) as usize,  // PT index (bits 20-12)
        _ => {
            // SAFETY: This function is only called with level 1-4 from within this module.
            // Invalid level is a programming error, not a runtime condition.
            // Output error to serial and halt (avoid panic! which allocates)
            unsafe {
                let msg = b"\n[FATAL] Invalid page table level!\n";
                for &byte in msg.iter() {
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3f8u16,
                        in("al") byte,
                        options(nomem, nostack, preserves_flags)
                    );
                }
                loop {
                    core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
                }
            }
        }
    }
}

/// Get the current CR3 value (physical address of PML4)
#[inline]
fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3,
            options(nomem, nostack, preserves_flags)
        );
    }
    cr3 & 0xFFFF_FFFF_FFFF_F000  // Mask off lower 12 bits
}

/// Walk the page tables to find the page table entry for a virtual address
///
/// Returns None if:
/// - Any level is not present
/// - A huge page is encountered before reaching PT level
///
/// Returns Some((pt_entry, pt_address, pt_index)) on success, where:
/// - pt_entry: The page table entry for this address
/// - pt_address: Physical address of the page table containing the entry
/// - pt_index: Index within that page table
pub unsafe fn walk_page_tables(virt_addr: u64) -> Option<(PageTableEntry, u64, usize)> {
    // Get PML4 base from CR3
    let pml4_phys = read_cr3();
    let pml4 = &*(phys_to_virt(pml4_phys) as *const PageTable);

    // Level 4: PML4
    let pml4_idx = page_table_index(virt_addr, 4);
    let pml4_entry = pml4.entry(pml4_idx);
    if !pml4_entry.is_present() {
        return None;
    }

    // Level 3: PDPT
    let pdpt_phys = pml4_entry.address();
    let pdpt = &*(phys_to_virt(pdpt_phys) as *const PageTable);
    let pdpt_idx = page_table_index(virt_addr, 3);
    let pdpt_entry = pdpt.entry(pdpt_idx);
    if !pdpt_entry.is_present() {
        return None;
    }
    if pdpt_entry.is_huge() {
        // 1GB huge page - can't modify at 4KB granularity
        return None;
    }

    // Level 2: PD
    let pd_phys = pdpt_entry.address();
    let pd = &*(phys_to_virt(pd_phys) as *const PageTable);
    let pd_idx = page_table_index(virt_addr, 2);
    let pd_entry = pd.entry(pd_idx);
    if !pd_entry.is_present() {
        return None;
    }
    if pd_entry.is_huge() {
        // 2MB huge page - need to split it into 4KB pages
        // For now, return None (Phase 2.1 will implement splitting)
        return None;
    }

    // Level 1: PT
    let pt_phys = pd_entry.address();
    let pt = &*(phys_to_virt(pt_phys) as *const PageTable);
    let pt_idx = page_table_index(virt_addr, 1);
    let pt_entry = pt.entry(pt_idx);

    Some((pt_entry, pt_phys, pt_idx))
}

/// Make a 2MB huge page read-only by modifying the PD entry
///
/// This modifies the PD (Page Directory) entry directly to make the entire
/// 2MB huge page read-only.
///
/// Returns the number of huge pages modified (typically 1)
unsafe fn make_huge_page_readonly(virt_addr: u64) -> Result<usize, &'static str> {
    // Get PML4 base from CR3
    let pml4_phys = read_cr3();
    let pml4 = &*(phys_to_virt(pml4_phys) as *const PageTable);

    // Level 4: PML4
    let pml4_idx = page_table_index(virt_addr, 4);
    let pml4_entry = pml4.entry(pml4_idx);
    if !pml4_entry.is_present() {
        return Err("PML4 entry not present");
    }

    // Level 3: PDPT
    let pdpt_phys = pml4_entry.address();
    let pdpt = &*(phys_to_virt(pdpt_phys) as *const PageTable);
    let pdpt_idx = page_table_index(virt_addr, 3);
    let pdpt_entry = pdpt.entry(pdpt_idx);
    if !pdpt_entry.is_present() {
        return Err("PDPT entry not present");
    }
    if pdpt_entry.is_huge() {
        return Err("1GB huge pages not supported");
    }

    // Level 2: PD - this is where the 2MB huge page entry is
    let pd_phys = pdpt_entry.address();
    let pd = &mut *(phys_to_virt(pd_phys) as *mut PageTable);
    let pd_idx = page_table_index(virt_addr, 2);
    let pd_entry = pd.entry_mut(pd_idx);

    if !pd_entry.is_present() {
        return Err("PD entry not present");
    }
    if !pd_entry.is_huge() {
        return Err("Not a huge page");
    }

    // Make the huge page read-only
    pd_entry.set_writable(false);

    Ok(1)
}

/// Make a virtual address range read-only
///
/// This clears the RW bit in the page table entries for all pages
/// in the specified range. Handles both 4KB pages and 2MB huge pages.
///
/// # Safety
/// - Must only be called after the memory has been initialized
/// - The range must be page-aligned
/// - Any writes to this range after calling this will cause a page fault
///
/// # Note
/// If the range spans 2MB huge pages, the ENTIRE 2MB page will be made
/// read-only, not just the portion within the range.
///
/// # Panics
/// - If the range is not page-aligned
pub unsafe fn make_readonly(start: u64, end: u64) -> Result<usize, &'static str> {
    // Validate page alignment
    if start % 0x1000 != 0 {
        return Err("Start address not page-aligned");
    }

    let mut addr = start & !0xFFF;  // Align down
    let mut pages_modified = 0;

    while addr < end {
        // Walk to the page table entry
        match walk_page_tables(addr) {
            Some((entry, pt_phys, pt_idx)) => {
                if !entry.is_present() {
                    return Err("Page not present");
                }

                // Get mutable access to the page table (convert physical to virtual)
                let pt = &mut *(phys_to_virt(pt_phys) as *mut PageTable);
                let pt_entry = pt.entry_mut(pt_idx);

                // Clear the RW bit to make read-only
                pt_entry.set_writable(false);

                pages_modified += 1;
                addr += 0x1000;  // Move to next 4KB page
            }
            None => {
                // Hit a huge page - modify it at PD level
                make_huge_page_readonly(addr)?;
                pages_modified += 1;

                // Skip to next 2MB boundary
                addr = (addr & !0x1F_FFFF) + 0x20_0000;
            }
        }
    }

    Ok(pages_modified)
}

/// Flush the Translation Lookaside Buffer (TLB)
///
/// This forces the CPU to reload page table entries from memory,
/// ensuring that page permission changes take effect immediately.
#[inline]
pub fn flush_tlb() {
    unsafe {
        // Reloading CR3 flushes the entire TLB
        core::arch::asm!(
            "mov {tmp}, cr3",
            "mov cr3, {tmp}",
            tmp = out(reg) _,
            options(nostack, preserves_flags)
        );
    }
}

/// Display information about a virtual address's page table mapping
///
/// Useful for debugging page table issues.
pub unsafe fn debug_page_mapping(virt_addr: u64) {
    crate::println!("Page mapping for 0x{:016x}:", virt_addr);

    let pml4_phys = read_cr3();
    crate::println!("  PML4 @ 0x{:016x}", pml4_phys);

    let pml4 = &*(phys_to_virt(pml4_phys) as *const PageTable);
    let pml4_idx = page_table_index(virt_addr, 4);
    let pml4_entry = pml4.entry(pml4_idx);
    crate::println!("    [{}] = 0x{:016x} ({})",
        pml4_idx, pml4_entry.raw(),
        if pml4_entry.is_present() { "present" } else { "not present" });

    if !pml4_entry.is_present() {
        return;
    }

    let pdpt_phys = pml4_entry.address();
    crate::println!("  PDPT @ 0x{:016x}", pdpt_phys);

    let pdpt = &*(phys_to_virt(pdpt_phys) as *const PageTable);
    let pdpt_idx = page_table_index(virt_addr, 3);
    let pdpt_entry = pdpt.entry(pdpt_idx);
    crate::println!("    [{}] = 0x{:016x} ({}, {})",
        pdpt_idx, pdpt_entry.raw(),
        if pdpt_entry.is_present() { "present" } else { "not present" },
        if pdpt_entry.is_huge() { "1GB page" } else { "normal" });

    if !pdpt_entry.is_present() || pdpt_entry.is_huge() {
        return;
    }

    let pd_phys = pdpt_entry.address();
    crate::println!("  PD @ 0x{:016x}", pd_phys);

    let pd = &*(phys_to_virt(pd_phys) as *const PageTable);
    let pd_idx = page_table_index(virt_addr, 2);
    let pd_entry = pd.entry(pd_idx);
    crate::println!("    [{}] = 0x{:016x} ({}, {}, {})",
        pd_idx, pd_entry.raw(),
        if pd_entry.is_present() { "present" } else { "not present" },
        if pd_entry.is_huge() { "2MB page" } else { "normal" },
        if pd_entry.is_writable() { "RW" } else { "RO" });

    if !pd_entry.is_present() || pd_entry.is_huge() {
        return;
    }

    let pt_phys = pd_entry.address();
    crate::println!("  PT @ 0x{:016x}", pt_phys);

    let pt = &*(phys_to_virt(pt_phys) as *const PageTable);
    let pt_idx = page_table_index(virt_addr, 1);
    let pt_entry = pt.entry(pt_idx);
    crate::println!("    [{}] = 0x{:016x} ({}, {})",
        pt_idx, pt_entry.raw(),
        if pt_entry.is_present() { "present" } else { "not present" },
        if pt_entry.is_writable() { "RW" } else { "RO" });
}

/// Convert a virtual address to a physical address
///
/// This is a simple identity mapping in the kernel's higher half.
/// Virtual addresses >= 0xFFFF_8000_0000_0000 are direct-mapped to physical memory.
///
/// # Safety
/// This assumes the kernel uses identity mapping in the higher half.
#[inline]
unsafe fn virt_to_phys(virt: u64) -> u64 {
    // For kernel addresses in higher half, subtract the kernel base
    // AethelOS uses identity mapping: kernel virtual = physical
    // (This is set up by the bootloader)
    if virt >= 0xFFFFFFFF80000000 {
        virt - 0xFFFFFFFF80000000
    } else {
        // For lower-half addresses, assume identity mapping
        virt
    }
}

/// Convert a physical address to a kernel virtual address
///
/// After higher-half kernel migration, ALL kernel memory access uses top 2GB mapping.
/// Physical memory is mapped at virtual address 0xFFFFFFFF80000000+.
///
/// This includes:
/// - Boot page tables at physical 0x70000 → virtual 0xFFFFFFFF80070000
/// - Kernel code/data at physical 0x100000+ → virtual 0xFFFFFFFF80100000+
/// - Kernel heap at physical 0x400000+ → virtual 0xFFFFFFFF80400000+
/// - Any other kernel-allocated memory
///
/// # Safety
/// This assumes the top 2GB mapping (PML4 entry 511) is active and maps
/// the first 1GB of physical memory to 0xFFFFFFFF80000000+.
#[inline]
unsafe fn phys_to_virt(phys: u64) -> u64 {
    const KERNEL_BASE: u64 = 0xFFFFFFFF80000000;
    KERNEL_BASE + phys
}

/// Allocate a new page table (zero-initialized)
///
/// Returns the physical address of the new page table.
///
/// # Safety
///
/// The page table is leaked and will never be freed.
/// This is acceptable for Phase 2; proper cleanup will be implemented in Phase 3+.
unsafe fn allocate_page_table() -> Result<u64, &'static str> {
    use alloc::boxed::Box;

    // Allocate a new page table (4096-byte aligned)
    let new_table = Box::new(PageTable {
        entries: [PageTableEntry { entry: 0 }; 512],
    });

    // Get the physical address
    let table_virt = &*new_table as *const PageTable as u64;
    let table_phys = virt_to_phys(table_virt);

    // Leak the Box to keep it alive
    core::mem::forget(new_table);

    Ok(table_phys)
}

/// Map a virtual page to a physical frame in user space
///
/// Creates page table structures (PT, PD, PDPT) as needed and inserts
/// a new entry mapping virt_addr → phys_addr with the specified flags.
///
/// # Arguments
///
/// * `pml4_phys` - Physical address of the PML4 (CR3 value)
/// * `virt_addr` - Virtual address to map (must be page-aligned and in user space)
/// * `phys_addr` - Physical address to map to (must be page-aligned)
/// * `flags` - Page table flags (raw u64, compatible with PageTableFlags::bits())
///
/// # Returns
///
/// * `Ok(())` - Mapping successful
/// * `Err(&str)` - Mapping failed
///
/// # Safety
///
/// - The caller must ensure pml4_phys points to a valid PML4
/// - The caller must ensure phys_addr points to a valid physical frame
/// - The caller must ensure virt_addr is not already mapped (or be prepared for remapping)
/// - The caller must flush TLB after mapping if needed (using flush_tlb() or by reloading CR3)
///
/// # Note
///
/// This function allocates intermediate page tables (PT, PD, PDPT) as needed
/// using Box::leak(), which means they will never be freed. This is acceptable
/// for Phase 2; proper cleanup will be implemented in Phase 3+.
///
/// # Example
///
/// ```ignore
/// // Map virtual address 0x400000 to physical frame 0x200000 with user access
/// let flags = PageTableFlags::PRESENT
///     | PageTableFlags::WRITABLE
///     | PageTableFlags::USER_ACCESSIBLE;
/// unsafe {
///     map_user_page(vessel_cr3, 0x400000, 0x200000, flags.bits())?;
///     flush_tlb();  // Ensure TLB is updated
/// }
/// ```
pub unsafe fn map_user_page(
    pml4_phys: u64,
    virt_addr: u64,
    phys_addr: u64,
    flags: u64,
) -> Result<(), &'static str> {
    // Validate that virtual address is in user space (lower half)
    if virt_addr >= 0x0000_8000_0000_0000 {
        return Err("Virtual address is in kernel space");
    }

    // Validate page alignment
    if virt_addr % 0x1000 != 0 {
        return Err("Virtual address not page-aligned");
    }
    if phys_addr % 0x1000 != 0 {
        return Err("Physical address not page-aligned");
    }

    // Flags for intermediate page tables (PRESENT | WRITABLE | USER_ACCESSIBLE)
    let intermediate_flags = (PageFlag::Present as u64)
        | (PageFlag::ReadWrite as u64)
        | (PageFlag::UserSupervisor as u64);

    // Get the PML4 (convert physical to virtual address)
    let pml4 = &mut *(phys_to_virt(pml4_phys) as *mut PageTable);

    // Level 4: PML4
    let pml4_idx = page_table_index(virt_addr, 4);
    let pml4_entry = pml4.entry_mut(pml4_idx);

    // If PML4 entry not present, allocate a new PDPT
    let pdpt_phys = if !pml4_entry.is_present() {
        let new_pdpt_phys = allocate_page_table()?;
        pml4_entry.set_raw(new_pdpt_phys | intermediate_flags);
        new_pdpt_phys
    } else {
        pml4_entry.address()
    };

    // Level 3: PDPT (convert physical to virtual address)
    let pdpt = &mut *(phys_to_virt(pdpt_phys) as *mut PageTable);
    let pdpt_idx = page_table_index(virt_addr, 3);
    let pdpt_entry = pdpt.entry_mut(pdpt_idx);

    // If PDPT entry not present, allocate a new PD
    let pd_phys = if !pdpt_entry.is_present() {
        let new_pd_phys = allocate_page_table()?;
        pdpt_entry.set_raw(new_pd_phys | intermediate_flags);
        new_pd_phys
    } else {
        if pdpt_entry.is_huge() {
            return Err("1GB huge page already mapped at this address");
        }
        pdpt_entry.address()
    };

    // Level 2: PD (convert physical to virtual address)
    let pd = &mut *(phys_to_virt(pd_phys) as *mut PageTable);
    let pd_idx = page_table_index(virt_addr, 2);
    let pd_entry = pd.entry_mut(pd_idx);

    // If PD entry not present, allocate a new PT
    let pt_phys = if !pd_entry.is_present() {
        let new_pt_phys = allocate_page_table()?;
        pd_entry.set_raw(new_pt_phys | intermediate_flags);
        new_pt_phys
    } else {
        if pd_entry.is_huge() {
            return Err("2MB huge page already mapped at this address");
        }
        pd_entry.address()
    };

    // Level 1: PT (convert physical to virtual address)
    let pt = &mut *(phys_to_virt(pt_phys) as *mut PageTable);
    let pt_idx = page_table_index(virt_addr, 1);
    let pt_entry = pt.entry_mut(pt_idx);

    // Check if page is already mapped
    if pt_entry.is_present() {
        return Err("Page already mapped");
    }

    // Set the page table entry
    pt_entry.set_raw(phys_addr | flags);

    // DEBUG: Verify the entry was set correctly
    crate::serial_println!("[MAP] virt={:#x} -> phys={:#x}, PT[{}]={:#x}",
        virt_addr, phys_addr, pt_idx, pt_entry.raw());

    Ok(())
}

/// Clone the kernel's page tables for a new Vessel
///
/// Creates a new PML4 with:
/// - Upper half (indices 256-511): Kernel mappings (shared across all Vessels)
/// - Lower half (indices 0-255): Empty (for user space)
///
/// # Returns
/// Physical address of the new PML4 (suitable for loading into CR3)
///
/// # Safety
/// - Must be called after the heap allocator is initialized
/// - The returned page table must eventually be freed to prevent memory leaks
/// - The caller must ensure the page table is properly populated before use
///
/// # Note
/// This function uses Box to allocate the page table, which means:
/// - The page table will be freed when the Box is dropped
/// - The caller must use Box::leak() or similar to keep it alive
/// - For Phase 2, we accept this memory leak; Phase 3+ will implement proper cleanup
pub unsafe fn clone_kernel_page_table() -> Result<u64, &'static str> {
    use alloc::boxed::Box;

    // Allocate a new PML4 (4096-byte aligned page table)
    let mut new_pml4 = Box::new(PageTable {
        entries: [PageTableEntry { entry: 0 }; 512],
    });

    // Get the current kernel PML4
    let kernel_pml4_phys = read_cr3();
    crate::serial_println!("[CLONE_PML4] Kernel CR3 (PML4 phys): {:#x}", kernel_pml4_phys);

    // CRITICAL: Must convert physical address to virtual before dereferencing!
    let kernel_pml4_virt = phys_to_virt(kernel_pml4_phys);
    crate::serial_println!("[CLONE_PML4] Kernel PML4 virt addr: {:#x}", kernel_pml4_virt);
    let kernel_pml4 = &*(kernel_pml4_virt as *const PageTable);

    // Copy the top 2GB kernel entry [511] to user page table
    // This maps kernel space at 0xFFFFFFFF80000000+
    crate::serial_println!("[CLONE_PML4] Copying kernel entry [511] to user page table...");
    let entry511 = kernel_pml4.entry(511);
    if entry511.is_present() {
        crate::serial_println!("[CLONE_PML4]   Entry[511] = {:#x} (present, copying)", entry511.raw());
        *new_pml4.entry_mut(511) = entry511;
    } else {
        crate::serial_println!("[CLONE_PML4]   WARNING: Entry[511] not present in kernel PML4!");
    }

    // Copy the recursive mapping entry [510] from kernel PML4
    // This is CRITICAL - without it, MMU cannot walk page tables after CR3 switch
    crate::serial_println!("[CLONE_PML4] Checking kernel's recursive entry [510]...");
    let entry510 = kernel_pml4.entry(510);
    if entry510.is_present() {
        crate::serial_println!("[CLONE_PML4]   Kernel PML4[510] = {:#x} (recursive mapping exists)", entry510.raw());
        crate::serial_println!("[CLONE_PML4]   Will override with user PML4's own recursive mapping");
    } else {
        crate::serial_println!("[CLONE_PML4]   WARNING: Kernel PML4[510] not set up (no recursive mapping)!");
    }

    // Get the physical address of the new PML4 BEFORE setting up recursive mapping
    let new_pml4_virt = &*new_pml4 as *const PageTable as u64;
    let new_pml4_phys = virt_to_phys(new_pml4_virt);

    crate::serial_println!("[CLONE_PML4] New PML4 virt addr: {:#x}", new_pml4_virt);
    crate::serial_println!("[CLONE_PML4] New PML4 phys addr: {:#x}", new_pml4_phys);

    // CRITICAL: Set up recursive page table mapping at PML4[510]
    // This allows the MMU to access page table structures themselves through virtual addresses
    // Without this, after CR3 switch, the MMU cannot walk the page tables!
    crate::serial_println!("[CLONE_PML4] Setting up recursive mapping at PML4[510]...");
    new_pml4.entry_mut(510).entry = new_pml4_phys |
        (PageFlag::Present as u64) |
        (PageFlag::ReadWrite as u64);
    crate::serial_println!("[CLONE_PML4]   PML4[510] = {:#x} (recursive, points to self)", new_pml4.entry(510).raw());

    // Lower entries (indices 0-509) are already zeroed (user space, initially empty)
    crate::serial_println!("[CLONE_PML4] Entries [0-509] left empty for user space");
    crate::serial_println!("[CLONE_PML4] ✓ Clone complete");

    // Leak the Box to prevent it from being freed
    // The page table must remain alive for the lifetime of the Vessel
    // TODO(phase-3): Implement proper cleanup when Vessel is destroyed
    core::mem::forget(new_pml4);

    Ok(new_pml4_phys)
}
