@echo off
REM =============================================================================
REM Killer Programming Language - Windows Installer
REM Version: 2.0
REM =============================================================================

setlocal enabledelayedexpansion

color 0A
cls

echo.
echo ╔═══════════════════════════════════════════════════════════════════════════╗
echo ║                    KILLER PROGRAMMING LANGUAGE                            ║
echo ║                       Windows Installer v2.0                             ║
echo ╚═══════════════════════════════════════════════════════════════════════════╝
echo.

REM Check for Python installation
python --version >nul 2>&1
if errorlevel 1 (
    color 0C
    echo [ERROR] Python 3.6+ is required but not installed!
    echo.
    echo Please install Python from: https://www.python.org
    echo Make sure to check "Add Python to PATH" during installation.
    echo.
    pause
    exit /b 1
)

for /f "tokens=2" %%i in ('python --version 2^>^&1') do set PYTHON_VERSION=%%i
echo [✓] Python %PYTHON_VERSION% detected
echo.

REM Determine installation directory
set INSTALL_DIR=%ProgramFiles%\Killer
set SYSTEM_PATH_ENTRY=%INSTALL_DIR%

echo Installation directory: %INSTALL_DIR%
echo.

REM Check if already installed
if exist "%INSTALL_DIR%" (
    echo [!] Killer is already installed at %INSTALL_DIR%
    echo.
    set /p UPGRADE="Do you want to upgrade in place? (y/n): "
    if /i not "!UPGRADE!"=="y" (
        echo Upgrade cancelled.
        pause
        exit /b 0
    )
    echo [✓] Continuing with in-place upgrade...
    echo.
)

REM Create installation directory
echo Creating installation directory...
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
    if errorlevel 1 (
        color 0C
        echo [ERROR] Failed to create installation directory!
        echo You may need to run as Administrator.
        echo.
        pause
        exit /b 1
    )
)
echo [✓] Installation directory created
echo.

REM Copy source files
echo Copying source files...
if not exist "src\" (
    color 0C
    echo [ERROR] Source files not found! Run installer from killer root directory.
    echo.
    pause
    exit /b 1
)

xcopy /E /I /Y "src" "%INSTALL_DIR%\src" >nul 2>&1
copy /Y "main.py" "%INSTALL_DIR%\" >nul 2>&1
copy /Y "DOCUMENTATION.md" "%INSTALL_DIR%\" >nul 2>&1
copy /Y "examples\*.*" "%INSTALL_DIR%\examples\" >nul 2>&1

echo [✓] Source files copied
echo.

REM Copy examples
if exist "examples\" (
    echo Copying example files...
    if not exist "%INSTALL_DIR%\examples" mkdir "%INSTALL_DIR%\examples"
    xcopy /E /I /Y "examples\*.*" "%INSTALL_DIR%\examples\" >nul 2>&1
    echo [✓] Example files copied
    echo.
)

REM Create wrapper batch file
echo Creating command-line launcher...
(
    echo @echo off
    echo REM Killer Programming Language Launcher
    echo cd /d "%INSTALL_DIR%"
    echo python main.py %%*
) > "%INSTALL_DIR%\killer.bat"

echo [✓] Launcher created
echo.

REM Add to PATH
echo.
echo Setting up PATH environment variable...

REM Update USER PATH safely (avoid setx truncation and duplicates)
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$install = '%INSTALL_DIR%'; " ^
  "$userPath = [Environment]::GetEnvironmentVariable('Path','User'); " ^
  "if ([string]::IsNullOrWhiteSpace($userPath)) { $parts = @() } else { $parts = $userPath -split ';' ^| Where-Object { $_ -and $_.Trim() -ne '' } }; " ^
  "if ($parts -notcontains $install) { $parts += $install }; " ^
  "[Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User')"

if errorlevel 1 (
    echo [!] Warning: Could not update user PATH automatically.
    echo Manually add this to your PATH: %INSTALL_DIR%
) else (
    echo [✓] User PATH updated for future terminals
)

REM Update current terminal PATH immediately
echo %PATH% | find /i "%INSTALL_DIR%" >nul
if errorlevel 1 set "PATH=%PATH%;%INSTALL_DIR%"
echo [✓] Current session PATH updated
echo.

REM Create shortcut to documentation
if exist "%INSTALL_DIR%\DOCUMENTATION.md" (
    echo Creating documentation link...
    copy "%INSTALL_DIR%\DOCUMENTATION.md" "%USERPROFILE%\Desktop\Killer-Documentation.md" >nul 2>&1
    echo [✓] Documentation link created on Desktop
    echo.
)

REM Verify installation
echo.
echo Verifying installation...
"%INSTALL_DIR%\killer.bat" --version >nul 2>&1
if errorlevel 1 (
    color 0E
    echo [!] Warning: Installation completed but verification failed.
    echo Try running: killer --version from command prompt
) else (
    color 0A
    echo [✓] Installation verified successfully!
)
echo.

REM Installation complete
echo ╔═══════════════════════════════════════════════════════════════════════════╗
echo ║                  INSTALLATION COMPLETE                                    ║
echo ╚═══════════════════════════════════════════════════════════════════════════╝
echo.
echo Installation Details:
echo   Location: %INSTALL_DIR%
echo   Launcher: killer.bat
echo   Examples: %INSTALL_DIR%\examples
echo   Docs: %INSTALL_DIR%\DOCUMENTATION.md
echo.
echo Quick Start:
echo   1. Open a NEW command prompt (cmd.exe)
echo   2. Create a file: hello.killer
echo   3. Add code: print("Hello, Killer!");
echo   4. Run: killer hello.killer
echo.
echo Documentation:
echo   https://localhost:8888/docs.html
echo   File: %INSTALL_DIR%\DOCUMENTATION.md
echo.
echo To uninstall:
echo   Run: killer-uninstall.bat
echo.

pause
endlocal
