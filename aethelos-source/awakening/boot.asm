; boot.asm - The First Spark
; This is the very first code that runs when AethelOS awakens
; It initializes the hardware and prepares for the Heartwood to be loaded

bits 16
org 0x7c00

section .boot
    ; The awakening begins
    cli                     ; Clear interrupts
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7c00         ; Set up stack

    ; Print awakening message
    mov si, msg_awakening
    call print_string

    ; Load the Heartwood loader from disk
    ; In a real implementation, this would:
    ; 1. Read additional sectors from disk
    ; 2. Enter protected mode
    ; 3. Set up paging
    ; 4. Jump to the Rust Heartwood loader

    ; For now, just halt
    mov si, msg_complete
    call print_string

halt:
    hlt
    jmp halt

; Print a null-terminated string
print_string:
    pusha
.loop:
    lodsb                  ; Load byte from SI into AL
    test al, al            ; Check if zero
    jz .done
    mov ah, 0x0e          ; BIOS teletype function
    int 0x10              ; BIOS video interrupt
    jmp .loop
.done:
    popa
    ret

msg_awakening: db '[] The First Spark...', 13, 10, 0
msg_complete:  db '[] Boot sector complete.', 13, 10, 0

; Boot signature
times 510-($-$$) db 0
dw 0xaa55
