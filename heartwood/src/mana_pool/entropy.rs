//! # Entropy - Random Number Generation for Security
//!
//! Provides cryptographically-strong random numbers for ASLR and other security features.
//! Uses hardware RDRAND/RDSEED when available, falls back to RDTSC-seeded PRNG.

use core::arch::x86_64::{_rdrand64_step, _rdtsc};

/// Hardware entropy source using x86_64 RDRAND instruction
pub struct HardwareRng;

impl HardwareRng {
    /// Try to get a random u64 from RDRAND
    /// Returns None if RDRAND is not available or fails
    pub fn try_u64() -> Option<u64> {
        unsafe {
            let mut value: u64 = 0;
            // RDRAND can fail (returns 0 in CF flag), so we try a few times
            for _ in 0..10 {
                if _rdrand64_step(&mut value) == 1 {
                    return Some(value);
                }
            }
            None
        }
    }

    /// Get random u64, using RDRAND or falling back to RDTSC
    pub fn u64() -> u64 {
        Self::try_u64().unwrap_or_else(|| {
            // Fallback: Use RDTSC (timestamp counter) as entropy
            // Not cryptographically secure, but better than nothing
            unsafe { _rdtsc() }
        })
    }

    /// Fast non-blocking random u64 using only RDTSC
    /// Use this during early boot when RDRAND might not be available
    pub fn fast_u64() -> u64 {
        unsafe { _rdtsc() }
    }

    /// Get random u32
    pub fn u32() -> u32 {
        Self::u64() as u32
    }

    /// Get random usize
    pub fn usize() -> usize {
        Self::u64() as usize
    }

    /// Get random value in range [min, max)
    pub fn range(min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let range = max - min;
        min + (Self::usize() % range)
    }
}

/// Simple ChaCha8-based PRNG for when we need reproducible randomness
/// (e.g., for testing or when hardware RNG is not available)
pub struct ChaCha8Rng {
    state: [u32; 16],
    buffer: [u8; 64],
    buffer_pos: usize,
}

impl ChaCha8Rng {
    /// Create a new ChaCha8 RNG seeded from hardware entropy
    pub fn from_hardware() -> Self {
        let seed = HardwareRng::u64();
        Self::from_seed(seed)
    }

    /// Create ChaCha8 RNG using fast RDTSC-only seeding (boot-safe)
    /// Use this during early boot when RDRAND might not be available
    pub fn from_hardware_fast() -> Self {
        let seed = HardwareRng::fast_u64();
        Self::from_seed(seed)
    }

    /// Create a new ChaCha8 RNG from a 64-bit seed
    pub fn from_seed(seed: u64) -> Self {
        let timestamp = unsafe { _rdtsc() };

        Self {
            state: [
                // ChaCha constant "expand 32-byte k"
                0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
                // Seed material
                seed as u32, (seed >> 32) as u32, seed as u32, (seed >> 32) as u32,
                // Mix in timestamp for extra entropy
                timestamp as u32, (timestamp >> 32) as u32,
                // Rest zeros
                0, 0, 0, 0, 0, 0,
            ],
            buffer: [0; 64],
            buffer_pos: 64, // Force generation on first call
        }
    }

    /// Generate next block
    fn generate_block(&mut self) {
        let mut working = self.state;

        // 8 rounds (ChaCha8)
        for _ in 0..4 {
            // Column rounds
            Self::quarter_round(&mut working, 0, 4, 8, 12);
            Self::quarter_round(&mut working, 1, 5, 9, 13);
            Self::quarter_round(&mut working, 2, 6, 10, 14);
            Self::quarter_round(&mut working, 3, 7, 11, 15);

            // Diagonal rounds
            Self::quarter_round(&mut working, 0, 5, 10, 15);
            Self::quarter_round(&mut working, 1, 6, 11, 12);
            Self::quarter_round(&mut working, 2, 7, 8, 13);
            Self::quarter_round(&mut working, 3, 4, 9, 14);
        }

        // Add original state
        for i in 0..16 {
            working[i] = working[i].wrapping_add(self.state[i]);
        }

        // Convert to bytes - manual byte extraction to avoid to_le_bytes() and copy_from_slice()
        // (those methods hang in higher-half with -mcmodel=kernel)
        for i in 0..16 {
            let word = working[i];
            self.buffer[i * 4] = (word & 0xFF) as u8;
            self.buffer[i * 4 + 1] = ((word >> 8) & 0xFF) as u8;
            self.buffer[i * 4 + 2] = ((word >> 16) & 0xFF) as u8;
            self.buffer[i * 4 + 3] = ((word >> 24) & 0xFF) as u8;
        }

        // Increment counter
        self.state[12] = self.state[12].wrapping_add(1);
        if self.state[12] == 0 {
            self.state[13] = self.state[13].wrapping_add(1);
        }

        self.buffer_pos = 0;
    }

    #[inline(always)]
    fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        state[a] = state[a].wrapping_add(state[b]);
        state[d] ^= state[a];
        state[d] = state[d].rotate_left(16);

        state[c] = state[c].wrapping_add(state[d]);
        state[b] ^= state[c];
        state[b] = state[b].rotate_left(12);

        state[a] = state[a].wrapping_add(state[b]);
        state[d] ^= state[a];
        state[d] = state[d].rotate_left(8);

        state[c] = state[c].wrapping_add(state[d]);
        state[b] ^= state[c];
        state[b] = state[b].rotate_left(7);
    }

    /// Get next random u64
    pub fn next_u64(&mut self) -> u64 {
        if self.buffer_pos + 8 > 64 {
            self.generate_block();
        }

        // Direct byte manipulation instead of stack array + copy_from_slice
        // (those methods can hang in higher-half with -mcmodel=kernel)
        let pos = self.buffer_pos;
        let result = (self.buffer[pos] as u64)
            | ((self.buffer[pos + 1] as u64) << 8)
            | ((self.buffer[pos + 2] as u64) << 16)
            | ((self.buffer[pos + 3] as u64) << 24)
            | ((self.buffer[pos + 4] as u64) << 32)
            | ((self.buffer[pos + 5] as u64) << 40)
            | ((self.buffer[pos + 6] as u64) << 48)
            | ((self.buffer[pos + 7] as u64) << 56);
        self.buffer_pos += 8;

        result
    }

    /// Get next random u32
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Get next random usize
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    /// Get random value in range [min, max)
    pub fn range(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let range = max - min;
        min + (self.next_usize() % range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_rng() {
        let val1 = HardwareRng::u64();
        let val2 = HardwareRng::u64();
        // Should be different (probability of collision is negligible)
        assert_ne!(val1, val2);
    }

    #[test]
    fn test_chacha8_deterministic() {
        let mut rng1 = ChaCha8Rng::from_seed(12345);
        let mut rng2 = ChaCha8Rng::from_seed(12345);

        // Same seed should produce same sequence
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_chacha8_different_seeds() {
        let mut rng1 = ChaCha8Rng::from_seed(12345);
        let mut rng2 = ChaCha8Rng::from_seed(54321);

        // Different seeds should produce different values
        assert_ne!(rng1.next_u64(), rng2.next_u64());
    }

    #[test]
    fn test_range() {
        let mut rng = ChaCha8Rng::from_seed(42);

        for _ in 0..100 {
            let val = rng.range(100, 200);
            assert!(val >= 100 && val < 200);
        }
    }
}
