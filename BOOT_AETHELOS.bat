@echo off
echo.
echo ═══════════════════════════════════════════════════════
echo   Booting AethelOS - The Symbiotic Operating System
echo ═══════════════════════════════════════════════════════
echo.
echo Starting QEMU with AethelOS ISO...
echo.

"C:\Program Files\qemu\qemu-system-x86_64.exe" ^
  -cdrom aethelos.iso ^
  -hda aethelos-test-ext4.img ^
  -boot d ^
  -serial file:serial.log ^
  -m 1024M ^
  -display gtk ^
  -no-reboot ^
  -cpu max ^
  -no-shutdown ^
  -D qemu-debug.log ^
  -d int,cpu_reset,guest_errors 

echo.
echo QEMU exited.
pause
