/// PERMANENCE - Display The Rune of Permanence status
///
/// This command shows detailed information about kernel structures protected
/// by The Rune of Permanence, including sealing status, protected structures,
/// and memory layout.

use crate::mana_pool::rune_of_permanence;

/// Entry point for the permanence command - sets up paging
pub fn cmd_permanence() {
    unsafe {
        crate::eldarin::PAGING_ACTIVE = true;
        crate::eldarin::PAGING_PAGE = 0;
        crate::eldarin::PAGING_COMMAND = Some(crate::eldarin::PagingCommand::Permanence);
    }
    show_permanence_page(0);
}

/// Display a specific page of permanence information
pub fn show_permanence_page(page: usize) {
    match page {
        0 => show_overview_page(),
        1 => show_protected_structures_page(),
        2 => show_security_policy_page(),
        _ => {
            // No more pages
            unsafe {
                crate::eldarin::PAGING_ACTIVE = false;
                crate::eldarin::PAGING_PAGE = 0;
                crate::eldarin::PAGING_COMMAND = None;
            }
            crate::eldarin::display_prompt();
        }
    }
}

/// Page 0: Overview of The Rune of Permanence
fn show_overview_page() {
    let (start, end) = rune_of_permanence::get_rune_boundaries();
    let size = rune_of_permanence::get_rune_size();
    let pages = rune_of_permanence::get_rune_page_count();
    let sealed = rune_of_permanence::is_sealed();

    crate::println!();
    crate::println!("◈ The Rune of Permanence - Immutable Kernel Structures");
    crate::println!();
    crate::println!("  \"The fundamental laws of the realm, once scribed at the Dawn of");
    crate::println!("   Awakening, are immutable. These crystalline structures, etched");
    crate::println!("   into the fabric of reality, cannot be altered—for to change them");
    crate::println!("   would be to rewrite the very physics of the world.\"");
    crate::println!();

    // Status
    crate::println!("  Status: {}", if sealed { "✓ SEALED" } else { "○ UNSEALED (boot in progress)" });
    crate::println!("  Protection: MMU Read-Only Pages (hardware-enforced)");
    crate::println!();

    // Memory layout
    crate::println!("  .rune Section:");
    crate::println!("    Address:  0x{:016x} - 0x{:016x}", start, end);
    crate::println!("    Size:     {} bytes ({} KB)", size, size / 1024);
    crate::println!("    Pages:    {} (4KB pages)", pages);
    crate::println!();

    if sealed {
        crate::println!("  The Rune is sealed. All structures within are immutable.");
        crate::println!("  Any attempt to modify them will trigger a page fault.");
    } else {
        crate::println!("  The Rune awaits sealing after initialization.");
    }

    crate::println!();
    crate::println!("Press SPACE for protected structures, or ESC to exit");
}

/// Page 1: Protected structures details
fn show_protected_structures_page() {
    let sealed = rune_of_permanence::is_sealed();

    crate::println!();
    crate::println!("◈ Protected Structures (Page 2/3)");
    crate::println!();

    // IDT
    crate::println!("  ◈ Interrupt Descriptor Table (IDT)");
    crate::println!("    Location: .rune section");
    crate::println!("    Size: 4096 bytes (256 entries × 16 bytes)");
    crate::println!("    Purpose: Maps hardware/software interrupts to handlers");
    crate::println!("    Status: {}", if sealed { "✓ Sealed (cannot modify handlers)" } else { "○ Unsealed" });
    crate::println!();
    crate::println!("    Protected Against:");
    crate::println!("      - Interrupt handler hijacking");
    crate::println!("      - Data-only attacks on control flow");
    crate::println!("      - Rootkit interrupt hooking");
    crate::println!();

    // GDT & TSS
    crate::println!("  ◈ Global Descriptor Table (GDT) & TSS");
    crate::println!("    Location: .rune section");
    crate::println!("    Size: {} bytes (GDT) + {} bytes (TSS)",
        core::mem::size_of::<crate::attunement::gdt::GlobalDescriptorTable>(),
        core::mem::size_of::<crate::attunement::gdt::TaskStateSegment>());
    crate::println!("    Purpose: Define privilege boundaries and task state");
    crate::println!("    Status: {}", if sealed { "✓ Sealed (cannot modify segments)" } else { "○ Unsealed" });
    crate::println!();
    crate::println!("    Protected Against:");
    crate::println!("      - Privilege escalation via segment modification");
    crate::println!("      - Kernel/user boundary corruption");
    crate::println!("      - Ring 0 privilege tampering");
    crate::println!();

    // Security Policy
    crate::println!("  ◈ Security Policy");
    crate::println!("    Location: .rune section");
    crate::println!("    Size: {} bytes", core::mem::size_of::<crate::mana_pool::security_policy::SecurityPolicy>());
    crate::println!("    Purpose: Critical security feature flags");
    crate::println!("    Status: {}", if sealed { "✓ Sealed (cannot disable protections)" } else { "○ Unsealed" });
    crate::println!();
    crate::println!("    Protected Against:");
    crate::println!("      - Security policy corruption");
    crate::println!("      - Protection bypass via flag tampering");
    crate::println!("      - Disable-security-and-exploit attacks");
    crate::println!();

    // Test structures
    crate::println!("  ◈ Verification Test Variable");
    crate::println!("    Location: .rune section");
    crate::println!("    Size: 8 bytes");
    crate::println!("    Purpose: Verify .rune mechanism works");
    crate::println!("    Status: {}", if sealed { "✓ Sealed" } else { "○ Unsealed" });
    crate::println!();

    crate::println!("Press SPACE for security policy details, or ESC to exit");
}

/// Page 2: Security policy details
fn show_security_policy_page() {
    crate::println!();
    crate::println!("◈ Security Policy Configuration (Page 3/3)");
    crate::println!();

    // Use the security_policy module's display function
    unsafe {
        let policy = crate::mana_pool::security_policy::get_policy();

        crate::println!("  Configuration Flags (all immutable after sealing):");
        crate::println!();
        crate::println!("    Capability-based Security: {}",
            if policy.capabilities_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("      Enforces capability-based access control for memory objects");
        crate::println!();

        crate::println!("    W^X Enforcement:           {}",
            if policy.wx_enforcement { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("      Prevents pages from being both writable and executable");
        crate::println!();

        crate::println!("    Stack Canaries:            {}",
            if policy.stack_canaries_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("      LLVM strong mode - per-function canaries");
        crate::println!();

        crate::println!("    Heap Canaries:             {}",
            if policy.heap_canaries_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("      Pre/post allocation guards (8 bytes each)");
        crate::println!();

        crate::println!("    ASLR:                      {}",
            if policy.aslr_enabled { "✓ ENABLED" } else { "✗ DISABLED" });
        crate::println!("      Address Space Layout Randomization");
        crate::println!();

        crate::println!("    Rune of Permanence:        {}",
            if policy.rune_sealed { "✓ SEALED" } else { "○ UNSEALED" });
        crate::println!("      Hardware-enforced immutability for critical structures");
        crate::println!();
    }

    crate::println!("  Security Note:");
    crate::println!("    These flags are stored in the .rune section. After sealing,");
    crate::println!("    they cannot be modified by ANY code, including the kernel.");
    crate::println!("    This prevents data-only attacks from disabling protections.");
    crate::println!();

    crate::println!("Press ESC to exit, or SPACE to return to page 1");
}
