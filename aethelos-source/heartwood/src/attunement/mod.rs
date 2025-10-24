//! # The Attunement Layer
//!
//! The hardware abstraction layer of AethelOS.
//! The Attunement Layer does not command hardware; it attunes to it,
//! establishing a symbiotic relationship between silicon and software.
//!
//! ## Philosophy
//! Hardware is not a slave to be controlled, but a partner to be understood.
//! The Attunement Layer speaks the language of the hardware, translating
//! the Heartwood's intentions into signals the silicon can comprehend.

pub mod gdt;
pub mod idt;
pub mod keyboard;
pub mod pic;
pub mod pit;

use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// The global Task State Segment
    static ref TSS: gdt::TaskStateSegment = gdt::TaskStateSegment::new();

    /// The global Global Descriptor Table
    static ref GDT: gdt::GlobalDescriptorTable = {
        let mut gdt = gdt::GlobalDescriptorTable::new();
        gdt.initialize(&TSS);
        gdt
    };

    /// The global Interrupt Descriptor Table
    static ref IDT: Mutex<idt::InterruptDescriptorTable> = {
        let mut idt = idt::InterruptDescriptorTable::new();
        idt.initialize();
        Mutex::new(idt)
    };

    /// The global PIC configuration
    static ref PIC: Mutex<pic::Pic> = Mutex::new(pic::Pic::new(32, 40));

    /// The global PIT configuration
    static ref PIT: pit::Pit = pit::Pit::new();
}

/// Initialize the Attunement Layer
pub fn init() {
    crate::println!("  ◈ Attuning to hardware...");

    // Initialize CPU features
    cpu::init();

    // Load the Global Descriptor Table
    crate::println!("    ∴ Loading privilege rings (GDT)...");
    GDT.load();
    GDT.load_tss();

    // Initialize Programmable Interrupt Controller (PIC)
    // Remap IRQs 0-15 to interrupts 32-47
    crate::println!("    ∴ Remapping interrupt controller (PIC)...");
    unsafe {
        PIC.lock().initialize();
        PIC.lock().disable_all(); // Start with all IRQs masked
    }

    // Load the Interrupt Descriptor Table
    crate::println!("    ∴ Preparing interrupt handlers (IDT)...");
    IDT.lock().load();

    // Initialize interrupt handling
    interrupts::init();

    // Initialize system timer (PIT)
    crate::println!("    ∴ Starting system heartbeat (PIT @ {} Hz)...", PIT.frequency());
    timer::init();

    // Initialize keyboard
    crate::println!("    ∴ Listening for keystrokes (PS/2)...");
    keyboard::init();

    crate::println!("  ◈ Hardware attunement complete");
}

/// CPU information and features
pub mod cpu {
    /// Initialize CPU-specific features
    pub fn init() {
        // In a real implementation:
        // - Detect CPU features (SSE, AVX, etc.)
        // - Enable necessary features
        // - Set up CPU-local storage
    }

    /// Get CPU information
    pub fn info() -> CpuInfo {
        CpuInfo {
            vendor: "Unknown",
            model: "Unknown",
            cores: 1,
        }
    }

    pub struct CpuInfo {
        pub vendor: &'static str,
        pub model: &'static str,
        pub cores: usize,
    }
}

/// Interrupt handling
pub mod interrupts {
    use core::arch::asm;

    /// Initialize interrupt handling
    pub fn init() {
        // IDT is already loaded by main init()
        // Enable interrupts
        unsafe {
            enable();
        }
    }

    /// Disable interrupts
    pub unsafe fn disable() {
        asm!("cli", options(nostack, nomem));
    }

    /// Enable interrupts
    pub unsafe fn enable() {
        asm!("sti", options(nostack, nomem));
    }

    /// Check if interrupts are enabled
    pub fn are_enabled() -> bool {
        let flags: u64;
        unsafe {
            asm!("pushf; pop {}", out(reg) flags, options(nomem, preserves_flags));
        }
        (flags & 0x200) != 0 // IF flag is bit 9
    }
}

/// System timer
pub mod timer {
    use lazy_static::lazy_static;
    use spin::Mutex;

    lazy_static! {
        static ref TICKS: Mutex<u64> = Mutex::new(0);
        static ref UPTIME_MS: Mutex<u64> = Mutex::new(0);
    }

    /// Initialize the system timer
    pub fn init() {
        // Enable timer IRQ (IRQ 0)
        unsafe {
            super::PIC.lock().enable_irq(0);
        }

        // Initialize the PIT to generate interrupts at 100 Hz
        unsafe {
            super::PIT.initialize();
        }
    }

    /// Get the number of timer ticks since boot
    pub fn ticks() -> u64 {
        *TICKS.lock()
    }

    /// Get uptime in milliseconds
    pub fn uptime_ms() -> u64 {
        *UPTIME_MS.lock()
    }

    /// Get uptime in seconds
    pub fn uptime_seconds() -> u64 {
        uptime_ms() / 1000
    }

    /// Timer interrupt handler (called by interrupt)
    pub fn on_tick() {
        let mut ticks = TICKS.lock();
        *ticks += 1;

        // Update uptime (100 Hz = 10ms per tick)
        let mut uptime = UPTIME_MS.lock();
        *uptime += super::pit::ms_per_tick(&super::PIT) as u64;
    }

    /// Sleep for approximately the given number of milliseconds
    ///
    /// # Note
    /// This is a busy-wait implementation. In a real system, this would
    /// yield to the scheduler.
    pub fn sleep_ms(ms: u64) {
        let start = uptime_ms();
        while uptime_ms() - start < ms {
            core::hint::spin_loop();
        }
    }

    /// Sleep for approximately the given number of ticks
    pub fn sleep_ticks(ticks_to_wait: u64) {
        let start = ticks();
        while ticks() - start < ticks_to_wait {
            core::hint::spin_loop();
        }
    }
}
