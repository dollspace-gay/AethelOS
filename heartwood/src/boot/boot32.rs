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
    .extern _start

    boot32_start:
        # Set up stack FIRST - use a large stack well above kernel
        # Kernel is at 1MB (0x100000), stack grows down from 4MB (0x400000)
        # This gives us ~3MB of stack space (needed for extensive debug output)
        mov esp, 0x400000
        mov ebp, esp

        # Write 'B' to serial port to prove we're executing
        mov dx, 0x3f8
        mov al, 66        # 'B'
        out dx, al

        # Disable interrupts for the rest of bootstrap
        cli

        # Set up page tables for BOTH identity and higher-half mapping
        # We need dual mapping:
        #   - Identity (PML4[0]): For boot code and early execution
        #   - Higher-half (PML4[256]): For kernel at 0xFFFF_8000_0000_0000+
        #
        # Clear the page table area (24KB from 0x70000-0x75FFF)
        mov edi, 0x70000
        mov ecx, 0x1800   # 6144 dwords = 24KB
        xor eax, eax
        rep stosd

        # PML4 setup (at 0x70000):
        # - Entry [0] -> PDPT at 0x71000 (identity mapping)
        # - Entry [511] -> PDPT at 0x73000 (higher-half mapping in top 2GB)
        mov dword ptr [0x70000], 0x71003      # PML4[0] -> PDPT (identity)
        mov dword ptr [0x70FF8], 0x73003      # PML4[511] -> PDPT (higher-half)
                                               # 0x70FF8 = 0x70000 + (511 * 8)

        # PDPT for identity mapping (at 0x71000):
        # - Entry [0] -> PD at 0x72000 (maps first 1GB at virtual 0x0+)
        mov dword ptr [0x71000], 0x72003      # PDPT[0] -> PD

        # PDPT for higher-half mapping (at 0x73000):
        # - Entry [510] -> PD at 0x74000 (maps first 1GB at virtual 0xFFFFFFFF80000000+)
        # PDPT[510] is at offset 510*8 = 0xFF0 from base 0x73000
        mov dword ptr [0x73FF0], 0x74003      # PDPT[510] -> PD

        # PD for identity mapping (at 0x72000):
        # Map 512 * 2MB huge pages = 1GB starting at physical 0x0
        mov edi, 0x72000
        mov eax, 0x83       # Present + Write + Huge (2MB)
        mov ecx, 512        # 512 entries
    1:
        mov [edi], eax
        add eax, 0x200000   # Next 2MB page
        add edi, 8
        loop 1b

        # PD for higher-half mapping (at 0x74000):
        # Map SAME physical memory (512 * 2MB = 1GB) but at higher-half virtual address
        mov edi, 0x74000
        mov eax, 0x83       # Present + Write + Huge (2MB)
        mov ecx, 512        # 512 entries
    2:
        mov [edi], eax
        add eax, 0x200000   # Next 2MB page
        add edi, 8
        loop 2b

        # Load CR3 with PML4 address
        mov eax, 0x70000
        mov cr3, eax

        # Enable PAE (Physical Address Extension) in CR4
        mov eax, cr4
        or eax, (1 << 5)    # CR4.PAE = 1
        mov cr4, eax

        # Set Long Mode and NX bits in EFER MSR
        mov ecx, 0xC0000080 # EFER MSR
        rdmsr
        or eax, (1 << 8)    # EFER.LME = 1 (Long Mode Enable)
        or eax, (1 << 11)   # EFER.NXE = 1 (No Execute Enable - required for W^X security)
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

        # Set up HIGHER-HALF stack before calling Rust
        # Physical 4MB = Virtual 0xFFFFFFFF80400000 in top 2GB
        movabs rsp, 0xFFFFFFFF80400000
        mov rbp, rsp

        # Write 'S' to serial after stack setup
        mov dx, 0x3f8
        mov al, 83        # 'S'
        out dx, al

        # Write 'R' to serial before calling Rust
        mov dx, 0x3f8
        mov al, 82        # 'R'
        out dx, al

        # Jump to Rust _start function in higher-half kernel
        # Use RIP-relative addressing to get _start address
        lea rax, [rip + _start_addr_rip]
        mov rax, [rax]
        jmp rax

_start_addr_rip:
        .quad _start

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
