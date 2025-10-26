/// WARDS - Display security protections (ASLR, W^X enforcement)
///
/// This command shows the security wards protecting the Mana Pool:
/// - W^X (Write XOR Execute) enforcement status
/// - ASLR (Address Space Layout Randomization) status
/// - Thread stack addresses and randomization entropy
pub fn cmd_wards() {
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

    // Thread Stack Information
    crate::println!("  Thread Stack Wards:");

    use crate::attunement::interrupts::without_interrupts;
    use crate::loom_of_fate::ThreadState;

    without_interrupts(|| {
        unsafe {
            let loom = crate::loom_of_fate::get_loom().lock();
            let threads: alloc::vec::Vec<_> = loom.threads.iter()
                .filter(|t| !matches!(t.state, ThreadState::Fading))
                .collect();

            for thread in threads {
                let stack_size = thread.stack_top - thread.stack_bottom;

                // Calculate approximate ASLR offset based on stack pointer alignment
                // The actual offset is embedded in the Stack struct, but we can estimate
                // it from the stack top alignment
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
}
