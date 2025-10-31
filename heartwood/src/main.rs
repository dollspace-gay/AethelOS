#![no_std]
#![no_main]

//! # The Heartwood
//!
//! The living core of AethelOS - a hybrid microkernel that embodies
//! the principles of symbiotic computing.
//!
//! The Heartwood manages only the most sacred responsibilities:
//! - The Loom of Fate (scheduler)
//! - The Mana Pool (memory management)
//! - The Nexus (inter-process communication)
//! - The Attunement Layer (hardware abstraction)

extern crate alloc;

// Reference the modules from lib.rs
use heartwood::{nexus, loom_of_fate, mana_pool, attunement, drivers};

// Need to use macros with #[macro_use]
#[macro_use]
extern crate heartwood;

/// Helper function to write a single character to COM1 serial port
/// NOTE: Only use this BEFORE serial driver is initialized!
/// After init, use drivers::serial::write_byte() instead
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// The First Spark - Entry point of the Heartwood
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Write '1' to serial to prove _start() was called
    unsafe { serial_out(b'1'); }

    // CRITICAL: Zero the .bss section
    // After higher-half migration, GRUB doesn't zero .bss at virtual addresses,
    // so all static variables contain garbage! This causes locks to spin forever.
    unsafe {
        extern "C" {
            static mut __bss_start: u8;
            static mut __bss_end: u8;
        }

        let bss_start = &mut __bss_start as *mut u8;
        let bss_end = &mut __bss_end as *mut u8;
        let bss_size = bss_end as usize - bss_start as usize;

        // DEBUG: Output BSS size to verify it's non-zero
        serial_out(b'[');
        if bss_size == 0 {
            serial_out(b'0');
        } else if bss_size > 0 {
            serial_out(b'+');
        }
        serial_out(b']');

        // Zero the BSS manually with a simple loop instead of write_bytes
        // (write_bytes might be optimized out or broken in higher-half)
        let mut i = 0usize;
        while i < bss_size {
            *bss_start.add(i) = 0;
            i += 1;

            // Progress marker every 10KB
            if i % 10240 == 0 {
                serial_out(b'.');
            }
        }

        serial_out(b'Z'); // BSS zeroed
    }

    // Ultra-early debug: write directly to VGA buffer BEFORE any initialization
    // After higher-half kernel migration, physical VGA buffer at 0xB8000 is
    // mapped to virtual address 0xFFFFFFFF800B8000 (top 2GB)
    unsafe {
        // Use the VGA buffer address constant from vga_buffer module (single source of truth)
        let vga = heartwood::vga_buffer::VGA_BUFFER_ADDRESS as *mut u16;
        // Write 'AETHEL' in white on black (0x0F = white, 0x00 = black)
        *vga.offset(0) = 0x0F41; // 'A'
        *vga.offset(1) = 0x0F45; // 'E'
        *vga.offset(2) = 0x0F54; // 'T'
        *vga.offset(3) = 0x0F48; // 'H'
        *vga.offset(4) = 0x0F45; // 'E'
        *vga.offset(5) = 0x0F4C; // 'L'
    }

    // Write '2' to serial after VGA write
    unsafe { serial_out(b'2'); }

    heartwood_init();

    // Write '3' to serial after init
    unsafe { serial_out(b'3'); }

    // --- THE GREAT HAND-OFF ---
    // This is the final act of the primordial bootstrap ghost.
    // We leap one-way into the idle thread, which will awaken the system.
    // This call NEVER returns - the ghost vanishes forever.
    unsafe {
        serial_out(b'H'); // Hand-off begins
        let idle_context = loom_of_fate::get_idle_thread_context();

        // CRITICAL: Verify the actual offsets of ThreadContext fields
        // Read critical fields from the context for validation
        let ctx_ptr_addr = idle_context as u64;
        let ctx_rsp = (*idle_context).rsp;
        let ctx_cs = (*idle_context).cs;
        let ctx_ss = (*idle_context).ss;

        // Validate context (silent checks - only error/warn on actual issues)
        if ctx_rsp == ctx_ptr_addr {
            println!("ERROR: RSP equals context pointer! Memory corruption detected!");
        }
        // Check stack alignment: RSP should be (16n - 8) for x86-64 ABI
        // (misaligned by 8 as if a call instruction pushed a return address)
        if ctx_rsp % 16 != 8 {
            println!("WARNING: RSP has incorrect alignment! rsp={:#x} (should be 16n-8)", ctx_rsp);
        }
        if ctx_cs != 0x08 {
            println!("WARNING: CS={:#x}, expected 0x08", ctx_cs);
        }
        if ctx_ss != 0x10 {
            println!("WARNING: SS={:#x}, expected 0x10", ctx_ss);
        }

        serial_out(b'X'); // About to call context_switch_first

        // CRITICAL: Force release all VGA buffer locks before the Great Hand-Off!
        // The spinlock MUST be clear or the first println in idle thread will deadlock
        heartwood::vga_buffer::force_unlock();

        serial_out(b'Y'); // VGA lock forcibly released

        heartwood::loom_of_fate::context::context_switch_first(idle_context);
    }

    // UNREACHABLE - the bootstrap ghost is gone
    // This is intentional defensive programming to document the expected behavior
    #[allow(unreachable_code)]
    {
        unreachable!("The Great Hand-Off should never return")
    }
}

