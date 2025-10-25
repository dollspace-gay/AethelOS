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
use heartwood::{nexus, loom_of_fate, mana_pool, attunement};

// Need to use macros with #[macro_use]
#[macro_use]
extern crate heartwood;

/// Helper function to write a single character to COM1 serial port
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

    // Ultra-early debug: write directly to VGA buffer BEFORE any initialization
    // VGA text buffer is at 0xB8000
    unsafe {
        let vga = 0xb8000 as *mut u16;
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

/// Initialize the Heartwood's core systems
fn heartwood_init() {
    unsafe { serial_out(b'A'); } // Before init sequence

    // Initialize VGA text mode FIRST (no allocator dependency now!)
    unsafe { serial_out(b'B'); }
    heartwood::vga_buffer::initialize();
    unsafe { serial_out(b'b'); }

    // Display boot banner
    heartwood::vga_buffer::print_banner();
    println!("◈ Initializing Heartwood subsystems...");

    // Initialize the global allocator FIRST (before any heap allocations!)
    unsafe { serial_out(b'@'); }
    println!("◈ Initializing global allocator...");
    heartwood::init_global_allocator();
    unsafe { serial_out(b'#'); }
    println!("  ✓ Buddy allocator ready (4MB heap)");

    // Initialize the Mana Pool (memory management)
    unsafe { serial_out(b'C'); }
    println!("◈ Awakening Mana Pool...");
    mana_pool::init();
    unsafe { serial_out(b'D'); }
    println!("  ✓ Mana Pool ready");

    // Initialize the Nexus (IPC)
    unsafe { serial_out(b'E'); }
    println!("◈ Opening the Nexus...");
    nexus::init();
    unsafe { serial_out(b'F'); }
    println!("  ✓ Nexus established");

    // Initialize the Loom of Fate (scheduler)
    unsafe { serial_out(b'G'); }
    println!("◈ Weaving the Loom of Fate...");
    loom_of_fate::init();
    unsafe { serial_out(b'H'); }
    println!("  ✓ Loom ready");

    // Initialize the Attunement Layer
    unsafe { serial_out(b'I'); }
    println!("◈ Attuning to hardware...");
    attunement::init();
    unsafe { serial_out(b'J'); }
    println!("  ✓ Attunement complete");

    // Initialize the Eldarin Shell
    unsafe { serial_out(b'L'); }
    println!("◈ Awakening the Eldarin Shell...");
    heartwood::eldarin::init();
    unsafe { serial_out(b'M'); }
    println!("  ✓ Shell ready");

    unsafe { serial_out(b'K'); }
    println!("\n◈ Heartwood initialization complete!");
    println!();
}

// Panic handler is defined in lib.rs
