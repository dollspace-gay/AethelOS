//! # User Address Space Management
//!
//! Manages virtual memory for userspace processes (Vessels).

use alloc::vec::Vec;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};

/// User address space boundaries
pub const USER_SPACE_START: u64 = 0x0000_0000_0000_0000;
pub const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_FFFF;

/// Default user stack location
pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FFFF_0000;
pub const USER_STACK_SIZE: u64 = 0x0001_0000; // 64 KB (was 1MB - too large!)

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    Code,
    Data,
    ReadOnlyData,
    Heap,
    Stack,
}

/// Memory region in user address space
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: VirtAddr,
    pub size: u64,
    pub region_type: RegionType,
    pub flags: PageTableFlags,
}

impl MemoryRegion {
    pub fn new(start: VirtAddr, size: u64, region_type: RegionType) -> Self {
        let flags = match region_type {
            RegionType::Code => PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE,
            RegionType::Data | RegionType::Heap | RegionType::Stack => {
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | 
                PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_EXECUTE
            }
            RegionType::ReadOnlyData => {
                PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | 
                PageTableFlags::NO_EXECUTE
            }
        };
        Self { start, size, region_type, flags }
    }

    pub fn end(&self) -> VirtAddr {
        self.start + self.size
    }

    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start && addr < self.end()
    }

    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}

/// User address space for a Vessel
pub struct UserAddressSpace {
    pub pml4_phys: PhysAddr,
    pub regions: Vec<MemoryRegion>,
    pub heap_break: VirtAddr,
    pub next_stack: VirtAddr,
}

impl UserAddressSpace {
    pub fn new() -> Result<Self, &'static str> {
        // Allocate a new PML4 for this address space
        // This clones the kernel's page tables (upper half) and leaves the lower half empty
        let pml4_phys_addr = unsafe {
            crate::mana_pool::page_tables::clone_kernel_page_table()?
        };

        crate::serial_println!("[USER_SPACE] Created new address space with PML4 @ {:#x}", pml4_phys_addr);

        Ok(Self {
            pml4_phys: PhysAddr::new(pml4_phys_addr),
            regions: Vec::new(),
            heap_break: VirtAddr::new(0x0000_0000_0040_0000),
            next_stack: VirtAddr::new(USER_STACK_TOP),
        })
    }

    pub fn add_region(&mut self, region: MemoryRegion) -> Result<(), &'static str> {
        for existing in &self.regions {
            if existing.overlaps(&region) {
                return Err("Region overlaps with existing region");
            }
        }

        self.regions.push(region);
        Ok(())
    }

    /// Map a memory region into the page tables
    ///
    /// # Arguments
    ///
    /// * `region` - The region to map
    /// * `physical_frames` - Physical frames to map (one per page)
    ///
    /// # Safety
    ///
    /// The caller must ensure that physical_frames are valid and not in use elsewhere.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Region mapped successfully
    /// * `Err(&str)` - Mapping error
    pub unsafe fn map_region(
        &mut self,
        region: &MemoryRegion,
        physical_frames: &[PhysAddr],
    ) -> Result<(), &'static str> {
        // Calculate number of pages needed
        let num_pages = (region.size + 0xFFF) / 0x1000;

        // Ensure we have enough physical frames
        if physical_frames.len() < num_pages as usize {
            return Err("Not enough physical frames for region");
        }

        crate::serial_println!(
            "[USER_SPACE] Mapping region at {:#x} (size {:#x}, {} pages) with flags {:?}",
            region.start.as_u64(),
            region.size,
            num_pages,
            region.flags
        );

        // Map each page
        for i in 0..num_pages {
            let virt_addr = region.start.as_u64() + (i * 0x1000);
            let phys_addr = physical_frames[i as usize].as_u64();

            // Convert PageTableFlags to raw u64 using .bits()
            let flags = region.flags.bits();

            crate::mana_pool::page_tables::map_user_page(
                self.pml4_phys.as_u64(),
                virt_addr,
                phys_addr,
                flags,
            )?;
        }

        crate::serial_println!("[USER_SPACE]   ✓ Mapped {} pages successfully", num_pages);

        Ok(())
    }

    /// Allocate a new user stack
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the stack in bytes (will be rounded up to page size)
    ///
    /// # Returns
    ///
    /// * `Ok(VirtAddr)` - Top of the new stack
    /// * `Err(&str)` - Allocation error
    pub fn allocate_stack(&mut self, size: u64) -> Result<VirtAddr, &'static str> {
        // Round size up to page boundary (4KB)
        let size_pages = (size + 0xFFF) / 0x1000;
        let aligned_size = size_pages * 0x1000;

        // Allocate stack growing downward from next_stack
        let stack_bottom = self.next_stack - aligned_size;
        let stack_region = MemoryRegion::new(
            stack_bottom,
            aligned_size,
            RegionType::Stack,
        );

        // Check for overlaps
        self.add_region(stack_region.clone())?;

        // Allocate physical frames for the stack
        let mut allocated_frames = alloc::vec::Vec::with_capacity(size_pages as usize);
        for _ in 0..size_pages {
            let frame = allocate_physical_frame()?;
            allocated_frames.push(frame);
        }

        // Extract physical addresses for mapping
        let phys_addrs: alloc::vec::Vec<PhysAddr> = allocated_frames
            .iter()
            .map(|f| f.phys_addr)
            .collect();

        // Map the stack
        unsafe {
            self.map_region(&stack_region, &phys_addrs)?;
        }

        // Note: Stack frames are already zero-initialized, no need to copy data

        // Update next_stack pointer (leave 64KB guard gap)
        self.next_stack = stack_bottom - 0x10000u64;

        // Return top of stack (grows downward)
        Ok(stack_bottom + aligned_size)
    }

    /// Find which region contains a given address
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to look up
    ///
    /// # Returns
    ///
    /// * `Some(&MemoryRegion)` - The region containing this address
    /// * `None` - Address not mapped
    pub fn find_region(&self, addr: VirtAddr) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| r.contains(addr))
    }
}

