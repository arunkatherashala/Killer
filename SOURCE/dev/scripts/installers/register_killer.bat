@echo off
REM Register .killer file association with Windows
REM This allows double-clicking .killer files to execute them

if "%1"=="--unregister" (
    echo Unregistering .killer file association...
    reg delete "HKCR\.killer" /f >nul 2>&1
    reg delete "HKCR\KillerScript" /f >nul 2>&1
    echo [+] .killer file association removed
    exit /b 0
)

echo Registering .killer file association...
echo.

REM Get the installation directory
set KILLER_DIR=%~dp0killer.bat
set KILLER_DIR=%KILLER_DIR:\killer.bat=%

REM Register .killer extension
reg add "HKCR\.killer" /ve /d "KillerScript" /f >nul 2>&1
if errorlevel 1 (
    echo [!] Error: Could not register file extension
    echo [*] Try running as Administrator
    exit /b 1
)

REM Register KillerScript file type
reg add "HKCR\KillerScript" /ve /d "Killer Script" /f >nul 2>&1
reg add "HKCR\KillerScript\shell" /ve /d "open" /f >nul 2>&1

REM Register open command
reg add "HKCR\KillerScript\shell\open\command" /ve /d "\"%KILLER_DIR%killer.bat\" \"%%%%1\"" /f >nul 2>&1
if errorlevel 1 (
    echo [!] Error: Could not register file association command
    exit /b 1
)

echo [+] .killer file association registered successfully!
echo [+] You can now double-click .killer files to run them
echo [+] Or run: test.killer (without 'killer' prefix)
echo.
echo To unregister: register_killer.bat --unregister
