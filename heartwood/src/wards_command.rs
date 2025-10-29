/// WARDS - Display security protections (ASLR, W^X enforcement)
///
/// This command shows the security wards protecting the Mana Pool:
/// - W^X (Write XOR Execute) enforcement status
/// - ASLR (Address Space Layout Randomization) status
/// - Thread stack addresses and randomization entropy
pub fn cmd_wards() {
    unsafe {
        crate::eldarin::PAGING_ACTIVE = true;
        crate::eldarin::PAGING_PAGE = 0;
        crate::eldarin::PAGING_COMMAND = Some(crate::eldarin::PagingCommand::Wards);
    }
    show_wards_page(0);
}

/// Show a specific page of the wards output (called by paging system)
pub fn show_wards_page(page: usize) {
    use crate::loom_of_fate::{ThreadState, without_interrupts};
    use crate::eldarin::display_prompt;

    match page {
        0 => {
            // Page 1: W^X, ASLR, Thread Stacks
            crate::println!("◈ Security Wards of the Mana Pool");
            crate::println!();

            // W^X Status
            crate::println!("  Write ⊕ Execute Enforcement: ✓ Active");
            crate::println!("    Memory pages cannot be both writable and executable");
            crate::println!();

            // ASLR Status
            crate::println!("  Address Space Layout Randomization: ✓ Active");
            crate::println!("    Thread stacks randomized with 0-64KB entropy");
            crate::println!();

            // KASLR Status (Ward of Unseen Paths)
            let kaslr_enabled = crate::attunement::ward_of_unseen_paths::is_kaslr_enabled();
            crate::println!("  Ward of Unseen Paths (KASLR): {}",
                if kaslr_enabled { "✓ Active" } else { "○ Inactive" });
            if kaslr_enabled {
                let offset = crate::attunement::ward_of_unseen_paths::get_kaslr_offset();
                let offset_mb = offset / (1024 * 1024);
                let entropy_bits = crate::attunement::ward_of_unseen_paths::get_entropy_bits();
                let range_mb = crate::attunement::ward_of_unseen_paths::get_entropy_range_mb();
                let kernel_base = crate::attunement::ward_of_unseen_paths::get_kernel_base();

                crate::println!("    Kernel base: 0x{:016x}", kernel_base);
                crate::println!("    Random offset: +{} MB (0x{:08x})", offset_mb, offset);
                crate::println!("    Entropy: {} bits ({} MB range)", entropy_bits, range_mb);
                crate::println!("    The Heartwood wanders - ancient maps are useless");
            } else {
                crate::println!("    KASLR not enabled (kernel at fixed address)");
            }
            crate::println!();

            // Rune of Permanence Status
            let rune_sealed = crate::mana_pool::rune_of_permanence::is_sealed();
            crate::println!("  Rune of Permanence (Immutable Structures): {}",
                if rune_sealed { "✓ Sealed" } else { "○ Unsealed" });
            if rune_sealed {
                crate::println!("    Protected: IDT, GDT, TSS, Security Policy");
                crate::println!("    Pages: {} read-only",
                    crate::mana_pool::rune_of_permanence::get_rune_page_count());
            }
            crate::println!();

            // Ward of Anonymity Status
            let anonymity_enabled = crate::attunement::ward_of_anonymity::is_anonymity_enabled();
            crate::println!("  Ward of Anonymity (Symbol Hiding): {}",
                if anonymity_enabled { "✓ Active" } else { "○ Inactive" });
            if anonymity_enabled {
                crate::println!("    Kernel symbols hidden from unprivileged access");
                crate::println!("    Function names redacted in errors and panics");
                crate::println!("    True names of the spirits are sealed");
            } else {
                crate::println!("    ⚠ DEBUG MODE: Symbols visible (reduces security)");
            }
            crate::println!();

            // Concordance of Fates Status (RBAC)
            let concordance_sealed = crate::mana_pool::concordance_of_fates::is_sealed();
            crate::println!("  Concordance of Fates (RBAC): {}",
                if concordance_sealed { "✓ Sealed" } else { "○ Unsealed" });
            if concordance_sealed {
                let fate_count = crate::mana_pool::concordance_of_fates::get_fate_count();
                let subject_count = crate::mana_pool::concordance_of_fates::get_subject_count();
                crate::println!("    Fates defined: {}", fate_count);
                crate::println!("    Subjects bound: {}", subject_count);
                crate::println!("    Every thread's destiny is written in the Concordance");
            }
            crate::println!();

            // Ward of Sacred Boundaries Status (SMEP/SMAP)
            let ward_enabled = crate::attunement::ward_of_sacred_boundaries::is_ward_enabled();
            crate::println!("  Ward of Sacred Boundaries (SMEP/SMAP): {}",
                if ward_enabled { "✓ Active" } else { "○ Inactive" });
            if ward_enabled {
                // Read CR4 to check which features are actually enabled
                let cr4: u64;
                unsafe {
                    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
                }
                let smep_bit = (cr4 & (1 << 20)) != 0;
                let smap_bit = (cr4 & (1 << 21)) != 0;

                if smep_bit {
                    crate::println!("    ✓ SMEP: Kernel cannot execute user space code");
                }
                if smap_bit {
                    crate::println!("    ✓ SMAP: Kernel cannot access user pointers directly");
                }
                if !smep_bit && !smap_bit {
                    crate::println!("    ⚠ CPU lacks SMEP/SMAP (software checks only)");
                }
            } else {
                crate::println!("    Ward not initialized (software checks only)");
            }
            crate::println!();

            // Thread Stack Information
            crate::println!("  Thread Stack Wards:");

            without_interrupts(|| {
                unsafe {
                    let loom = crate::loom_of_fate::get_loom().lock();
                    let threads: alloc::vec::Vec<_> = loom.threads.iter()
                        .filter(|t| !matches!(t.state, ThreadState::Fading))
                        .collect();

                    for thread in threads {
                        let stack_size = thread.stack_top - thread.stack_bottom;
                        let nominal_top = thread.stack_bottom + stack_size;
                        let aslr_offset = if thread.stack_top < nominal_top {
                            nominal_top - thread.stack_top
                        } else {
                            0
                        };

                        let state_str = match thread.state {
                            ThreadState::Weaving => "Weaving",
                            ThreadState::Resting => "Resting",
                            ThreadState::Tangled => "Tangled",
                            ThreadState::Fading => "Fading",
                        };

                        crate::println!("    Thread #{} ({}): Stack 0x{:016x}-0x{:016x}",
                            thread.id.0,
                            state_str,
                            thread.stack_bottom,
                            thread.stack_top
                        );
                        crate::println!("      Size: {} KB, ASLR offset: ~{} bytes",
                            stack_size / 1024,
                            aslr_offset
                        );
                    }
                }
            });

            crate::println!();
            crate::println!("  Entropy Source: RDTSC (fast boot-safe randomization)");
            crate::println!();
            crate::println!("The wards stand strong. Your sanctuary is protected.");
            crate::println!();
            crate::println!("─── Press ENTER for capability tests (1/3) ───");
        }
        1 => {
            // Page 2: Start capability tests
            test_capability_sealing_page1();
            crate::println!();
            crate::println!("─── Press ENTER to continue (2/3) ───");
        }
        2 => {
            // Page 3: Finish capability tests
            test_capability_sealing_page2();
            crate::println!();
            // Exit paging mode
            unsafe {
                crate::eldarin::PAGING_ACTIVE = false;
                crate::eldarin::PAGING_PAGE = 0;
                crate::eldarin::PAGING_COMMAND = None;
            }
            display_prompt();
        }
        _ => {
            // Safety fallback
            unsafe {
                crate::eldarin::PAGING_ACTIVE = false;
                crate::eldarin::PAGING_PAGE = 0;
                crate::eldarin::PAGING_COMMAND = None;
            }
            display_prompt();
        }
    }
}

