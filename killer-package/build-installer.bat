@echo off
REM ============================================================================
REM Killer Language v1.1 - NSIS Installer Compiler
REM ============================================================================
REM
REM This script compiles the NSIS script into a Windows installer executable.
REM
REM Requirements:
REM   - NSIS (download from: https://nsis.sourceforge.io/)
REM   - Install NSIS to default location: C:\Program Files\NSIS
REM
REM Usage:
REM   1. Download and install NSIS from https://nsis.sourceforge.io/
REM   2. Run this script: build-installer.bat
REM   3. Output: Killer-v1.1-Setup.exe
REM
REM ============================================================================

setlocal enabledelayedexpansion

echo.
echo ========================================================================
echo   Killer Language v1.1 - Installer Builder
echo ========================================================================
echo.

REM Check if NSIS is installed
if not exist "C:\Program Files\NSIS\makensis.exe" (
    echo [ERROR] NSIS not found!
    echo.
    echo Please download and install NSIS from:
    echo   https://nsis.sourceforge.io/
    echo.
    echo After installation, run this script again.
    echo.
    pause
    exit /b 1
)

echo [OK] Found NSIS compiler
echo [INFO] Compiling killer-installer.nsi...
echo.

REM Compile the NSIS script
"C:\Program Files\NSIS\makensis.exe" killer-installer.nsi

if !errorlevel! equ 0 (
    echo.
    echo ========================================================================
    echo   SUCCESS! Installer created: Killer-v1.1-Setup.exe
    echo ========================================================================
    echo.
    echo Next steps:
    echo   1. Find "Killer-v1.1-Setup.exe" in this folder
    echo   2. Share it with your team
    echo   3. Users simply run the .exe to install Killer
    echo.
    echo The installer will:
    echo   - Install Killer to C:\Program Files\Killer
    echo   - Add 'killer.exe' to system PATH
    echo   - Create Start Menu shortcuts
    echo   - Enable uninstall from Control Panel
    echo.
) else (
    echo.
    echo [ERROR] Compilation failed!
    echo Please check the error message above.
    echo.
)

pause
