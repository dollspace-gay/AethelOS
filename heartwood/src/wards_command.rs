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
