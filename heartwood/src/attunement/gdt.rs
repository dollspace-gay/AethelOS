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
//!
//! ## The Rune of Permanence
//! The GDT and TSS are placed in the .rune section and become read-only after
//! boot, protecting them from data-only attacks that might try to modify
//! privilege levels or segment boundaries.

use core::arch::asm;
use core::mem::{size_of, MaybeUninit};

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
            granularity: 0b11001111,  // 4KB granularity, 32-bit (leave as-is for kernel), limit high = 0xF
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
            granularity: 0b10001111,  // 4KB granularity, 64-bit (D/B=0), limit high = 0xF
            base_high: 0,
        }
    }

    /// Create a service code segment (Ring 1, executable, readable)
    /// Ring 1 is for privileged services (Groves) that sit between kernel and userspace
    pub const fn service_code() -> Self {
        GdtEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0b10111010,  // P=1, DPL=01(Ring1), S=1(code/data), E=1(exec), DC=0, RW=1(readable), A=0
            granularity: 0b10101111,  // 4KB granularity, 64-bit, limit high = 0xF
            base_high: 0,
        }
    }

    /// Create a service data segment (Ring 1, writable)
    /// Ring 1 is for privileged services (Groves) that sit between kernel and userspace
    pub const fn service_data() -> Self {
        GdtEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0b10110010,  // P=1, DPL=01(Ring1), S=1(code/data), E=0(data), DC=0, RW=1(writable), A=0
            granularity: 0b10001111,  // 4KB granularity, 64-bit (D/B=0), limit high = 0xF
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
    /// rsp[0] = Ring 0 (kernel) stack
    /// rsp[1] = Ring 1 (service) stack
    /// rsp[2] = Ring 2 (unused)
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

    /// Set the kernel stack pointer (used when switching from Ring 3/1 to Ring 0)
    pub fn set_kernel_stack(&mut self, stack_ptr: u64) {
        self.rsp[0] = stack_ptr;
    }

    /// Set the service stack pointer (used when switching from Ring 3 to Ring 1)
    pub fn set_service_stack(&mut self, stack_ptr: u64) {
        self.rsp[1] = stack_ptr;
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
    entries: [GdtEntry; 10],  // Null, K_Code, K_Data, U_Data, U_Code, S_Code, S_Data, TSS_Low, TSS_High, Reserved
    len: usize,
}

impl GlobalDescriptorTable {
    /// Create a new GDT with default segments
    pub const fn new() -> Self {
        GlobalDescriptorTable {
            entries: [GdtEntry::null(); 10],
            len: 1,  // Start with null descriptor
        }
    }

    /// Initialize the GDT with kernel, user, and service segments
    pub fn initialize(&mut self, tss: &'static TaskStateSegment) {
        self.len = 1;  // Reset to just null descriptor

        // Add kernel code segment (index 1, selector 0x08)
        let idx = self.len;
        self.entries[self.len] = GdtEntry::kernel_code();
        crate::serial_println!("[GDT INIT] Added kernel_code at index {}", idx);
        self.len += 1;

        // Add kernel data segment (index 2, selector 0x10)
        let idx = self.len;
        self.entries[self.len] = GdtEntry::kernel_data();
        crate::serial_println!("[GDT INIT] Added kernel_data at index {}", idx);
        self.len += 1;

        // === FIX: User segments must be in order SS, then CS for sysret ===
        // Add user data segment (index 3, selector 0x18) - Used as SS by sysret
        let idx = self.len;
        self.entries[self.len] = GdtEntry::user_data();
        crate::serial_println!("[GDT INIT] Added user_data at index {}", idx);
        self.len += 1;

        // Add user code segment (index 4, selector 0x20) - Used as CS by sysret
        // sysret requires: CS selector = SS selector + 8
        let idx = self.len;
        self.entries[self.len] = GdtEntry::user_code();
        crate::serial_println!("[GDT INIT] Added user_code at index {}", idx);
        self.len += 1;

        // Add service code segment (index 5, selector 0x28) - Ring 1 for Groves
        let service_code = GdtEntry::service_code();
        let service_code_value = unsafe { core::mem::transmute::<GdtEntry, u64>(service_code) };
        let idx = self.len;
        self.entries[self.len] = service_code;
        crate::serial_println!("[GDT INIT] Added service_code at index {} = {:#018x}",
            idx,
            service_code_value
        );
        self.len += 1;

        // Add service data segment (index 6, selector 0x30) - Ring 1 for Groves
        let service_data = GdtEntry::service_data();
        let service_data_value = unsafe { core::mem::transmute::<GdtEntry, u64>(service_data) };
        let idx = self.len;
        self.entries[self.len] = service_data;
        crate::serial_println!("[GDT INIT] Added service_data at index {} = {:#018x}",
            idx,
            service_data_value
        );
        self.len += 1;

        // Add TSS (takes 2 entries in 64-bit mode, indices 7-8)
        let tss_entries = GdtEntry::tss(tss);
        self.entries[self.len] = tss_entries[0];
        self.len += 1;
        self.entries[self.len] = tss_entries[1];
        self.len += 1;
    }

    /// Load the GDT and reload segment registers
    pub fn load(&'static self) {
        let len = self.len;
        let base_addr = self.entries.as_ptr() as u64;
        let limit_val = (len * size_of::<GdtEntry>() - 1) as u16;

        let ptr = GdtPointer {
            limit: limit_val,
            base: base_addr,
        };

        crate::serial_println!("[GDT LOAD] Loading GDT at {:#x} with limit {:#x} ({} entries)",
            base_addr, limit_val, len);

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
            // TSS selector is at index 7, so selector = 7 * 8 = 0x38
            asm!("ltr ax", in("ax") selectors::TSS, options(nostack, preserves_flags));
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

    /// User data segment selector (Ring 3) - Stack Segment for sysret
    /// MUST be 8 bytes before USER_CODE for sysret compatibility
    pub const USER_DATA: u16 = 0x18 | 3;  // RPL = 3, index 3

    /// User code segment selector (Ring 3) - Code Segment for sysret
    /// MUST be USER_DATA + 8 for sysret compatibility
    pub const USER_CODE: u16 = 0x20 | 3;  // RPL = 3, index 4

    /// Service code segment selector (Ring 1) - For privileged Groves
    pub const SERVICE_CODE: u16 = 0x28 | 1;  // RPL = 1, index 5

    /// Service data segment selector (Ring 1) - For privileged Groves
    pub const SERVICE_DATA: u16 = 0x30 | 1;  // RPL = 1, index 6

    /// TSS selector (updated for Ring 1 segments)
    pub const TSS: u16 = 0x38;  // Index 7
}

// ═══════════════════════════════════════════════════════════════════════════
// Runtime GDT and TSS - Placed in .rune for permanence
// ═══════════════════════════════════════════════════════════════════════════

/// The Task State Segment - placed in .rune for permanence
///
/// After boot, this becomes read-only, preventing modification of privilege
/// stack pointers or IST entries.
#[link_section = ".rune"]
static mut TSS: MaybeUninit<TaskStateSegment> = MaybeUninit::uninit();

/// The Global Descriptor Table - placed in .rune for permanence
///
/// After boot, this becomes read-only, preventing modification of segment
/// descriptors, privilege levels, or segment boundaries.
#[link_section = ".rune"]
static mut GDT: MaybeUninit<GlobalDescriptorTable> = MaybeUninit::uninit();

/// Track whether GDT has been initialized
static mut GDT_INITIALIZED: bool = false;

/// Initialize the GDT and TSS
///
/// This MUST be called before seal_rune_section(), as it writes to the GDT/TSS.
/// It replaces the boot GDT (from boot32.rs) with a proper runtime GDT that
/// includes user segments and a TSS.
pub fn init() {
    unsafe {
        crate::serial_println!("[GDT INIT] GDT static variable is at address: {:#x}",
            &GDT as *const _ as u64);

        // Initialize TSS
        let tss = TaskStateSegment::new();
        TSS.write(tss);
        let tss_ref: &'static TaskStateSegment = TSS.assume_init_ref();

        // Initialize GDT with all segments
        let mut gdt = GlobalDescriptorTable::new();
        gdt.initialize(tss_ref);
        GDT.write(gdt);
        GDT_INITIALIZED = true;

        crate::serial_println!("[GDT INIT] After GDT.write(), entries array is at: {:#x}",
            GDT.assume_init_ref().entries.as_ptr() as u64);

        // Load the new GDT
        let gdt_ref: &'static GlobalDescriptorTable = GDT.assume_init_ref();
        gdt_ref.load();

        // Load the TSS
        gdt_ref.load_tss();
    }
}

/// Get a reference to the GDT (for introspection)
///
/// # Safety
/// Must only be called after init()
pub unsafe fn get_gdt() -> &'static GlobalDescriptorTable {
    if !GDT_INITIALIZED {
        panic!("GDT not initialized!");
    }
    GDT.assume_init_ref()
}

/// Get a reference to the TSS (for introspection)
///
/// # Safety
/// Must only be called after init()
pub unsafe fn get_tss() -> &'static TaskStateSegment {
    if !GDT_INITIALIZED {
        panic!("GDT/TSS not initialized!");
    }
    TSS.assume_init_ref()
}

