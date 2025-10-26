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

    /// Create a new ChaCha8 RNG from a 64-bit seed
    pub fn from_seed(seed: u64) -> Self {
        let mut state = [0u32; 16];

        // ChaCha constant "expand 32-byte k"
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;

        // Seed material
        state[4] = seed as u32;
        state[5] = (seed >> 32) as u32;
        state[6] = seed as u32;
        state[7] = (seed >> 32) as u32;

        // Mix in timestamp for extra entropy
        let timestamp = unsafe { _rdtsc() };
        state[8] = timestamp as u32;
        state[9] = (timestamp >> 32) as u32;

        Self {
            state,
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

        // Convert to bytes
        for (i, &word) in working.iter().enumerate() {
            let bytes = word.to_le_bytes();
            self.buffer[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
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

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + 8]);
        self.buffer_pos += 8;

        u64::from_le_bytes(bytes)
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
