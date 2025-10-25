//! # Global Descriptor Table
//!
//! The GDT defines the privilege boundaries of AethelOS.
//! Unlike traditional systems that enforce rigid hierarchies,
//! our rings exist in symbiosis - Ring 0 (kernel) and Ring 3 (user)
//! dance together in mutual respect.
//!
//! ## Philosophy
//! Privilege is not dominion, but responsibility.
//! The kernel serves userspace as much as userspace relies on the kernel.

use core::arch::asm;
use core::mem::size_of;

/// GDT Entry - A segment descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    /// Create a null descriptor
    pub const fn null() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }

    /// Create a kernel code segment (Ring 0, executable, readable)
    pub const fn kernel_code() -> Self {
        GdtEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0b10011010,  // Present, Ring 0, Code, Execute/Read
            granularity: 0b10101111,  // 4KB granularity, 64-bit, limit high = 0xF
            base_high: 0,
        }
    }

    /// Create a kernel data segment (Ring 0, writable)
    pub const fn kernel_data() -> Self {
        GdtEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0b10010010,  // Present, Ring 0, Data, Read/Write
            granularity: 0b11001111,  // 4KB granularity, 32-bit, limit high = 0xF
            base_high: 0,
        }
    }

    /// Create a user code segment (Ring 3, executable, readable)
    pub const fn user_code() -> Self {
        GdtEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0b11111010,  // Present, Ring 3, Code, Execute/Read
            granularity: 0b10101111,  // 4KB granularity, 64-bit, limit high = 0xF
            base_high: 0,
        }
    }

    /// Create a user data segment (Ring 3, writable)
    pub const fn user_data() -> Self {
        GdtEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0b11110010,  // Present, Ring 3, Data, Read/Write
            granularity: 0b11001111,  // 4KB granularity, 32-bit, limit high = 0xF
            base_high: 0,
        }
    }

    /// Create a TSS descriptor from a TSS reference
    pub fn tss(tss: &'static TaskStateSegment) -> [GdtEntry; 2] {
        let ptr = tss as *const _ as u64;
        let limit = (size_of::<TaskStateSegment>() - 1) as u64;

        let low = GdtEntry {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (ptr & 0xFFFF) as u16,
            base_middle: ((ptr >> 16) & 0xFF) as u8,
            access: 0b10001001,  // Present, Ring 0, TSS (available)
            granularity: ((limit >> 16) & 0x0F) as u8,
            base_high: ((ptr >> 24) & 0xFF) as u8,
        };

        let high = GdtEntry {
            limit_low: ((ptr >> 32) & 0xFFFF) as u16,
            base_low: ((ptr >> 48) & 0xFFFF) as u16,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        };

        [low, high]
    }
}

/// Task State Segment - Holds stack pointers for privilege transitions
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    /// Privilege stack pointers (RSP for rings 0-2)
    pub rsp: [u64; 3],
    reserved_2: u64,
    /// Interrupt stack table (IST 1-7)
    pub ist: [u64; 7],
    reserved_3: u64,
    reserved_4: u16,
    /// I/O Map Base Address
    pub iomap_base: u16,
}

impl TaskStateSegment {
    /// Create a new TSS with default values
    pub const fn new() -> Self {
        TaskStateSegment {
            reserved_1: 0,
            rsp: [0; 3],
            reserved_2: 0,
            ist: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: size_of::<TaskStateSegment>() as u16,
        }
    }

    /// Set the kernel stack pointer (used when switching from Ring 3 to Ring 0)
    pub fn set_kernel_stack(&mut self, stack_ptr: u64) {
        self.rsp[0] = stack_ptr;
    }

    /// Set an interrupt stack (IST entry)
    pub fn set_interrupt_stack(&mut self, index: usize, stack_ptr: u64) {
        if index < 7 {
            self.ist[index] = stack_ptr;
        }
    }
}

/// GDT Pointer structure for LGDT instruction
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

/// The Global Descriptor Table
#[repr(align(16))]
pub struct GlobalDescriptorTable {
    entries: [GdtEntry; 8],  // Null, K_Code, K_Data, U_Code, U_Data, TSS_Low, TSS_High, Reserved
    len: usize,
}

impl GlobalDescriptorTable {
    /// Create a new GDT with default segments
    pub const fn new() -> Self {
        GlobalDescriptorTable {
            entries: [GdtEntry::null(); 8],
            len: 1,  // Start with null descriptor
        }
    }

    /// Initialize the GDT with kernel and user segments
    pub fn initialize(&mut self, tss: &'static TaskStateSegment) {
        self.len = 1;  // Reset to just null descriptor

        // Add kernel code segment (index 1, selector 0x08)
        self.entries[self.len] = GdtEntry::kernel_code();
        self.len += 1;

        // Add kernel data segment (index 2, selector 0x10)
        self.entries[self.len] = GdtEntry::kernel_data();
        self.len += 1;

        // Add user code segment (index 3, selector 0x18)
        self.entries[self.len] = GdtEntry::user_code();
        self.len += 1;

        // Add user data segment (index 4, selector 0x20)
        self.entries[self.len] = GdtEntry::user_data();
        self.len += 1;

        // Add TSS (takes 2 entries in 64-bit mode)
        let tss_entries = GdtEntry::tss(tss);
        self.entries[self.len] = tss_entries[0];
        self.len += 1;
        self.entries[self.len] = tss_entries[1];
        self.len += 1;
    }

    /// Load the GDT and reload segment registers
    pub fn load(&'static self) {
        let ptr = GdtPointer {
            limit: (self.len * size_of::<GdtEntry>() - 1) as u16,
            base: self.entries.as_ptr() as u64,
        };

        unsafe {
            // Load GDT
            asm!("lgdt [{}]", in(reg) &ptr, options(readonly, nostack, preserves_flags));

            // Reload segment registers
            // Code segment via far return
            asm!(
                "push 0x08",           // Push kernel code selector
                "lea {tmp}, [rip + 2f]",
                "push {tmp}",
                "retfq",               // Far return to reload CS
                "2:",
                tmp = lateout(reg) _,
                options(preserves_flags),
            );

            // Data segments
            asm!(
                "mov ax, 0x10",        // Kernel data selector
                "mov ds, ax",
                "mov es, ax",
                "mov fs, ax",
                "mov gs, ax",
                "mov ss, ax",
                out("ax") _,
                options(preserves_flags, nostack),
            );
        }
    }

    /// Load the Task State Segment
    pub fn load_tss(&self) {
        unsafe {
            // TSS selector is at index 5, so selector = 5 * 8 = 0x28
            asm!("ltr ax", in("ax") 0x28u16, options(nostack, preserves_flags));
        }
    }
}

/// Segment selectors
#[allow(dead_code)]
pub mod selectors {
    /// Kernel code segment selector (Ring 0)
    pub const KERNEL_CODE: u16 = 0x08;

    /// Kernel data segment selector (Ring 0)
    pub const KERNEL_DATA: u16 = 0x10;

    /// User code segment selector (Ring 3)
    pub const USER_CODE: u16 = 0x18 | 3;  // RPL = 3

    /// User data segment selector (Ring 3)
    pub const USER_DATA: u16 = 0x20 | 3;  // RPL = 3

    /// TSS selector
    pub const TSS: u16 = 0x28;
}
