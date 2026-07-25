@echo off
REM =============================================================================
REM Killer Programming Language - Installation Verification Script
REM Tests if Killer is properly installed and working
REM =============================================================================

setlocal enabledelayedexpansion

color 0A
cls

echo.
echo ╔═══════════════════════════════════════════════════════════════════════════╗
echo ║          KILLER PROGRAMMING LANGUAGE - INSTALLATION VERIFICATION          ║
echo ╚═══════════════════════════════════════════════════════════════════════════╝
echo.

set PASS=0
set FAIL=0
set INSTALL_DIR=%ProgramFiles%\Killer

REM Test 1: Check Python
echo.
echo Test 1: Python installation...
python --version >nul 2>&1
if errorlevel 1 (
    color 0C
    echo [FAILED] Python 3 not found
    set /a FAIL=!FAIL!+1
    color 0A
) else (
    for /f "tokens=2" %%i in ('python --version 2^>^&1') do echo [PASSED] %%i
    set /a PASS=!PASS!+1
)

REM Test 2: Check Killer installation directory
echo.
echo Test 2: Installation directory...
if exist "%INSTALL_DIR%" (
    echo [PASSED] Found at %INSTALL_DIR%
    set /a PASS=!PASS!+1
) else (
    color 0C
    echo [FAILED] Not found at %INSTALL_DIR%
    color 0A
    set /a FAIL=!FAIL!+1
)

REM Test 3: Check killer.bat launcher
echo.
echo Test 3: Killer launcher...
if exist "%INSTALL_DIR%\killer.bat" (
    echo [PASSED] killer.bat found
    set /a PASS=!PASS!+1
) else (
    color 0C
    echo [FAILED] killer.bat not found
    color 0A
    set /a FAIL=!FAIL!+1
)

REM Test 4: Check source files
echo.
echo Test 4: Source files...
if exist "%INSTALL_DIR%\src" (
    echo [PASSED] src directory found
    set /a PASS=!PASS!+1
) else (
    color 0C
    echo [FAILED] src directory not found
    color 0A
    set /a FAIL=!FAIL!+1
)

REM Test 5: Check examples
echo.
echo Test 5: Example files...
if exist "%INSTALL_DIR%\examples" (
    echo [PASSED] examples directory found
    set /a PASS=!PASS!+1
) else (
    echo [PASSED] (optional)
    set /a PASS=!PASS!+1
)

REM Test 6: Test version command
echo.
echo Test 6: Version check...
"%INSTALL_DIR%\killer.bat" --version >nul 2>&1
if errorlevel 1 (
    color 0C
    echo [FAILED] killer --version returned error
    color 0A
    set /a FAIL=!FAIL!+1
) else (
    echo [PASSED] Version command works
    set /a PASS=!PASS!+1
)

REM Test 7: Simple execution
echo.
echo Test 7: Simple execution...
(
    echo print("Installation verified!");
) > "%TEMP%\killer_test.killer"

"%INSTALL_DIR%\killer.bat" "%TEMP%\killer_test.killer" >nul 2>&1
if errorlevel 1 (
    color 0C
    echo [FAILED] Simple execution failed
    color 0A
    set /a FAIL=!FAIL!+1
) else (
    echo [PASSED] Simple execution works
    set /a PASS=!PASS!+1
)
del /q "%TEMP%\killer_test.killer" >nul 2>&1

REM Test 8: Python transpilation
echo.
echo Test 8: Transpilation to Python...
(
    echo x = 10;
    echo print(x);
) > "%TEMP%\killer_test.killer"

"%INSTALL_DIR%\killer.bat" --python "%TEMP%\killer_test.killer" >nul 2>&1
if errorlevel 1 (
    color 0C
    echo [FAILED] Python transpilation failed
    color 0A
    set /a FAIL=!FAIL!+1
) else (
    echo [PASSED] Python transpilation works
    set /a PASS=!PASS!+1
)
del /q "%TEMP%\killer_test.killer" >nul 2>&1

REM Test 9: JavaScript transpilation
echo.
echo Test 9: Transpilation to JavaScript...
(
    echo x = 10;
    echo print(x);
) > "%TEMP%\killer_test.killer"

"%INSTALL_DIR%\killer.bat" --js "%TEMP%\killer_test.killer" >nul 2>&1
if errorlevel 1 (
    color 0C
    echo [FAILED] JavaScript transpilation failed
    color 0A
    set /a FAIL=!FAIL!+1
) else (
    echo [PASSED] JavaScript transpilation works
    set /a PASS=!PASS!+1
)
del /q "%TEMP%\killer_test.killer" >nul 2>&1

REM Results
echo.
echo ╔═══════════════════════════════════════════════════════════════════════════╗
echo ║                           TEST RESULTS                                    ║
echo ╚═══════════════════════════════════════════════════════════════════════════╝
echo.
echo Passed: !PASS!/9
echo Failed: !FAIL!/9
echo.

if !FAIL! EQU 0 (
    color 0A
    echo [SUCCESS] All tests passed! Installation is working correctly.
    echo.
    echo Next steps:
    echo   1. Create a file: echo print("Hello, Killer!"); > hello.killer
    echo   2. Run it: killer hello.killer
    echo   3. Read docs: %INSTALL_DIR%\DOCUMENTATION.md
    echo.
) else (
    color 0C
    echo [FAILED] Some tests failed. Check the output above for details.
    echo.
    echo Troubleshooting:
    echo   * Make sure Python 3 is installed and in PATH
    echo   * Run installer as Administrator: Right-click killer-installer.bat
    echo   * Open a NEW Command Prompt after installation
    echo   * Try reinstalling: killer-installer.bat
    echo.
    color 0A
)

pause
endlocal