/// Detect ATA drives and mount filesystem (FAT32 or ext4)
fn detect_and_mount_storage() {
    use heartwood::drivers::AtaDrive;
    use heartwood::vfs::fat32::Fat32;
    use heartwood::vfs::ext4::Ext4;
    use heartwood::vfs::global as vfs_global;
    use alloc::boxed::Box;

    // Initialize global VFS
    vfs_global::init();

    // Try to detect primary master first
    println!("  Checking primary master...");
    let drive = match AtaDrive::detect_primary_master() {
        Some(d) => Some(d),
        None => {
            // Master not found or is ATAPI, try slave
            println!("  Checking primary slave...");
            AtaDrive::detect_primary_slave()
        }
    };

    match drive {
        Some(drive) => {
            let sectors = drive.sector_count();
            let size_mb = (sectors * 512) / (1024 * 1024);
            println!("  ✓ Detected ATA drive: {} sectors (~{} MB)", sectors, size_mb);

            // Try to auto-detect filesystem type
            println!("  ◈ Detecting filesystem type...");

            // Try ext4 first (check magic number)
            println!("  ◈ Attempting to mount ext4 filesystem...");
            match Ext4::new(Box::new(drive)) {
                Ok(fs) => {
                    println!("  ✓ ext4 filesystem mounted successfully!");

                    // Mount globally so shell commands can access it
                    vfs_global::mount(Box::new(fs));
                    println!("  ✓ Filesystem mounted at / (accessible via shell)");
                    println!();
                    println!("  Try: reveal to list files");
                }
                Err(_) => {
                    // ext4 failed, try FAT32
                    println!("  ⚠ Not an ext4 filesystem, trying FAT32...");

                    // Need to re-detect drive since we consumed it
                    // Try master first, then slave
                    let drive2 = match AtaDrive::detect_primary_master() {
                        Some(d) => Some(d),
                        None => AtaDrive::detect_primary_slave()
                    };

                    if let Some(drive2) = drive2 {
                        match Fat32::new(Box::new(drive2)) {
                            Ok(fs) => {
                                println!("  ✓ FAT32 filesystem mounted successfully!");

                                // Mount globally so shell commands can access it
                                vfs_global::mount(Box::new(fs));
                                println!("  ✓ Filesystem mounted at / (accessible via shell)");
                                println!();
                                println!("  Try: reveal to list files");
                            }
                            Err(e) => {
                                println!("  ✗ Failed to mount FAT32: {}", e);
                                println!("  ✗ Unknown or unsupported filesystem type");
                            }
                        }
                    } else {
                        println!("  ✗ Could not re-detect drive for FAT32 probe");
                    }
                }
            }
        }
        None => {
            println!("  ⚠ No ATA drive detected on primary channel");
            println!("  (Use QEMU with -hda <disk.img> to attach a disk)");
        }
    }
}

