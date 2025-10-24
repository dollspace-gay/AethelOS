# Makefile for AethelOS
# The build incantations for awakening the system

.PHONY: all clean bootloader kernel groves runes test qemu

# Colors for output
COLOR_GREEN=\033[0;32m
COLOR_BLUE=\033[0;34m
COLOR_CYAN=\033[0;36m
COLOR_RESET=\033[0m

all: bootloader kernel groves runes
	@echo "$(COLOR_GREEN)[] AethelOS build complete!$(COLOR_RESET)"

# Build the bootloader
bootloader:
	@echo "$(COLOR_CYAN)[] Awakening the bootloader...$(COLOR_RESET)"
	nasm -f bin awakening/boot.asm -o build/boot.bin
	@echo "$(COLOR_GREEN)[] Bootloader ready$(COLOR_RESET)"

# Build the Heartwood kernel
kernel:
	@echo "$(COLOR_CYAN)[] Building the Heartwood...$(COLOR_RESET)"
	cd heartwood && cargo build --release
	@echo "$(COLOR_GREEN)[] Heartwood compiled$(COLOR_RESET)"

# Build all Grove services
groves:
	@echo "$(COLOR_CYAN)[] Growing the Groves...$(COLOR_RESET)"
	cd groves/world-tree_grove && cargo build --release
	cd groves/the-weave_grove && cargo build --release
	cd groves/lanthir_grove && cargo build --release
	cd groves/network_sprite && cargo build --release
	@echo "$(COLOR_GREEN)[] Groves flourishing$(COLOR_RESET)"

# Build Ancient Runes libraries
runes:
	@echo "$(COLOR_CYAN)[] Inscribing the Ancient Runes...$(COLOR_RESET)"
	cd ancient-runes/corelib && cargo build --release
	cd ancient-runes/weaving && cargo build --release
	cd ancient-runes/script && cargo build --release
	@echo "$(COLOR_GREEN)[] Runes inscribed$(COLOR_RESET)"

# Run tests
test:
	@echo "$(COLOR_CYAN)[] Testing the Heartwood...$(COLOR_RESET)"
	cd heartwood && cargo test
	@echo "$(COLOR_GREEN)[] Tests complete$(COLOR_RESET)"

# Create build directory
build:
	mkdir -p build

# Run in QEMU (requires bootable image)
qemu: build bootloader
	@echo "$(COLOR_BLUE)[] Awakening in QEMU...$(COLOR_RESET)"
	qemu-system-x86_64 -drive format=raw,file=build/boot.bin

# Clean build artifacts
clean:
	@echo "$(COLOR_CYAN)[] Returning to the void...$(COLOR_RESET)"
	cargo clean
	rm -rf build
	@echo "$(COLOR_GREEN)[] Clean complete$(COLOR_RESET)"

# Check code formatting
fmt:
	cargo fmt --all

# Run clippy lints
clippy:
	cargo clippy --all-targets --all-features

# Generate documentation
docs:
	cargo doc --workspace --no-deps --open

# Show build information
info:
	@echo "$(COLOR_BLUE)=====================================$(COLOR_RESET)"
	@echo "$(COLOR_BLUE)    AethelOS Build System$(COLOR_RESET)"
	@echo "$(COLOR_BLUE)=====================================$(COLOR_RESET)"
	@echo "Version: 0.1.0-alpha"
	@echo "Language: Rust $(shell rustc --version | cut -d' ' -f2)"
	@echo ""
	@echo "Available targets:"
	@echo "  all        - Build everything"
	@echo "  bootloader - Build the awakening sequence"
	@echo "  kernel     - Build the Heartwood"
	@echo "  groves     - Build all Grove services"
	@echo "  runes      - Build Ancient Runes libraries"
	@echo "  test       - Run tests"
	@echo "  qemu       - Run in QEMU emulator"
	@echo "  clean      - Remove build artifacts"
	@echo "  fmt        - Format code"
	@echo "  clippy     - Run lints"
	@echo "  docs       - Generate documentation"
	@echo "$(COLOR_BLUE)=====================================$(COLOR_RESET)"
