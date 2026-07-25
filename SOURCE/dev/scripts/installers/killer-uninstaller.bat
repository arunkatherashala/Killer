@echo off
REM =============================================================================
REM Killer Programming Language - Windows Uninstaller
REM Version: 2.0
REM =============================================================================

setlocal enabledelayedexpansion

color 0A
cls

echo.
echo ╔═══════════════════════════════════════════════════════════════════════════╗
echo ║                    KILLER PROGRAMMING LANGUAGE                            ║
echo ║                      Windows Uninstaller v2.0                            ║
echo ╚═══════════════════════════════════════════════════════════════════════════╝
echo.

set INSTALL_DIR=%ProgramFiles%\Killer

if not exist "%INSTALL_DIR%" (
    echo Killer is not installed.
    echo.
    pause
    exit /b 0
)

echo Installation found at: %INSTALL_DIR%
echo.
set /p CONFIRM="Are you sure you want to uninstall Killer? (y/n): "

if /i not "%CONFIRM%"=="y" (
    echo Uninstallation cancelled.
    pause
    exit /b 0
)

echo.
echo Removing Killer installation...
rmdir /s /q "%INSTALL_DIR%"

if errorlevel 1 (
    color 0C
    echo [ERROR] Failed to remove installation directory.
    echo You may need to run as Administrator.
    echo.
    pause
    exit /b 1
)

echo Removing Killer from user PATH...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$install = '%INSTALL_DIR%'; " ^
  "$userPath = [Environment]::GetEnvironmentVariable('Path','User'); " ^
  "if (-not $userPath) { exit 0 }; " ^
  "$parts = $userPath -split ';' ^| Where-Object { $_ -and $_.Trim() -ne '' -and $_ -ne $install }; " ^
  "[Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User')"

if errorlevel 1 (
    echo [!] Warning: Could not update user PATH automatically.
) else (
    echo [✓] User PATH entry removed
)

color 0A
echo [✓] Killer has been uninstalled successfully!
echo.
echo Note: You may need to restart your command prompt for PATH changes to take effect.
echo.
pause
endlocal