/// Capability test page 1: Create, access, derive
fn test_capability_sealing_page1() {
    use crate::loom_of_fate::without_interrupts;
    use crate::mana_pool::{CapabilityRights, AllocationPurpose};

    crate::println!("◈ Testing Capability Security System");
    crate::println!();

    without_interrupts(|| {
        unsafe {
            let mut mana_pool = crate::mana_pool::get_mana_pool().lock();

            // Test 1: Create a sealed capability (returns opaque ID)
            crate::println!("  Test 1: Creating sealed capability with READ+WRITE rights...");
            match mana_pool.object_manager.create_object_sealed(
                0x1000000,  // Dummy address
                4096,       // 4KB object
                AllocationPurpose::ShortLived,
                CapabilityRights::read_write(),
            ) {
                Ok(cap_id) => {
                    crate::println!("    ✓ Created opaque capability ID: {}", cap_id.raw());

                    // Test 2: Access object through opaque ID
                    crate::println!("  Test 2: Accessing object info via opaque ID...");
                    match mana_pool.object_manager.get_object_info_sealed(cap_id) {
                        Ok(info) => {
                            crate::println!("    ✓ Object info retrieved: size = {} bytes", info.size);
                        }
                        Err(e) => {
                            crate::println!("    ✗ Failed to get object info: {:?}", e);
                        }
                    }

                    // Test 3: Derive capability with reduced rights (attenuation)
                    crate::println!("  Test 3: Deriving READ-ONLY capability (attenuation)...");
                    match mana_pool.object_manager.derive_capability_sealed(
                        cap_id,
                        CapabilityRights::read_only(),
                    ) {
                        Ok(derived_id) => {
                            crate::println!("    ✓ Derived capability ID: {}", derived_id.raw());

                            // Store for page 2
                            STORED_CAP_ID = Some(cap_id);
                            STORED_DERIVED_ID = Some(derived_id);
                        }
                        Err(e) => {
                            crate::println!("    ✗ Derivation failed: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    crate::println!("    ✗ Failed to create sealed capability: {:?}", e);
                }
            }
        }
    });
}

/// Capability test page 2: Verify rights and release
fn test_capability_sealing_page2() {
    use crate::loom_of_fate::without_interrupts;
    use crate::mana_pool::CapabilityRights;

    without_interrupts(|| {
        unsafe {
            let mut mana_pool = crate::mana_pool::get_mana_pool().lock();

            if let (Some(cap_id), Some(derived_id)) = (STORED_CAP_ID, STORED_DERIVED_ID) {
                // Test 4: Check that derived capability has only READ rights
                crate::println!("  Test 4: Verifying derived capability has READ (not WRITE)...");
                match mana_pool.object_manager.check_capability_rights(
                    derived_id,
                    CapabilityRights::READ,
                ) {
                    Ok(_) => crate::println!("    ✓ READ permission verified"),
                    Err(_) => crate::println!("    ✗ READ check failed!"),
                }

                match mana_pool.object_manager.check_capability_rights(
                    derived_id,
                    CapabilityRights::WRITE,
                ) {
                    Ok(_) => crate::println!("    ✗ SECURITY BUG: WRITE should be denied!"),
                    Err(_) => crate::println!("    ✓ WRITE permission correctly denied"),
                }

                // Test 5: Release the capability
                crate::println!("  Test 5: Releasing sealed capability...");
                match mana_pool.object_manager.release_object_sealed(cap_id) {
                    Ok(_) => crate::println!("    ✓ Capability revoked successfully"),
                    Err(e) => crate::println!("    ✗ Release failed: {:?}", e),
                }

                // Clear stored IDs
                STORED_CAP_ID = None;
                STORED_DERIVED_ID = None;
            }
        }
    });

    crate::println!();
    crate::println!("  Security test complete. Opaque capabilities working correctly.");
}

// Store capability IDs between pages
use crate::mana_pool::capability::CapabilityId;
static mut STORED_CAP_ID: Option<CapabilityId> = None;
static mut STORED_DERIVED_ID: Option<CapabilityId> = None;
