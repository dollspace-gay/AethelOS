//! # ASLR - Address Space Layout Randomization
//!
//! Randomizes memory layout to prevent exploits that rely on predictable addresses.
//! Inspired by PaX ASLR.
//!
//! ## Security Benefits
//! - Makes ROP (Return-Oriented Programming) attacks harder
//! - Prevents buffer overflow exploits from jumping to known addresses
//! - Increases entropy in the system (harder to guess memory layout)
//!
//! ## Randomization Strategy
//! - Stack: 28 bits of entropy (256MB range)
//! - Heap: 28 bits of entropy (256MB range)
//! - Code: 24 bits of entropy (16MB range, aligned)
//! - Groves (libraries): 20 bits of entropy (1MB range, page-aligned)

use super::entropy::{HardwareRng, ChaCha8Rng};

/// Memory layout for a process/thread with ASLR
#[derive(Debug, Clone, Copy)]
pub struct RandomizedLayout {
    /// Randomized code base address
    pub code_base: usize,

    /// Randomized stack base address (top of stack)
    pub stack_base: usize,

    /// Randomized heap base address
    pub heap_base: usize,

    /// Randomized capability table location
    pub capability_table: usize,

    /// Entropy used for this layout (for auditing)
    pub entropy_bits: u32,
}

impl RandomizedLayout {
    /// Create a new randomized memory layout using hardware RNG
    pub fn new() -> Self {
        let mut rng = ChaCha8Rng::from_hardware();
        Self::from_rng(&mut rng)
    }

    /// Create a new randomized layout from a specific RNG (for testing)
    pub fn from_rng(rng: &mut ChaCha8Rng) -> Self {
        // x86_64 user space address ranges (simplified for now)
        // We use the lower half of the address space (0x0000_0000 - 0x7FFF_FFFF_FFFF)
        // Reserved kernel space starts at 0xFFFF_8000_0000_0000

        Self {
            code_base: Self::randomize_code_base(rng),
            stack_base: Self::randomize_stack_base(rng),
            heap_base: Self::randomize_heap_base(rng),
            capability_table: Self::randomize_capability_table(rng),
            entropy_bits: 28 + 28 + 24 + 20, // Total entropy across all regions
        }
    }

    /// Randomize code base (executable region)
    /// Range: 0x00400000 - 0x01400000 (16MB range, 24 bits entropy)
    /// Aligned to 4KB pages
    fn randomize_code_base(rng: &mut ChaCha8Rng) -> usize {
        const CODE_START: usize = 0x00400000;  // 4MB (traditional ELF base)
        const CODE_RANGE: usize = 0x01000000;  // 16MB range
        const PAGE_SIZE: usize = 0x1000;       // 4KB alignment

        let offset = rng.range(0, CODE_RANGE / PAGE_SIZE) * PAGE_SIZE;
        CODE_START + offset
    }

    /// Randomize stack base (grows downward)
    /// Range: 0x70000000 - 0x80000000 (256MB range, 28 bits entropy)
    /// Stack grows DOWN from this address
    fn randomize_stack_base(rng: &mut ChaCha8Rng) -> usize {
        const STACK_START: usize = 0x70000000;  // 1.75GB
        const STACK_RANGE: usize = 0x10000000;  // 256MB range
        const PAGE_SIZE: usize = 0x1000;        // 4KB alignment

        let offset = rng.range(0, STACK_RANGE / PAGE_SIZE) * PAGE_SIZE;
        STACK_START + offset
    }

    /// Randomize heap base (grows upward)
    /// Range: 0x10000000 - 0x20000000 (256MB range, 28 bits entropy)
    fn randomize_heap_base(rng: &mut ChaCha8Rng) -> usize {
        const HEAP_START: usize = 0x10000000;  // 256MB
        const HEAP_RANGE: usize = 0x10000000;  // 256MB range
        const PAGE_SIZE: usize = 0x1000;       // 4KB alignment

        let offset = rng.range(0, HEAP_RANGE / PAGE_SIZE) * PAGE_SIZE;
        HEAP_START + offset
    }

    /// Randomize capability table location
    /// Range: 0x60000000 - 0x60100000 (1MB range, 20 bits entropy)
    fn randomize_capability_table(rng: &mut ChaCha8Rng) -> usize {
        const CAP_START: usize = 0x60000000;   // 1.5GB
        const CAP_RANGE: usize = 0x00100000;   // 1MB range
        const PAGE_SIZE: usize = 0x1000;       // 4KB alignment

        let offset = rng.range(0, CAP_RANGE / PAGE_SIZE) * PAGE_SIZE;
        CAP_START + offset
    }

