@echo off
REM Simple launcher - just double-click this file!

echo Building AethelOS kernel...
cd /d "%~dp0\heartwood"

cargo build --bin heartwood --target x86_64-aethelos.json

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Build failed!
    pause
    exit /b 1
)

cd ..
echo.
echo Booting in QEMU with multiboot support...
echo.
echo Note: Output will appear in the QEMU window (VGA text mode)
echo.

REM QEMU has built-in multiboot loader support
REM The -kernel flag with an ELF multiboot kernel should work
"C:\Program Files\qemu\qemu-system-x86_64.exe" ^
  -kernel target\x86_64-aethelos\debug\heartwood ^
  -serial stdio ^
  -m 256M ^
  -display gtk ^
  -no-reboot ^
  -no-shutdown ^
  -d guest_errors

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo QEMU failed to boot. This might be because:
    echo   1. The ELF format is not compatible
    echo   2. QEMU version doesn't support this multiboot format
    echo.
    echo Creating a GRUB ISO instead...
    echo This requires WSL or Linux tools.
    echo See BOOT_GUIDE.md for instructions.
)

pause
