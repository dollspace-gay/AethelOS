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
        _ => panic!("Invalid page table level: {}", level),
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
    let pml4 = &*(pml4_phys as *const PageTable);

    // Level 4: PML4
    let pml4_idx = page_table_index(virt_addr, 4);
    let pml4_entry = pml4.entry(pml4_idx);
    if !pml4_entry.is_present() {
        return None;
    }

    // Level 3: PDPT
    let pdpt_phys = pml4_entry.address();
    let pdpt = &*(pdpt_phys as *const PageTable);
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
    let pd = &*(pd_phys as *const PageTable);
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
    let pt = &*(pt_phys as *const PageTable);
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
    let pml4 = &*(pml4_phys as *const PageTable);

    // Level 4: PML4
    let pml4_idx = page_table_index(virt_addr, 4);
    let pml4_entry = pml4.entry(pml4_idx);
    if !pml4_entry.is_present() {
        return Err("PML4 entry not present");
    }

    // Level 3: PDPT
    let pdpt_phys = pml4_entry.address();
    let pdpt = &*(pdpt_phys as *const PageTable);
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
    let pd = &mut *(pd_phys as *mut PageTable);
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

                // Get mutable access to the page table
                let pt = &mut *(pt_phys as *mut PageTable);
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

    let pml4 = &*(pml4_phys as *const PageTable);
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

    let pdpt = &*(pdpt_phys as *const PageTable);
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

    let pd = &*(pd_phys as *const PageTable);
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

    let pt = &*(pt_phys as *const PageTable);
    let pt_idx = page_table_index(virt_addr, 1);
    let pt_entry = pt.entry(pt_idx);
    crate::println!("    [{}] = 0x{:016x} ({}, {})",
        pt_idx, pt_entry.raw(),
        if pt_entry.is_present() { "present" } else { "not present" },
        if pt_entry.is_writable() { "RW" } else { "RO" });
}
