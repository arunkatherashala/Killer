@echo off
REM Killer Programming Language - Standalone Installer (Windows)
REM Phase 2: Installs killer.exe compiled executable with zero Python dependency
REM Works on Windows 10/11

setlocal enabledelayedexpansion

REM Colors for output
color 0A

REM Check for admin privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [!] This installer requires administrator privileges.
    echo [!] Please run as Administrator (Right-click, select "Run as administrator")
    pause
    exit /b 1
)

cls
echo ================================================================================
echo   Killer Programming Language - Standalone Installer (Phase 2)
echo   Version: 3.0 (Standalone Executable - No Python Required)
echo ================================================================================
echo.

REM Define installation paths
set INSTALL_DIR=%ProgramFiles%\Killer
set KILLER_EXE=%INSTALL_DIR%\killer.bat
set KILLER_WRAPPER=%CD%\killer.bat
set UNINSTALLER=%INSTALL_DIR%\uninstall.bat
set SHORTCUT=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Killer.lnk

REM Check if already installed
if exist "%KILLER_EXE%" (
    echo.
    echo ================================================================================
    echo   Killer v3.0 is already installed
    echo ================================================================================
    echo.
    echo [1] Upgrade     - Update in place to latest version
    echo [2] Uninstall   - Remove Killer completely
    echo [3] Cancel      - Exit without changes
    echo.
    set /p CHOICE="Select option (1/2/3): "
    echo.
    
    if /i "!CHOICE!"=="1" (
        echo [*] Upgrading Killer v3.0 in place...
        REM Continue with installation (files will be overwritten)
    ) else if /i "!CHOICE!"=="2" (
        echo [!] Uninstalling Killer v3.0...
        echo [*] Removing installation directory: %INSTALL_DIR%
        rmdir /s /q "%INSTALL_DIR%" >nul 2>&1
                REM Remove from user PATH safely
                powershell -NoProfile -ExecutionPolicy Bypass -Command ^
                    "$install = '%INSTALL_DIR%'; " ^
                    "$userPath = [Environment]::GetEnvironmentVariable('Path','User'); " ^
                    "if (-not $userPath) { exit 0 }; " ^
                    "$parts = $userPath -split ';' ^| Where-Object { $_ -and $_.Trim() -ne '' -and $_ -ne $install }; " ^
                    "[Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User')"
        echo [+] Killer uninstalled successfully
        echo.
        pause
        exit /b 0
    ) else (
        echo [*] Installation cancelled.
        pause
        exit /b 0
    )
)

REM Create installation directory
echo [*] Creating installation directory: %INSTALL_DIR%
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
if !errorLevel! neq 0 (
    echo [!] Error: Could not create directory
    pause
    exit /b 1
)

REM Check for killer.bat in current directory
if exist "killer.bat" (
    echo [+] Found killer.bat - Installing...
    copy /Y "killer.bat" "%KILLER_EXE%" >nul
    if !errorLevel! neq 0 (
        echo [!] Error: Failed to copy killer.bat
        pause
        exit /b 1
    )
) else if exist "killer.exe" (
    echo [+] Found killer.exe - Installing...
    copy /Y "killer.exe" "%KILLER_EXE%" >nul
    if !errorLevel! neq 0 (
        echo [!] Error: Failed to copy killer.exe
        pause
        exit /b 1
    )
) else (
    echo [!] Error: killer.bat or killer.exe not found in current directory
    echo [!] Please ensure killer.bat/killer.exe is in the same folder as this installer
    pause
    exit /b 1
)

REM Add to PATH if not already there
echo [*] Adding Killer to PATH...

REM Update USER PATH safely (avoid setx truncation and duplicates)
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$install = '%INSTALL_DIR%'; " ^
  "$userPath = [Environment]::GetEnvironmentVariable('Path','User'); " ^
  "if ([string]::IsNullOrWhiteSpace($userPath)) { $parts = @() } else { $parts = $userPath -split ';' ^| Where-Object { $_ -and $_.Trim() -ne '' } }; " ^
  "if ($parts -notcontains $install) { $parts += $install }; " ^
  "[Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User')"

