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

use spin::Mutex;
use core::mem::MaybeUninit;

/// The global Task State Segment
static mut TSS: MaybeUninit<gdt::TaskStateSegment> = MaybeUninit::uninit();

/// The global Global Descriptor Table
static mut GDT: MaybeUninit<gdt::GlobalDescriptorTable> = MaybeUninit::uninit();

/// The global Interrupt Descriptor Table
static mut IDT: MaybeUninit<Mutex<idt::InterruptDescriptorTable>> = MaybeUninit::uninit();

/// The global PIC configuration
static mut PIC: MaybeUninit<Mutex<pic::Pic>> = MaybeUninit::uninit();

/// The global PIT configuration
static mut PIT: MaybeUninit<pit::Pit> = MaybeUninit::uninit();

static mut ATTUNEMENT_INITIALIZED: bool = false;

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Get references to globals (assumes initialized)
unsafe fn get_tss() -> &'static gdt::TaskStateSegment {
    TSS.assume_init_ref()
}

unsafe fn get_gdt() -> &'static gdt::GlobalDescriptorTable {
    GDT.assume_init_ref()
}

unsafe fn get_idt() -> &'static Mutex<idt::InterruptDescriptorTable> {
    IDT.assume_init_ref()
}

unsafe fn get_pic() -> &'static Mutex<pic::Pic> {
    PIC.assume_init_ref()
}

unsafe fn get_pit() -> &'static pit::Pit {
    PIT.assume_init_ref()
}

/// Initialize the Attunement Layer
pub fn init() {
    crate::println!("  ◈ Attuning to hardware...");

    unsafe {
        serial_out(b'T'); // TSS init
        core::ptr::write(TSS.as_mut_ptr(), gdt::TaskStateSegment::new());
        serial_out(b't'); // TSS complete

        serial_out(b'G'); // GDT init
        let mut gdt = gdt::GlobalDescriptorTable::new();
        gdt.initialize(get_tss());
        core::ptr::write(GDT.as_mut_ptr(), gdt);
        serial_out(b'g'); // GDT complete

        serial_out(b'I'); // IDT init
        let mut idt = idt::InterruptDescriptorTable::new();
        idt.initialize();
        core::ptr::write(IDT.as_mut_ptr(), Mutex::new(idt));
        serial_out(b'i'); // IDT complete

        serial_out(b'P'); // PIC init
        core::ptr::write(PIC.as_mut_ptr(), Mutex::new(pic::Pic::new(32, 40)));
        serial_out(b'p'); // PIC complete

        serial_out(b'~'); // PIT init
        core::ptr::write(PIT.as_mut_ptr(), pit::Pit::new());
        serial_out(b'^'); // PIT complete

        ATTUNEMENT_INITIALIZED = true;
        serial_out(b'@'); // All statics initialized
    }

    // Initialize CPU features
    cpu::init();

    // Load the Global Descriptor Table
    crate::println!("    ∴ Loading privilege rings (GDT)...");
    unsafe {
        get_gdt().load();
        get_gdt().load_tss();
    }

    // Initialize Programmable Interrupt Controller (PIC)
    // Remap IRQs 0-15 to interrupts 32-47
    crate::println!("    ∴ Remapping interrupt controller (PIC)...");
    unsafe {
        get_pic().lock().initialize();
        get_pic().lock().disable_all(); // Start with all IRQs masked
    }

    // Load the Interrupt Descriptor Table
    crate::println!("    ∴ Preparing interrupt handlers (IDT)...");
    unsafe {
        get_idt().lock().load();
    }

    // Initialize interrupt handling
    interrupts::init();

    // Initialize system timer (PIT)
    unsafe {
        crate::println!("    ∴ Starting system heartbeat (PIT @ {} Hz)...", get_pit().frequency());
    }
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
    use spin::Mutex;
    use core::mem::MaybeUninit;

    static mut TICKS: MaybeUninit<Mutex<u64>> = MaybeUninit::uninit();
    static mut UPTIME_MS: MaybeUninit<Mutex<u64>> = MaybeUninit::uninit();
    static mut TIMER_INITIALIZED: bool = false;

    unsafe fn get_ticks() -> &'static Mutex<u64> {
        TICKS.assume_init_ref()
    }

    unsafe fn get_uptime_ms() -> &'static Mutex<u64> {
        UPTIME_MS.assume_init_ref()
    }

    /// Initialize the system timer
    pub fn init() {
        unsafe {
            core::ptr::write(TICKS.as_mut_ptr(), Mutex::new(0));
            core::ptr::write(UPTIME_MS.as_mut_ptr(), Mutex::new(0));
            TIMER_INITIALIZED = true;
        }

        // Enable timer IRQ (IRQ 0)
        unsafe {
            super::get_pic().lock().enable_irq(0);
        }

        // Initialize the PIT to generate interrupts at 100 Hz
        unsafe {
            super::get_pit().initialize();
        }
    }

    /// Get the number of timer ticks since boot
    pub fn ticks() -> u64 {
        unsafe { *get_ticks().lock() }
    }

    /// Get uptime in milliseconds
    pub fn uptime_ms() -> u64 {
        unsafe { *get_uptime_ms().lock() }
    }

    /// Get uptime in seconds
    pub fn uptime_seconds() -> u64 {
        uptime_ms() / 1000
    }

    /// Timer interrupt handler (called by interrupt)
    pub fn on_tick() {
        unsafe {
            let mut ticks = get_ticks().lock();
            *ticks += 1;

            // Update uptime (100 Hz = 10ms per tick)
            let mut uptime = get_uptime_ms().lock();
            *uptime += super::pit::ms_per_tick(super::get_pit()) as u64;
        }
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
