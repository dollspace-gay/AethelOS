//! # 32-bit Bootstrap Code
//!
//! Multiboot2 bootloaders (like GRUB) load the kernel in 32-bit protected mode.
//! This module transitions from 32-bit protected mode to 64-bit long mode.

use core::arch::global_asm;

global_asm!(
    r#"
    .section .boot.text, "awx"
    .code32
    .globl boot32_start

    boot32_start:
        # Set up stack FIRST - use a large stack well above kernel
        # Kernel is at 1MB (0x100000), stack grows down from 2MB (0x200000)
        # This gives us ~1MB of stack space
        mov esp, 0x200000
        mov ebp, esp

        # Write 'B' to serial port to prove we're executing
        mov dx, 0x3f8
        mov al, 66        # 'B'
        out dx, al

        # Disable interrupts for the rest of bootstrap
        cli

        # Set up page tables for identity mapping (first 1GB using 2MB pages)
        # Clear the page table area (16KB from 0x70000-0x73FFF)
        mov edi, 0x70000
        mov ecx, 0x1000   # 4096 dwords = 16KB
        xor eax, eax
        rep stosd

        # PML4[0] -> PDPT at 0x71000
        mov dword ptr [0x70000], 0x71003  # Present + Write

        # PDPT[0] -> PD at 0x72000 (maps first 1GB)
        mov dword ptr [0x71000], 0x72003  # Present + Write

        # Map first 512 entries in PD (512 * 2MB = 1GB)
        mov edi, 0x72000
        mov eax, 0x83       # Present + Write + Huge (2MB), starting at 0
        mov ecx, 512        # 512 entries
    1:
        mov [edi], eax
        add eax, 0x200000   # Next 2MB page
        add edi, 8
        loop 1b

        # Load CR3 with PML4 address
        mov eax, 0x70000
        mov cr3, eax

        # Enable PAE (Physical Address Extension) in CR4
        mov eax, cr4
        or eax, (1 << 5)    # CR4.PAE = 1
        mov cr4, eax

        # Set Long Mode bit in EFER MSR
        mov ecx, 0xC0000080 # EFER MSR
        rdmsr
        or eax, (1 << 8)    # EFER.LME = 1 (Long Mode Enable)
        wrmsr

        # Enable paging to activate long mode
        mov eax, cr0
        or eax, (1 << 31)   # CR0.PG = 1
        mov cr0, eax

        # === AWAKEN SSE & SSE2 ===
        # Enable SSE before jumping to 64-bit mode (required by x86-64 ABI)
        # This prevents #UD (Invalid Opcode) exceptions when Rust uses SSE instructions

        # Clear EM (Emulation) bit and set MP (Monitor Coprocessor) bit in CR0
        mov eax, cr0
        and ax, 0xFFFB      # Clear EM bit (bit 2) - no x87 emulation
        or ax, 0x2          # Set MP bit (bit 1) - monitor coprocessor
        mov cr0, eax

        # Set OSFXSR and OSXMMEXCPT bits in CR4
        mov eax, cr4
        or ax, 0x600        # Set OSFXSR (bit 9) and OSXMMEXCPT (bit 10)
        mov cr4, eax        # OS supports FXSAVE/FXRSTOR and SSE exceptions

        # Write 'S' to serial to indicate SSE is now enabled
        mov dx, 0x3f8
        mov al, 83          # 'S' for SSE enabled
        out dx, al

        # Now in compatibility mode - load 64-bit GDT
        lgdt [gdt64_pointer]

        # Far jump to 64-bit code segment (using push+ret trick)
        push 0x08
        lea eax, [boot64_start]
        push eax
        retf

    # 64-bit GDT
    .align 8
    gdt64:
        .quad 0                           # Null descriptor
        .quad 0x00AF9A000000FFFF          # 64-bit code segment (selector 0x08)
        .quad 0x00AF92000000FFFF          # 64-bit data segment (selector 0x10)
    gdt64_pointer:
        .word gdt64_pointer - gdt64 - 1   # Limit
        .quad gdt64                        # Base

    # 64-bit entry point
    .code64
    boot64_start:
        # Set up data segments
        mov ax, 0x10
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov ss, ax

        # Write 'L' to serial to indicate we're in long mode
        mov dx, 0x3f8
        mov al, 76        # 'L'
        out dx, al

        # Test: Write directly to VGA text buffer at 0xB8000
        # Write "OK" in white-on-blue (attribute 0x1F) to top-left corner
        mov rax, 0xB8000
        mov word ptr [rax], 0x1F4F      # 'O' in white on blue
        mov word ptr [rax + 2], 0x1F4B  # 'K' in white on blue

        # Set up a proper stack before calling Rust (same 2MB location)
        mov rsp, 0x200000
        mov rbp, rsp

        # Write 'S' to serial after stack setup
        mov dx, 0x3f8
        mov al, 83        # 'S'
        out dx, al

        # Write 'R' to serial before calling Rust
        mov dx, 0x3f8
        mov al, 82        # 'R'
        out dx, al

        # Call the Rust kernel entry point
        call _start

        # Write 'X' to serial if _start returns (it shouldn't)
        mov dx, 0x3f8
        mov al, 88        # 'X'
        out dx, al

        # If _start returns (it shouldn't), halt
    halt_loop:
        hlt
        jmp halt_loop
    "#
);