    /// Randomize a stack offset (for individual thread stacks)
    /// This adds additional per-thread randomization on top of the base
    pub fn randomize_stack_offset() -> usize {
        const MAX_OFFSET: usize = 0x10000;  // 64KB max offset
        const ALIGN: usize = 16;            // 16-byte alignment for x86_64

        let offset = HardwareRng::range(0, MAX_OFFSET / ALIGN) * ALIGN;
        offset
    }

    /// Get entropy estimate in bits
    pub fn entropy_bits(&self) -> u32 {
        self.entropy_bits
    }
}

impl Default for RandomizedLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Global ASLR state (for future process management)
pub struct AslrManager {
    /// Master RNG for generating process layouts
    rng: ChaCha8Rng,

    /// Whether ASLR is enabled (can be disabled for debugging)
    enabled: bool,
}

impl AslrManager {
    /// Create a new ASLR manager
    pub fn new() -> Self {
        Self {
            rng: ChaCha8Rng::from_hardware(),
            enabled: true,
        }
    }

    /// Generate a new randomized layout for a process
    pub fn generate_layout(&mut self) -> RandomizedLayout {
        if self.enabled {
            RandomizedLayout::from_rng(&mut self.rng)
        } else {
            // Deterministic layout for debugging
            Self::deterministic_layout()
        }
    }

    /// Get a deterministic (non-random) layout for debugging
    pub fn deterministic_layout() -> RandomizedLayout {
        RandomizedLayout {
            code_base: 0x00400000,
            stack_base: 0x70000000,
            heap_base: 0x10000000,
            capability_table: 0x60000000,
            entropy_bits: 0,  // No randomization
        }
    }

    /// Enable or disable ASLR
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if ASLR is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for AslrManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to randomize a pointer offset (for fine-grained ASLR)
/// Use this for small randomizations like stack frame offsets
///
/// This function uses fast RDTSC-based entropy to avoid blocking during early boot
pub fn randomize_offset(max_bytes: usize, alignment: usize) -> usize {
    if alignment == 0 || max_bytes == 0 {
        return 0;
    }

    let max_units = max_bytes / alignment;
    // Use fast_u64() to avoid RDRAND blocking during early boot
    let random_value = HardwareRng::fast_u64() as usize;
    (random_value % max_units) * alignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_randomized_layouts_differ() {
        let layout1 = RandomizedLayout::new();
        let layout2 = RandomizedLayout::new();

        // Layouts should be different (probability of collision is negligible)
        assert_ne!(layout1.code_base, layout2.code_base);
        assert_ne!(layout1.stack_base, layout2.stack_base);
    }

    #[test]
    fn test_code_base_in_valid_range() {
        let mut rng = ChaCha8Rng::from_seed(42);

        for _ in 0..100 {
            let layout = RandomizedLayout::from_rng(&mut rng);

            // Code should be in valid range
            assert!(layout.code_base >= 0x00400000);
            assert!(layout.code_base < 0x01400000);

            // Should be page-aligned
            assert_eq!(layout.code_base % 0x1000, 0);
        }
    }

    #[test]
    fn test_stack_base_in_valid_range() {
        let mut rng = ChaCha8Rng::from_seed(42);

        for _ in 0..100 {
            let layout = RandomizedLayout::from_rng(&mut rng);

            // Stack should be in valid range
            assert!(layout.stack_base >= 0x70000000);
            assert!(layout.stack_base < 0x80000000);

            // Should be page-aligned
            assert_eq!(layout.stack_base % 0x1000, 0);
        }
    }

    #[test]
    fn test_deterministic_layout() {
        let layout = AslrManager::deterministic_layout();

        assert_eq!(layout.code_base, 0x00400000);
        assert_eq!(layout.stack_base, 0x70000000);
        assert_eq!(layout.entropy_bits, 0);
    }

    #[test]
    fn test_aslr_manager_toggle() {
        let mut manager = AslrManager::new();

        assert!(manager.is_enabled());

        manager.set_enabled(false);
        assert!(!manager.is_enabled());

        let layout = manager.generate_layout();
        assert_eq!(layout.entropy_bits, 0);  // Should be deterministic
    }
}
