//! # Security Policy - Immutable Security Configuration
//!
//! This module contains security policy flags that are initialized at boot
//! and then made immutable by placing them in the .rune section.
//!
//! After The Rune of Permanence is sealed, these flags cannot be modified,
//! preventing data-only attacks from disabling security features.

/// Security Policy Configuration - placed in .rune for permanence
///
/// These flags control critical security features and become read-only
/// after boot, preventing attackers from disabling protections.
#[repr(C)]
pub struct SecurityPolicy {
    /// Whether capability-based security is enforced
    pub capabilities_enabled: bool,

    /// Whether W^X (Write XOR Execute) is enforced
    pub wx_enforcement: bool,

    /// Whether stack canaries are enabled
    pub stack_canaries_enabled: bool,

    /// Whether heap canaries are enabled
    pub heap_canaries_enabled: bool,

    /// Whether The Rune of Permanence is sealed
    pub rune_sealed: bool,

    /// Whether ASLR is enabled
    pub aslr_enabled: bool,
}

/// The global security policy - placed in .rune section for permanence
#[link_section = ".rune"]
static mut SECURITY_POLICY: SecurityPolicy = SecurityPolicy {
    capabilities_enabled: false,  // Will be set to true during init
    wx_enforcement: false,        // Will be set to true during init
    stack_canaries_enabled: false,
    heap_canaries_enabled: false,
    rune_sealed: false,
    aslr_enabled: false,
};

/// Initialize the security policy
///
/// This MUST be called before seal_rune_section(), as it writes to the policy.
pub fn init() {
    unsafe {
        // Enable all security features
        SECURITY_POLICY.capabilities_enabled = true;
        SECURITY_POLICY.wx_enforcement = true;
        SECURITY_POLICY.stack_canaries_enabled = true;
        SECURITY_POLICY.heap_canaries_enabled = true;
        SECURITY_POLICY.aslr_enabled = true;
        // rune_sealed will be set to true by seal_rune_section()
    }
}

/// Mark the rune as sealed in the security policy
///
/// Called by seal_rune_section() after sealing is complete.
///
/// # Safety
/// This must be the LAST write to the security policy before sealing,
/// or it must be called from within seal_rune_section() before the
/// actual MMU protection is applied.
pub unsafe fn mark_rune_sealed() {
    SECURITY_POLICY.rune_sealed = true;
}

/// Check if capabilities are enabled
#[inline(always)]
pub fn are_capabilities_enabled() -> bool {
    unsafe { SECURITY_POLICY.capabilities_enabled }
}

/// Check if W^X enforcement is enabled
#[inline(always)]
pub fn is_wx_enforced() -> bool {
    unsafe { SECURITY_POLICY.wx_enforcement }
}

/// Check if stack canaries are enabled
#[inline(always)]
pub fn are_stack_canaries_enabled() -> bool {
    unsafe { SECURITY_POLICY.stack_canaries_enabled }
}

/// Check if heap canaries are enabled
#[inline(always)]
pub fn are_heap_canaries_enabled() -> bool {
    unsafe { SECURITY_POLICY.heap_canaries_enabled }
}

/// Check if the rune is sealed
#[inline(always)]
pub fn is_rune_sealed() -> bool {
    unsafe { SECURITY_POLICY.rune_sealed }
}

/// Check if ASLR is enabled
#[inline(always)]
pub fn is_aslr_enabled() -> bool {
    unsafe { SECURITY_POLICY.aslr_enabled }
}

/// Get a reference to the security policy (for introspection)
///
/// # Safety
/// Must only be called after init()
pub unsafe fn get_policy() -> &'static SecurityPolicy {
    &SECURITY_POLICY
}

/// Display the security policy
pub fn display_policy() {
    unsafe {
        let policy = &SECURITY_POLICY;

        crate::println!();
        crate::println!("◈ Security Policy (Permanent Configuration)");
        crate::println!();
        crate::println!("  Capability-based Security: {}",
            if policy.capabilities_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("  W^X Enforcement:           {}",
            if policy.wx_enforcement { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("  Stack Canaries:            {}",
            if policy.stack_canaries_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("  Heap Canaries:             {}",
            if policy.heap_canaries_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("  ASLR:                      {}",
            if policy.aslr_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("  Rune of Permanence:        {}",
            if policy.rune_sealed { "✓ SEALED" } else { "○ UNSEALED" });
        crate::println!();

        if policy.rune_sealed {
            crate::println!("  All security policies are now IMMUTABLE (hardware-enforced).");
        } else {
            crate::println!("  Security policies will become immutable after sealing.");
        }
    }
}
