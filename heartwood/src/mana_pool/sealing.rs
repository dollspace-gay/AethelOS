///! Capability Sealing - Cryptographic protection against forgery
///!
///! This module implements unforgeable capability seals using HMAC-SHA256.
///! Each capability is sealed with a kernel-only secret key, preventing
///! user space from forging or tampering with capabilities.
///!
///! # Security Properties
///! - **Unforgeable**: Can't create valid seal without kernel secret key
///! - **Tamper-Proof**: Modifying capability data invalidates seal
///! - **Constant-Time**: Verification uses constant-time comparison
///! - **Kernel-Only**: Secret key never leaves kernel space

use core::mem::MaybeUninit;
use crate::mana_pool::entropy::ChaCha8Rng;

/// Capability sealer using HMAC-SHA256
pub struct CapabilitySealer {
    /// Secret 256-bit key for HMAC (kernel-only, never exposed)
    seal_key: [u8; 32],
}

impl CapabilitySealer {
    /// Create a new sealer with cryptographically random key
    ///
    /// # Security
    /// The seal key is generated using hardware entropy (RDRAND/RDTSC)
    /// and mixed with ChaCha8 for additional randomness. This key must
    /// never be exposed to user space.
    ///
    /// # Safety
    /// Must be called only once during kernel initialization
    pub fn new() -> Self {
        let seal_key = Self::generate_seal_key();
        Self { seal_key }
    }

    /// Generate a cryptographically secure seal key
    ///
    /// Uses hardware entropy sources (RDTSC for boot-safety)
    /// mixed with ChaCha8 to generate 256 bits of entropy.
    fn generate_seal_key() -> [u8; 32] {
        let mut key = [0u8; 32];

        // Use ChaCha8 seeded from fast hardware entropy (RDTSC-only for boot safety)
        let mut rng = ChaCha8Rng::from_hardware_fast();

        // Generate 32 random bytes
        for i in 0..8 {
            let random = rng.next_u32();
            let bytes = random.to_le_bytes();
            key[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
        }

        key
    }

    /// Compute HMAC-SHA256 seal for capability data
    ///
    /// # Arguments
    /// * `data` - Serialized capability data to seal
    ///
    /// # Returns
    /// 256-bit HMAC-SHA256 tag
    pub fn seal(&self, data: &[u8]) -> [u8; 32] {
        hmac_sha256::HMAC::mac(data, &self.seal_key)
    }

    /// Verify capability seal (constant-time)
    ///
    /// # Arguments
    /// * `data` - Capability data to verify
    /// * `seal` - Seal tag to check
    ///
    /// # Returns
    /// `true` if seal is valid, `false` otherwise
    ///
    /// # Security
    /// Uses constant-time comparison to prevent timing attacks.
    /// An attacker cannot learn information about the correct seal
    /// by measuring verification time.
    pub fn verify(&self, data: &[u8], seal: &[u8; 32]) -> bool {
        let expected_seal = self.seal(data);
        constant_time_compare(&expected_seal, seal)
    }
}

/// Constant-time equality comparison
///
/// Compares two byte slices in constant time to prevent timing attacks.
/// Always compares all bytes regardless of when a mismatch is found.
///
/// # Security
/// This function takes the same amount of time whether the inputs match
/// or differ in the first byte vs last byte, preventing attackers from
/// learning information about the correct value through timing analysis.
fn constant_time_compare(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff: u8 = 0;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Global capability sealer (initialized at boot)
static mut SEALER: MaybeUninit<CapabilitySealer> = MaybeUninit::uninit();
static mut SEALER_INITIALIZED: bool = false;

/// Initialize the global capability sealer
///
/// # Safety
/// Must be called exactly once during kernel initialization,
/// before any capabilities are created.
pub unsafe fn init() {
    if !SEALER_INITIALIZED {
        let sealer = CapabilitySealer::new();
        core::ptr::write(core::ptr::addr_of_mut!(SEALER).cast(), sealer);
        SEALER_INITIALIZED = true;
        // TODO: Add logging when serial_println! is available
    }
}

/// Get reference to global sealer
///
/// # Safety
/// Caller must ensure `init()` has been called
pub unsafe fn get_sealer() -> &'static CapabilitySealer {
    debug_assert!(SEALER_INITIALIZED, "Sealer not initialized!");
    &*core::ptr::addr_of!(SEALER).cast::<CapabilitySealer>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_generation() {
        let sealer = CapabilitySealer::new();
        let data = b"test capability data";

        let seal = sealer.seal(data);

        // Seal should be deterministic for same data
        let seal2 = sealer.seal(data);
        assert_eq!(seal, seal2);
    }

    #[test]
    fn test_seal_verification() {
        let sealer = CapabilitySealer::new();
        let data = b"test capability data";

        let seal = sealer.seal(data);

        // Valid seal should verify
        assert!(sealer.verify(data, &seal));
    }

    #[test]
    fn test_tampered_data_fails() {
        let sealer = CapabilitySealer::new();
        let data = b"test capability data";

        let seal = sealer.seal(data);

        // Modified data should fail
        let tampered = b"test capability DATA";  // Changed case
        assert!(!sealer.verify(tampered, &seal));
    }

    #[test]
    fn test_tampered_seal_fails() {
        let sealer = CapabilitySealer::new();
        let data = b"test capability data";

        let seal = sealer.seal(data);

        // Flip one bit in seal
        let mut bad_seal = seal;
        bad_seal[0] ^= 1;

        assert!(!sealer.verify(data, &bad_seal));
    }

    #[test]
    fn test_different_keys_different_seals() {
        let sealer1 = CapabilitySealer::new();
        let sealer2 = CapabilitySealer::new();
        let data = b"test capability data";

        let seal1 = sealer1.seal(data);
        let seal2 = sealer2.seal(data);

        // Different keys should produce different seals
        // (With overwhelming probability)
        assert_ne!(seal1, seal2);
    }

    #[test]
    fn test_constant_time_compare() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let mut c = [0u8; 32];
        c[0] = 1;  // Differ in first byte
        let mut d = [0u8; 32];
        d[31] = 1;  // Differ in last byte

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
        assert!(!constant_time_compare(&a, &d));

        // All comparisons should take same time (not verifiable in test,
        // but at least check correctness)
    }
}