/// Information about a segment contributing to a merged region
#[derive(Clone)]
struct SegmentContribution<'a> {
    segment: &'a crate::loom_of_fate::elf_loader::LoadedSegment,
}

/// A merged region that may contain multiple ELF segments
struct MergedRegion<'a> {
    page_aligned_start: u64,
    aligned_size: u64,
    region_type: RegionType,
    contributing_segments: Vec<SegmentContribution<'a>>,
}

/// Merge ELF segments that share pages after alignment
///
/// When segments are close together, they may map to the same pages after
/// rounding to 4KB boundaries. This function detects such overlaps and
/// merges them into single regions.
///
/// # Arguments
///
/// * `segments` - List of ELF segments to merge
///
/// # Returns
///
/// * `Ok(Vec<MergedRegion>)` - List of non-overlapping merged regions
/// * `Err(&str)` - Merge error
fn merge_overlapping_segments<'a>(
    segments: &'a [crate::loom_of_fate::elf_loader::LoadedSegment],
) -> Result<Vec<MergedRegion<'a>>, &'static str> {
    use alloc::vec::Vec;

    if segments.is_empty() {
        return Ok(Vec::new());
    }

    // Create a list of (page_start, page_end, segment) tuples
    let mut segment_ranges: Vec<(u64, u64, &crate::loom_of_fate::elf_loader::LoadedSegment)> = segments
        .iter()
        .map(|seg| {
            let page_start = seg.vaddr & !0xFFF;
            let offset_in_page = seg.vaddr - page_start;
            let page_end = ((seg.vaddr + seg.memsz + 0xFFF) & !0xFFF);
            (page_start, page_end, seg)
        })
        .collect();

    // Sort by page-aligned start address
    segment_ranges.sort_by_key(|(start, _, _)| *start);

    let mut merged_regions: Vec<MergedRegion> = Vec::new();

    for (page_start, page_end, segment) in segment_ranges {
        // Check if this segment overlaps with the last merged region
        if let Some(last_region) = merged_regions.last_mut() {
            let last_end = last_region.page_aligned_start + last_region.aligned_size;

            if page_start < last_end {
                // Overlaps! Extend the last region
                crate::serial_println!(
                    "[MERGE] Segment at {:#x} overlaps with previous region, merging",
                    segment.vaddr
                );

                // Extend the region to cover both
                let new_end = core::cmp::max(last_end, page_end);
                last_region.aligned_size = new_end - last_region.page_aligned_start;

                // Update region type to be the most permissive
                if segment.is_writable() {
                    last_region.region_type = RegionType::Data;
                } else if segment.is_executable() && last_region.region_type != RegionType::Data {
                    last_region.region_type = RegionType::Code;
                }

                // Add this segment to the contributors
                last_region.contributing_segments.push(SegmentContribution { segment });
                continue;
            }
        }

        // No overlap, create a new region
        let region_type = if segment.is_executable() {
            RegionType::Code
        } else if segment.is_writable() {
            RegionType::Data
        } else {
            RegionType::ReadOnlyData
        };

        merged_regions.push(MergedRegion {
            page_aligned_start: page_start,
            aligned_size: page_end - page_start,
            region_type,
            contributing_segments: alloc::vec![SegmentContribution { segment }],
        });
    }

    Ok(merged_regions)
}

