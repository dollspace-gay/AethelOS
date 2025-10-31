//! # Kernel Memory Remapping
//!
//! Ensures kernel memory is properly mapped as writable.
//! This is necessary because multiboot2 may map kernel sections
//! based on ELF program headers, which can mark sections as read-only.

use x86_64::structures::paging::PageTableFlags;

/// Helper to write to serial for debugging (no dependencies)
#[inline]
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Remap ONLY the kernel heap region as writable
///
/// This function ensures the heap region (12MB-20MB in kernel space) is writable,
/// while preserving W^X for code sections. This is critical for allowing the
/// kernel to write to heap-allocated structures (like ThreadContext).
///
/// # Safety
/// Must be called early in kernel initialization, before any heap allocations.
pub unsafe fn ensure_kernel_memory_writable() {
    // Heap bounds (from lib.rs init_global_allocator)
    const KERNEL_BASE: usize = 0xFFFFFFFF80000000;
    const HEAP_START: usize = KERNEL_BASE + 0xC00000;  // 12MB
    const HEAP_SIZE: usize = 0x800000;                  // 8MB
    const HEAP_END: usize = HEAP_START + HEAP_SIZE;     // 20MB
    // Use direct port I/O for diagnostics (no dependencies)
    serial_out(b'[');
    serial_out(b'R');
    serial_out(b'E');
    serial_out(b'M');
    serial_out(b'A');
    serial_out(b'P');
    serial_out(b']');

    crate::serial_println!("[REMAP] Ensuring kernel memory is writable...");

    serial_out(b'1'); // Marker 1: About to read CR3

    // Read current CR3 (physical address of PML4)
    let cr3_phys = read_cr3();

    serial_out(b'2'); // Marker 2: CR3 read successfully

    // Convert to virtual address so we can access it
    let pml4_virt = phys_to_virt(cr3_phys);
    serial_out(b'3'); // Marker 3: Converted address

    let pml4 = &mut *(pml4_virt as *mut PageTable);
    serial_out(b'4'); // Marker 4: Got PML4 reference

    // Entry [511] maps kernel space (0xFFFFFFFF80000000 to 0xFFFFFFFFFFFFFFFF)
    // This is the only entry we need to fix
    let entry_511 = &mut pml4.entries[511];
    serial_out(b'5'); // Marker 5: Got entry [511]

    if !entry_511.is_present() {
        serial_out(b'X'); // Entry not present!
        crate::serial_println!("[REMAP] WARNING: Entry[511] not present!");
        return;
    }
    serial_out(b'6'); // Marker 6: Entry is present

    // Walk page tables and ONLY set WRITABLE for heap pages
    // This preserves W^X for code sections
    let pdpt_phys = entry_511.addr().as_u64();
    let pdpt_virt = phys_to_virt(pdpt_phys);
    let pdpt = &mut *(pdpt_virt as *mut PageTable);

    serial_out(b'7'); // Got PDPT

    // Page table math:
    // - Each PML4 entry covers 512GB
    // - Each PDPT entry covers 1GB
    // - Each PD entry covers 2MB
    // - Each PT entry covers 4KB

    // Heap is at KERNEL_BASE + 12MB to KERNEL_BASE + 20MB
    // KERNEL_BASE = 0xFFFFFFFF80000000
    // Calculating PDPT index from address bits 38-30:
    // (0xFFFFFFFF80000000 >> 30) & 0x1FF = 510
    // PD index: (offset % 1GB) / 2MB = 6 to 10

    let pdpt_entry = &mut pdpt.entries[510]; // Kernel is at PDPT[510]
    if !pdpt_entry.is_present() {
        serial_out(b'N'); // PDPT[510] not present
        crate::serial_println!("[REMAP] WARNING: PDPT[510] not present!");
        return;
    }

    serial_out(b'8'); // PDPT[510] present

    // Walk to PD (Page Directory)
    let pd_phys = pdpt_entry.addr().as_u64();
    let pd_virt = phys_to_virt(pd_phys);
    let pd = &mut *(pd_virt as *mut PageTable);

    // Set WRITABLE for PD entries covering heap (12MB-20MB = entries 6-9)
    // Entry 6: 12MB-14MB
    // Entry 7: 14MB-16MB
    // Entry 8: 16MB-18MB
    // Entry 9: 18MB-20MB
    for i in 6..10 {
        let pd_entry = &mut pd.entries[i];
        if !pd_entry.is_present() {
            continue; // Skip unmapped pages
        }

        let mut flags = pd_entry.flags();

        // Only set WRITABLE if not already set (preserve existing EXEC permissions)
        if !flags.contains(PageTableFlags::WRITABLE) {
            flags |= PageTableFlags::WRITABLE;
            pd_entry.set_flags(flags);
        }

        // If this is a huge page (2MB), we're done for this entry
        if flags.contains(PageTableFlags::HUGE_PAGE) {
            continue;
        }

        // Walk down to PT (Page Table) for 4KB pages
        let pt_phys = pd_entry.addr().as_u64();
        let pt_virt = phys_to_virt(pt_phys);
        let pt = &mut *(pt_virt as *mut PageTable);

        // Set WRITABLE for all PT entries
        for pt_entry in pt.entries.iter_mut() {
            if pt_entry.is_present() {
                let mut pt_flags = pt_entry.flags();
                if !pt_flags.contains(PageTableFlags::WRITABLE) {
                    pt_flags |= PageTableFlags::WRITABLE;
                    pt_entry.set_flags(pt_flags);
                }
            }
        }
    }

    serial_out(b'9'); // Completed heap remapping

    // Flush TLB to apply changes
    serial_out(b'F'); // About to flush TLB
    flush_tlb();
    serial_out(b'D'); // Done flushing TLB

    crate::serial_println!("[REMAP] ✓ Kernel memory remapped as writable");

    serial_out(b'[');
    serial_out(b'O');
    serial_out(b'K');
    serial_out(b']');
}

/// Read CR3 register (page table base)
#[inline]
fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}

/// Convert physical address to virtual address
#[inline]
fn phys_to_virt(phys: u64) -> u64 {
    // This function is called BEFORE remove_identity_mapping(),
    // so the identity mapping (PML4[0]) is still active.
    // This means physical addresses can be accessed directly as virtual addresses.
    phys
}

/// Flush TLB (Translation Lookaside Buffer)
#[inline]
fn flush_tlb() {
    unsafe {
        // Reload CR3 to flush TLB
        let cr3 = read_cr3();
        core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
    }
}

/// Page table structure (simplified for direct access)
#[repr(C, align(4096))]
struct PageTable {
    entries: [PageTableEntry; 512],
}

/// Page table entry (simplified)
#[repr(C)]
struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    fn is_present(&self) -> bool {
        (self.entry & 0x1) != 0
    }

    fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.entry)
    }

    fn set_flags(&mut self, flags: PageTableFlags) {
        // Preserve address bits (bits 12-51), update flags
        let addr = self.entry & 0x000F_FFFF_FFFF_F000;
        self.entry = addr | flags.bits();
    }

    fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.entry & 0x000F_FFFF_FFFF_F000)
    }
}

/// Physical address wrapper
struct PhysAddr(u64);

impl PhysAddr {
    fn new(addr: u64) -> Self {
        PhysAddr(addr)
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}