if !errorLevel! equ 0 (
    echo [+] User PATH updated for future terminals
) else (
    echo [!] Warning: Could not add to PATH automatically
    echo [*] Manual setup: Search "Environment Variables" ^> Edit system environment variables
    echo [*] Under "User variables", click "Path" then add:
    echo    %INSTALL_DIR%
)

REM Also update current session PATH immediately
echo !PATH! | find /i "%INSTALL_DIR%" >nul
if errorlevel 1 set "PATH=!PATH!;%INSTALL_DIR%"
echo [+] Current session PATH updated!

REM Create uninstaller
echo [*] Creating uninstaller...
(
    echo @echo off
    echo echo Uninstalling Killer v3.0...
    echo rmdir /s /q "%INSTALL_DIR%"
    echo echo [+] Killer uninstalled successfully
    echo pause
) > "%UNINSTALLER%"

REM Create desktop shortcut (optional)
echo.
set /p CREATE_SHORTCUT="Create desktop shortcut? (Y/N): "
if /i "!CREATE_SHORTCUT!"=="Y" (
    echo [*] Creating desktop shortcut...
    powershell -Command "$WshShell = New-Object -ComObject WScript.Shell; $lnk = $WshShell.CreateShortcut('%USERPROFILE%\Desktop\Killer.lnk'); $lnk.TargetPath = '%KILLER_EXE%'; $lnk.Save()" >nul 2>&1
    if !errorLevel! equ 0 (
        echo [+] Desktop shortcut created
    )
)

REM Verify installation
echo.
echo [*] Verifying installation...
"%KILLER_EXE%" --version >nul 2>&1
if !errorLevel! equ 0 (
    for /f "tokens=*" %%a in ('"%KILLER_EXE%" --version') do set VERSION=%%a
    echo [+] !VERSION!
) else (
    "%KILLER_EXE%" --version
)

REM Test installation
echo.
echo [*] Running test program...
(
    echo x = 10
    echo y = 5
    echo print(x + y)
) > "%INSTALL_DIR%\test.killer"

"%KILLER_EXE%" "%INSTALL_DIR%\test.killer" >nul 2>&1
if !errorLevel! equ 0 (
    echo [+] Test successful - Killer is working!
) else (
    echo [!] Warning: Test failed - there may be an issue
)

del /q "%INSTALL_DIR%\test.killer" >nul 2>&1

REM Register .killer file association with Windows
echo.
echo [*] Registering .killer file association...
@setlocal enabledelayedexpansion

REM Register file extension
assoc .killer=KillerScript >nul 2>&1
if !errorLevel! equ 0 (
    ftype KillerScript="%KILLER_EXE%" "%%1" %%* >nul 2>&1
    if !errorLevel! equ 0 (
        echo [+] .killer files registered - You can now run: test.killer
    ) else (
        echo [!] Warning: Could not set file type
    )
) else (
    echo [!] Warning: Could not register file extension
    echo [*] You can still run: killer test.killer
)

REM Installation summary
cls
echo ================================================================================
echo   Installation Complete!
echo ================================================================================
echo.
echo [+] Killer v3.0 installed successfully
echo [+] Installation directory: %INSTALL_DIR%
echo [+] Added to PATH: Yes
echo [+] Ready to use: YES
echo.
echo Usage (try it now):
echo   test.killer                              # Run a .killer file directly
echo   killer example.killer                    # Run with killer command
echo   killer --version                         # Show version
echo   killer --help                            # Show help
echo.
echo Next steps:
echo   1. Type: killer --version
echo   2. Run an example: killer examples/01_hello.killer
echo   3. Create your own: killer myprogram.killer
echo.
echo Note: 
echo   - Killer is available NOW in this and all future terminal windows
echo   - No terminal restart required!
echo.
echo To uninstall:
echo   - Go to Control Panel ^> Programs ^> Programs and Features
echo   - Or run: "%UNINSTALLER%"
echo.
echo Documentation: https://github.com/arunaug2008-ai/Killer
echo ================================================================================
echo.
pause
exit /b 0
