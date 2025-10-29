//! Interrupt handler implementations
//!
//! These handlers are called when hardware or CPU exceptions occur.

use core::arch::{asm, naked_asm};

//
// Exception Handlers (0-31)
// These are disharmony alerts from the CPU
//

/// Division by zero - mathematical disharmony
#[unsafe(naked)]
pub extern "C" fn divide_by_zero_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "call {inner}",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "iretq",
            inner = sym divide_by_zero_inner,
        );
    }
}

extern "C" fn divide_by_zero_inner() {
    crate::println!("❖ Disharmony: Division by zero attempted");
    crate::println!("❖ The void cannot be divided.");
}

/// General Protection Fault - privilege violation
#[unsafe(naked)]
pub extern "C" fn general_protection_fault_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "call {inner}",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "add rsp, 8",
            "iretq",
            inner = sym general_protection_fault_inner,
        );
    }
}

extern "C" fn general_protection_fault_inner() {
    crate::println!("❖ Disharmony: General protection fault");
    crate::println!("❖ A boundary has been crossed without permission.");
}

/// Page Fault - memory seeking
#[unsafe(naked)]
pub extern "C" fn page_fault_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "call {inner}",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "add rsp, 8",
            "iretq",
            inner = sym page_fault_inner,
        );
    }
}

extern "C" fn page_fault_inner() {
    let faulting_address: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) faulting_address, options(nostack, preserves_flags));
    }
    crate::println!("❖ Disharmony: Page fault at address {:#x}", faulting_address);
    crate::println!("❖ Memory seeks what has not been granted.");
}

/// Double Fault - cascade of disharmony
#[unsafe(naked)]
pub extern "C" fn double_fault_handler() {
    unsafe {
        naked_asm!(
            "cli",
            "call {inner}",
            inner = sym double_fault_inner,
        );
    }
}

extern "C" fn double_fault_inner() -> ! {
    crate::println!("❖❖❖ CRITICAL DISHARMONY: Double Fault ❖❖❖");
    crate::println!("❖ The system has lost its balance.");
    crate::println!("❖ The Heartwood must rest and be reborn.");
    loop {
        unsafe { asm!("hlt", options(nostack, nomem)) };
    }
}

// Stub handlers for other exceptions
macro_rules! stub_exception_handler {
    ($name:ident, $message:expr) => {
        #[unsafe(naked)]
        pub extern "C" fn $name() {
            unsafe {
                naked_asm!(
                    "iretq"
                );
            }
        }
    };
}

stub_exception_handler!(debug_handler, "Debug exception");
stub_exception_handler!(nmi_handler, "Non-maskable interrupt");
stub_exception_handler!(breakpoint_handler, "Breakpoint");
stub_exception_handler!(overflow_handler, "Overflow");
stub_exception_handler!(bound_range_handler, "Bound range exceeded");
stub_exception_handler!(invalid_opcode_handler, "Invalid opcode");
stub_exception_handler!(device_not_available_handler, "Device not available");
stub_exception_handler!(invalid_tss_handler, "Invalid TSS");
stub_exception_handler!(segment_not_present_handler, "Segment not present");
stub_exception_handler!(stack_segment_fault_handler, "Stack segment fault");
stub_exception_handler!(x87_floating_point_handler, "x87 FPU error");
stub_exception_handler!(alignment_check_handler, "Alignment check");
stub_exception_handler!(machine_check_handler, "Machine check");
stub_exception_handler!(simd_floating_point_handler, "SIMD floating point");
stub_exception_handler!(virtualization_handler, "Virtualization exception");
stub_exception_handler!(security_exception_handler, "Security exception");

//
// Hardware Interrupt Handlers (32-47)
// These are invitations from hardware
//

/// Timer interrupt - The Pulse of Time
#[unsafe(naked)]
pub extern "C" fn timer_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "call {inner}",
            "mov al, 0x20",
            "out 0x20, al",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "iretq",
            inner = sym timer_inner,
        );
    }
}

extern "C" fn timer_inner() {
    // Will be connected to PIT driver
    crate::attunement::timer::on_tick();
}

/// Keyboard interrupt - The Voice of Intent
#[unsafe(naked)]
pub extern "C" fn keyboard_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "call {inner}",
            "mov al, 0x20",
            "out 0x20, al",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "iretq",
            inner = sym keyboard_inner,
        );
    }
}

extern "C" fn keyboard_inner() {
    // Handle keyboard interrupt via the keyboard driver
    crate::attunement::keyboard::on_interrupt();
}

// Stub handlers for other hardware interrupts
macro_rules! stub_irq_handler {
    ($name:ident, $irq:expr) => {
        #[unsafe(naked)]
        pub extern "C" fn $name() {
            unsafe {
                naked_asm!(
                    "push rax",
                    "mov al, 0x20",
                    "out 0x20, al",
                    concat!(".if ", stringify!($irq), " >= 8"),
                    "out 0xA0, al",
                    ".endif",
                    "pop rax",
                    "iretq",
                );
            }
        }
    };
}

stub_irq_handler!(cascade_handler, 2);
stub_irq_handler!(com2_handler, 3);
stub_irq_handler!(com1_handler, 4);
stub_irq_handler!(lpt2_handler, 5);
stub_irq_handler!(floppy_handler, 6);
stub_irq_handler!(lpt1_handler, 7);
stub_irq_handler!(rtc_handler, 8);
stub_irq_handler!(acpi_handler, 9);
stub_irq_handler!(available1_handler, 10);
stub_irq_handler!(available2_handler, 11);
stub_irq_handler!(mouse_handler, 12);
stub_irq_handler!(fpu_handler, 13);
stub_irq_handler!(ata1_handler, 14);
stub_irq_handler!(ata2_handler, 15);

/// System call handler (Int 0x80)
#[unsafe(naked)]
pub extern "C" fn syscall_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "call {inner}",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "iretq",
            inner = sym syscall_inner,
        );
    }
}

extern "C" fn syscall_inner() {
    crate::println!("❖ System call received (not yet implemented)");
}
