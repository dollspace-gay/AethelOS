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
  -hda aethelos-test.img ^
  -boot d ^
  -serial file:serial.log ^
  -m 256M ^
  -display gtk ^
  -no-reboot ^
  -no-shutdown ^
  -d guest_errors

echo.
echo QEMU exited.
pause
