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
use heartwood::{nexus, loom_of_fate, mana_pool, attunement, vga_buffer};

// Need to use macros with #[macro_use]
#[macro_use]
extern crate heartwood;

/// The First Spark - Entry point of the Heartwood
#[no_mangle]
pub extern "C" fn _start() -> ! {
    heartwood_init();

    vga_buffer::print_banner();

    // Start the system threads and begin multitasking
    println!();
    match loom_of_fate::start_system_threads() {
        Ok(()) => {
            // Begin weaving - this will never return
            loom_of_fate::begin_weaving();
        }
        Err(e) => {
            println!("❌ Failed to start system threads: {:?}", e);
            // Fall back to infinite loop
            loop {
                // Halt to save power
                unsafe {
                    core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
                }
            }
        }
    }
}

/// Initialize the Heartwood's core systems
fn heartwood_init() {
    // Initialize VGA buffer for early output
    vga_buffer::initialize();

    println!("[] Awakening the Heartwood...");

    // Initialize the Mana Pool (memory management)
    println!("[] Kindling the Mana Pool...");
    mana_pool::init();

    // Initialize the Nexus (IPC)
    println!("[] Opening the Nexus...");
    nexus::init();

    // Initialize the Loom of Fate (scheduler)
    println!("[] Weaving the Loom of Fate...");
    loom_of_fate::init();

    // Initialize the Attunement Layer
    println!("[] Attuning to the hardware...");
    attunement::init();

    println!("[] The Heartwood lives!");
}

// Panic handler is defined in lib.rs
