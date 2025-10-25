#!/bin/bash
# Verify multiboot2 header in the kernel binary

KERNEL="target/x86_64-aethelos/debug/heartwood"

if [ ! -f "$KERNEL" ]; then
    echo "Error: Kernel binary not found at $KERNEL"
    exit 1
fi

echo "Checking for Multiboot2 magic number (0xE85250D6)..."

# Search for the magic number in first 32KB
if od -An -tx4 -N32768 "$KERNEL" | grep -q "e85250d6"; then
    echo "✓ Multiboot2 magic found!"

    # Show where it's located
    OFFSET=$(od -An -tx4 "$KERNEL" | grep -n "e85250d6" | head -1 | cut -d: -f1)
    BYTE_OFFSET=$(( ($OFFSET - 1) * 16 ))
    echo "  Location: ~$BYTE_OFFSET bytes from start"

    if [ $BYTE_OFFSET -lt 32768 ]; then
        echo "  ✓ Within required 32KB limit"
    else
        echo "  ✗ WARNING: Beyond 32KB limit!"
    fi

    echo ""
    echo "Multiboot2 header structure:"
    od -An -tx4 -N32 "$KERNEL" | grep -A3 "e85250d6" | head -4

    echo ""
    echo "✓ Kernel appears to be multiboot2-compliant!"
    exit 0
else
    echo "✗ Multiboot2 magic NOT found in first 32KB"
    echo "  The kernel may not boot with GRUB"
    exit 1
fi