/// Remove identity mapping after boot
///
/// We need identity mapping during boot to execute low-address code,
/// but once we're running at higher-half addresses, we should remove
/// it to free PML4[0] for user space.
///
/// # Safety
/// This must be called AFTER:
/// - Boot code has finished executing
/// - Stack has been switched to higher-half address
/// - Global allocator has been initialized with higher-half addresses
///
/// After this function, accessing low addresses (0x0-0x3FFFFFFF) will
/// cause a page fault, which is intentional.
unsafe fn remove_identity_mapping() {
    use core::arch::asm;

    // Get current PML4 physical address from CR3
    let pml4_phys: u64;
    asm!("mov {}, cr3", out(reg) pml4_phys, options(nomem, nostack));

    // Convert to virtual address using identity mapping (still active)
    // Bootloader identity-maps first 1GB, so physical address = virtual address
    let pml4 = &mut *(pml4_phys as *mut mana_pool::page_tables::PageTable);

    // Clear PML4[0] to remove identity mapping
    // This frees virtual addresses 0x0-0x7FFF_FFFF_FFFF for user space
    pml4.entry_mut(0).set_raw(0);

    // CRITICAL: Set up recursive page table mapping at PML4[510]
    // This allows page table structures to be accessed after CR3 switch
    // PML4[510] points to the PML4 itself (recursive mapping)
    serial_out(b'[');
    serial_out(b'R');
    serial_out(b'E');
    serial_out(b'C');
    serial_out(b']');

    use mana_pool::page_tables::PageFlag;
    pml4.entry_mut(510).set_raw(pml4_phys |
        (PageFlag::Present as u64) |
        (PageFlag::ReadWrite as u64));

    serial_out(b'5'); // PML4[510] = recursive
    serial_out(b'1');
    serial_out(b'0');

    // Flush entire TLB to ensure changes take effect immediately
    asm!("mov cr3, {}", in(reg) pml4_phys, options(nostack));

    // Simple marker instead of serial_println to avoid formatting issues
    serial_out(b'R'); // Identity mapping Removed
}