/// Force-reload Ring 1 segments into the ACTIVE GDT
///
/// This is a workaround for the issue where the GDT gets reloaded at a different
/// address after initialization. We use sgdt to find the current GDT and patch
/// the Ring 1 segments directly into it.
pub unsafe fn patch_ring1_segments_into_active_gdt() {
    // Get the current GDT address via sgdt
    let mut gdt_base: u64 = 0;
    let mut gdt_limit: u16 = 0;
    core::arch::asm!(
        "sub rsp, 10",
        "sgdt [rsp]",
        "mov {0:x}, [rsp + 2]",  // Load base
        "mov {1:x}, [rsp]",       // Load limit
        "add rsp, 10",
        out(reg) gdt_base,
        out(reg) gdt_limit,
    );

    crate::serial_println!("[GDT PATCH] Current GDT at {:#x}, limit {:#x}", gdt_base, gdt_limit);

    // Create Ring 1 segment descriptors
    let service_code = GdtEntry::service_code();
    let service_data = GdtEntry::service_data();

    // Calculate pointers to indices 5 and 6 in the active GDT
    let service_code_ptr = (gdt_base + 5 * 8) as *mut GdtEntry;
    let service_data_ptr = (gdt_base + 6 * 8) as *mut GdtEntry;

    // Write the Ring 1 segments directly to the active GDT
    core::ptr::write(service_code_ptr, service_code);
    core::ptr::write(service_data_ptr, service_data);

    crate::serial_println!("[GDT PATCH] ✓ Patched Ring 1 segments into active GDT");
    crate::serial_println!("[GDT PATCH]   SERVICE_CODE at {:#x}", service_code_ptr as u64);
    crate::serial_println!("[GDT PATCH]   SERVICE_DATA at {:#x}", service_data_ptr as u64);
}