/// Create a user address space from a loaded ELF file
///
/// This function creates page table mappings for all loadable segments
/// in the ELF file and sets up the initial user stack.
///
/// # Arguments
///
/// * `elf_data` - The raw ELF file data
///
/// # Returns
///
/// * `Ok((UserAddressSpace, u64))` - The address space and entry point
/// * `Err(&str)` - Load error
pub fn create_address_space_from_elf(
    elf_data: &[u8],
) -> Result<(UserAddressSpace, u64), &'static str> {
    use crate::loom_of_fate::elf_loader::{load_elf, ElfError};

    // Parse the ELF file
    let loaded_elf = load_elf(elf_data).map_err(|e| match e {
        ElfError::InvalidMagic => "Invalid ELF magic",
        ElfError::InvalidClass => "Not a 64-bit ELF",
        ElfError::InvalidEndianness => "Not little-endian",
        ElfError::InvalidVersion => "Invalid ELF version",
        ElfError::InvalidType => "Not an executable",
        ElfError::InvalidMachine => "Not x86-64",
        ElfError::FileTooSmall => "File too small",
        ElfError::ProgramHeaderOutOfBounds => "Invalid program headers",
        ElfError::InvalidAlignment => "Invalid alignment",
        ElfError::SegmentOverlapsKernel => "Segment overlaps kernel",
        ElfError::NoLoadableSegments => "No loadable segments",
    })?;

    crate::serial_println!("[USER_SPACE] Creating address space from ELF");
    crate::serial_println!("[USER_SPACE]   Entry point: {:#x}", loaded_elf.entry_point);
    crate::serial_println!("[USER_SPACE]   Base address: {:#x}", loaded_elf.base_address);

    // Create new address space
    let mut address_space = UserAddressSpace::new()?;

    // Group segments by page-aligned regions to handle overlaps
    let merged_regions = merge_overlapping_segments(&loaded_elf.segments)?;

    crate::serial_println!("[USER_SPACE] Merged {} segments into {} regions",
        loaded_elf.segments.len(), merged_regions.len());

    // Map each merged region
    for merged in &merged_regions {
        crate::serial_println!(
            "[USER_SPACE]   Mapping merged region: vaddr={:#x} size={:#x} ({} segments)",
            merged.page_aligned_start,
            merged.aligned_size,
            merged.contributing_segments.len()
        );

        // Create memory region
        let region = MemoryRegion::new(
            VirtAddr::new(merged.page_aligned_start),
            merged.aligned_size,
            merged.region_type,
        );

        // Add to address space
        address_space.add_region(region.clone())?;

        // Allocate physical frames for this region
        let num_pages = merged.aligned_size / 0x1000;
        let mut allocated_frames = alloc::vec::Vec::with_capacity(num_pages as usize);
        for _ in 0..num_pages {
            let frame = allocate_physical_frame()?;
            allocated_frames.push(frame);
        }

        // Extract physical addresses for mapping
        let phys_addrs: alloc::vec::Vec<PhysAddr> = allocated_frames
            .iter()
            .map(|f| f.phys_addr)
            .collect();

        // Map the region with allocated frames
        unsafe {
            address_space.map_region(&region, &phys_addrs)?;
        }

        // Copy data from all contributing segments
        for seg_info in &merged.contributing_segments {
            if seg_info.segment.filesz > 0 {
                crate::serial_println!(
                    "[USER_SPACE]   Copying segment data: vaddr={:#x} filesz={:#x}",
                    seg_info.segment.vaddr,
                    seg_info.segment.filesz
                );

                let file_start = seg_info.segment.file_offset as usize;
                let file_end = file_start + seg_info.segment.filesz as usize;

                if file_end <= elf_data.len() {
                    let segment_data = &elf_data[file_start..file_end];

                    // Calculate offset from the start of the merged region
                    let offset_from_region_start = seg_info.segment.vaddr - merged.page_aligned_start;

                    crate::serial_println!(
                        "[USER_SPACE]   Offset from region start: {:#x}",
                        offset_from_region_start
                    );

                    unsafe {
                        copy_to_frames_with_offset(&allocated_frames, segment_data, offset_from_region_start);
                    }
                    crate::serial_println!("[USER_SPACE]   ✓ Segment data copied");
                } else {
                    crate::serial_println!("[USER_SPACE]   WARNING: Segment data out of bounds");
                }
            }
        }
    }

    // Allocate initial user stack
    let stack_top = address_space.allocate_stack(USER_STACK_SIZE)?;
    crate::serial_println!("[USER_SPACE]   User stack: {:#x}", stack_top.as_u64());

    crate::serial_println!("[USER_SPACE] ✓ Address space created successfully");

    Ok((address_space, loaded_elf.entry_point))
}