/// Initialize the Heartwood's core systems
fn heartwood_init() {
    unsafe { serial_out(b'A'); } // Before init sequence

    // Initialize VGA text mode FIRST (no allocator dependency now!)
    unsafe { serial_out(b'B'); }
    heartwood::vga_buffer::initialize();
    unsafe { serial_out(b'b'); }

    // Initialize serial port (COM1 at 115200 baud, 8N1)
    unsafe {
        drivers::serial::init();
        drivers::serial::write_str("AethelOS serial port initialized\n");
    }

    heartwood::vga_buffer::print_banner();
    unsafe { serial_out(b'*'); } // DEBUG: After banner returns

    // TEST: Try calling write_str directly (bypass write_fmt)
    unsafe { serial_out(b'D'); } // Before direct write_str
    unsafe {
        heartwood::vga_buffer::test_write_str();
    }
    unsafe { serial_out(b'd'); } // After direct write_str

    // TEST: Try simple ASCII println first
    unsafe { serial_out(b'T'); } // Before test println
    println!("TEST");
    unsafe { serial_out(b't'); } // After test println

    println!("◈ Initializing Heartwood subsystems...");
    unsafe { serial_out(b'P'); }

    // Initialize the global allocator FIRST (before any heap allocations!)
    println!("◈ Initializing global allocator...");
    unsafe { serial_out(b'G'); } // Before allocator init
    heartwood::init_global_allocator();
    unsafe { serial_out(b'g'); } // After allocator init
    println!("  ✓ Buddy allocator ready (4MB heap)");

    // Ensure kernel memory is writable (fix multiboot2 read-only mappings)
    // CRITICAL: Must be called BEFORE removing identity mapping!
    // This function needs identity mapping active to access page table physical addresses.
    println!("◈ Remapping kernel memory as writable...");
    unsafe {
        mana_pool::ensure_kernel_memory_writable();
    }
    println!("  ✓ Kernel memory permissions corrected");

    // Remove identity mapping now that we're fully in higher-half
    // This frees PML4[0] for user-space programs
    println!("◈ Removing identity mapping...");
    unsafe {
        remove_identity_mapping();
    }
    println!("  ✓ Kernel now higher-half only (PML4[256-511])");

    // Initialize the Mana Pool (memory management)
    println!("◈ Awakening Mana Pool...");
    mana_pool::init();
    println!("  ✓ Mana Pool ready");

    // Initialize capability sealing (must be after Mana Pool, before capabilities are created)
    println!("◈ Forging security wards...");
    unsafe { mana_pool::sealing::init(); }
    println!("  ✓ Capability sealing ready (HMAC-SHA256)");

    // Initialize The Weaver's Sigil (stack canary protection)
    println!("◈ Weaving the protective sigils...");
    unsafe {
        // Set initial canary for boot thread
        // This will be updated per-thread by the scheduler during context switches
        use mana_pool::entropy::ChaCha8Rng;
        let mut rng = ChaCha8Rng::from_hardware_fast();
        let boot_sigil = ((rng.next_u32() as u64) << 32) | (rng.next_u32() as u64);
        heartwood::stack_protection::set_current_canary(boot_sigil);
    }
    println!("  ✓ The Weaver's Sigil active (stack canary: LLVM strong mode)");
    println!("    Protecting all functions with buffers or address-taken locals");

    // Initialize the Nexus (IPC)
    println!("◈ Opening the Nexus...");
    nexus::init();
    println!("  ✓ Nexus established");

    // Initialize the Loom of Fate (scheduler)
    println!("◈ Weaving the Loom of Fate...");
    loom_of_fate::init();
    println!("  ✓ Loom ready");

    // Initialize heap canaries (after thread creation to avoid early boot issues)
    // Fixed: User data is now padded to 8-byte alignment to ensure post-canary is aligned
    unsafe {
        mana_pool::heap_canaries::init();
    }
    println!("  ✓ Heap canaries active (pre/post allocation protection, 8-byte aligned)");

    // Verify The Rune of Permanence section
    println!("◈ Verifying The Rune of Permanence...");
    if mana_pool::rune_of_permanence::verify_rune_section() {
        let (start, end) = mana_pool::rune_of_permanence::get_rune_boundaries();
        let size = mana_pool::rune_of_permanence::get_rune_size();
        let pages = mana_pool::rune_of_permanence::get_rune_page_count();
        println!("  ✓ .rune section verified");
        println!("    Address: 0x{:016x} - 0x{:016x}", start, end);
        println!("    Size: {} bytes ({} KB)", size, size / 1024);
        println!("    Pages: {} (4KB pages)", pages);
    } else {
        panic!("◈ FATAL: .rune section verification failed!");
    }

    // Initialize the Attunement Layer (this includes IDT initialization)
    // IMPORTANT: Must happen before sealing, as IDT is in .rune section
    println!("◈ Attuning to hardware...");
    unsafe {
        for &byte in b"[DEBUG] Before attunement::init()\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    attunement::init();
    unsafe {
        for &byte in b"[DEBUG] After attunement::init() returned\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    println!("  ✓ Attunement complete");
    unsafe {
        for &byte in b"[DEBUG] After Attunement complete println\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }

    // Initialize security policy (must be before sealing)
    unsafe {
        for &byte in b"[DEBUG] Before security policy init\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    println!("◈ Scribing security policy...");
    mana_pool::security_policy::init();
    println!("  ✓ Security policy configured");
    mana_pool::security_policy::display_policy();
    unsafe {
        for &byte in b"[DEBUG] After security policy display\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }

    // Initialize the Eldarin Shell
    unsafe {
        for &byte in b"[DEBUG] Before eldarin::init()\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    println!("◈ Awakening the Eldarin Shell...");
    heartwood::eldarin::init();
    unsafe {
        for &byte in b"[DEBUG] After eldarin::init() returned\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    println!("  ✓ Shell ready");
    unsafe {
        for &byte in b"[DEBUG] After Shell ready println\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }

    // Detect ATA drives and mount filesystem (BEFORE sealing to avoid issues)
    println!("◈ Detecting storage devices...");
    unsafe {
        for &byte in b"[DEBUG] About to detect storage\n".iter() {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    detect_and_mount_storage();
    println!();

    // Seal The Rune of Permanence (make read-only at MMU level)
    // This must happen AFTER all .rune structures are initialized (IDT, security policy, etc.)
    // AND after disk mounting to avoid page table conflicts
    unsafe {
        mana_pool::rune_of_permanence::seal_rune_section();
    }
    println!("  ✓ Rune of Permanence sealed (kernel data now immutable)");

    println!("\n◈ Heartwood initialization complete!");
    println!();
}

// Panic handler is defined in lib.rs