/// Get a mutable reference to the TSS
///
/// # Safety
/// Must only be called after init()
/// Caller must ensure no concurrent access to TSS
pub unsafe fn get_tss_mut() -> &'static mut TaskStateSegment {
    if !GDT_INITIALIZED {
        panic!("GDT/TSS not initialized!");
    }
    TSS.assume_init_mut()
}

/// Update the kernel stack pointer in the TSS and per-CPU data
///
/// This should be called during context switches when switching to a
/// user-mode thread. The kernel_stack value will be loaded into RSP
/// when a syscall or interrupt occurs from user mode.
///
/// **CRITICAL**: This updates BOTH:
/// - TSS.rsp[0] - Used by hardware interrupts
/// - GS:[8] (PerCpuData.kernel_stack_top) - Used by syscall instruction
///
/// # Arguments
/// * `kernel_stack` - Top of kernel stack (RSP value for syscall/interrupt entry)
///
/// # Safety
/// Must be called after GDT/TSS initialization and per-CPU data initialization
pub unsafe fn set_kernel_stack(kernel_stack: u64) {
    crate::serial_println!("[GDT] set_kernel_stack: About to get_tss_mut()");
    // Update TSS for hardware interrupts
    let tss = get_tss_mut();
    crate::serial_println!("[GDT] set_kernel_stack: Got TSS, setting to {:#x}", kernel_stack);
    tss.set_kernel_stack(kernel_stack);
    crate::serial_println!("[GDT] set_kernel_stack: TSS updated");

    // Update per-CPU data for syscall instruction
    // syscall_entry reads from GS:[8] to load kernel stack
    crate::serial_println!("[GDT] set_kernel_stack: About to get per_cpu");
    let per_cpu = crate::attunement::per_cpu::get_mut();
    crate::serial_println!("[GDT] set_kernel_stack: Got per_cpu, updating GS:[8]");
    per_cpu.kernel_stack_top = kernel_stack;
    crate::serial_println!("[GDT] set_kernel_stack: ✓ Complete");
}