/// Represents a physical frame with both its physical address and virtual pointer
struct AllocatedFrame {
    phys_addr: PhysAddr,
    virt_ptr: *mut u8,
}

/// Allocate a single physical frame (4KB page)
///
/// Uses the heap allocator to allocate a 4KB page. Returns both the physical
/// address (for page tables) and virtual pointer (for writing data).
/// The frame is guaranteed to be 4KB-aligned.
///
/// # Returns
/// AllocatedFrame containing both physical address and virtual pointer
///
/// # Safety
/// The frame is leaked and will never be freed (acceptable for Phase 2).
fn allocate_physical_frame() -> Result<AllocatedFrame, &'static str> {
    use alloc::alloc::{alloc, Layout};

    // Create a layout for 4KB page with 4KB alignment
    let layout = Layout::from_size_align(0x1000, 0x1000)
        .map_err(|_| "Failed to create layout")?;

    // Allocate the frame
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
        return Err("Failed to allocate frame");
    }

    // Zero-initialize the frame
    unsafe {
        core::ptr::write_bytes(ptr, 0, 0x1000);
    }

    // Get virtual address
    let virt_addr = ptr as u64;

    // Convert to physical address
    // AethelOS kernel base is 0xFFFFFFFF80000000
    const KERNEL_BASE: u64 = 0xFFFFFFFF80000000;
    let phys_addr = if virt_addr >= KERNEL_BASE {
        virt_addr - KERNEL_BASE
    } else {
        virt_addr
    };

    // Don't deallocate - we're leaking this intentionally
    // (In Phase 3+, we'll track these for proper cleanup)

    Ok(AllocatedFrame {
        phys_addr: PhysAddr::new(phys_addr),
        virt_ptr: ptr,
    })
}

/// Copy data into allocated physical frames with an optional offset in the first frame
///
/// # Arguments
/// * `frames` - Allocated frames with virtual pointers
/// * `data` - Data to copy
/// * `page_offset` - Offset within the first frame to start writing (for non-page-aligned segments)
///
/// # Safety
/// Caller must ensure frames are valid and properly allocated
unsafe fn copy_to_frames_with_offset(frames: &[AllocatedFrame], data: &[u8], page_offset: u64) {
    crate::serial_println!("[COPY] Starting copy: {} bytes to {} frames (offset={:#x})",
        data.len(), frames.len(), page_offset);

    let mut data_offset = 0usize;
    let mut frame_idx = 0;

    while data_offset < data.len() && frame_idx < frames.len() {
        let frame = &frames[frame_idx];
        crate::serial_println!("[COPY] Frame {}: phys={:#x} virt={:p}",
            frame_idx, frame.phys_addr.as_u64(), frame.virt_ptr);

        // For the first frame, account for page_offset
        let frame_offset = if frame_idx == 0 { page_offset as usize } else { 0 };
        let available_in_frame = 0x1000 - frame_offset;
        let bytes_to_copy = core::cmp::min(available_in_frame, data.len() - data_offset);
        let data_slice = &data[data_offset..data_offset + bytes_to_copy];

        // Calculate destination pointer (base + offset for first frame)
        let dest_ptr = frame.virt_ptr.add(frame_offset);

        crate::serial_println!("[COPY] Copying {} bytes to {:p} (frame offset={:#x})",
            bytes_to_copy, dest_ptr, frame_offset);

        // Copy data directly to the virtual pointer (already mapped in kernel space)
        core::ptr::copy_nonoverlapping(
            data_slice.as_ptr(),
            dest_ptr,
            bytes_to_copy,
        );

        crate::serial_println!("[COPY] ✓ Copied {} bytes", bytes_to_copy);

        data_offset += bytes_to_copy;
        frame_idx += 1;
    }
    crate::serial_println!("[COPY] ✓ All frames copied");
}

