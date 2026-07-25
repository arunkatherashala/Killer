@echo off
REM Killer Developer Environment Setup (Windows)
REM Supports venv by default, optional conda mode

setlocal enabledelayedexpansion

if "%1"=="--help" goto :help
if "%1"=="-h" goto :help

set MODE=venv
if /i "%1"=="--conda" set MODE=conda

set ROOT=%~dp0..\..
pushd "%ROOT%"

if /i "%MODE%"=="conda" goto :setup_conda

echo [*] Using Python venv mode
python --version >nul 2>&1
if errorlevel 1 (
    echo [!] Python not found in PATH
    echo [!] Install Python 3.10+ and rerun
    popd
    exit /b 1
)

if not exist ".venv" (
    echo [*] Creating .venv
    python -m venv .venv
    if errorlevel 1 (
        echo [!] Failed to create .venv
        popd
        exit /b 1
    )
) else (
    echo [+] .venv already exists
)

echo [*] Upgrading pip
call .venv\Scripts\python.exe -m pip install --upgrade pip >nul 2>&1

if exist "requirements-dev.txt" (
    echo [*] Installing requirements-dev.txt
    call .venv\Scripts\python.exe -m pip install -r requirements-dev.txt
)

echo [*] Verifying Killer CLI
if exist "killer.bat" (
    call killer.bat --help >nul 2>&1
    if errorlevel 1 (
        echo [!] Killer local launcher check failed
    ) else (
        echo [+] Killer local launcher is ready
    )
)

echo.
echo [+] Developer environment ready

echo [>] Activate with:
echo     .\.venv\Scripts\Activate.ps1

echo [>] Run Killer with:
echo     .\killer --help

popd
exit /b 0

:setup_conda
where conda >nul 2>&1
if errorlevel 1 (
    echo [!] Conda not found in PATH
    echo [!] Install Anaconda/Miniconda and rerun with --conda
    popd
    exit /b 1
)

set ENV_NAME=killer-dev

echo [*] Creating or updating conda environment: %ENV_NAME%
call conda env list | findstr /i "^%ENV_NAME% " >nul
if errorlevel 1 (
    call conda create -y -n %ENV_NAME% python=3.12
) else (
    echo [+] Conda environment already exists
)

if exist "requirements-dev.txt" (
    echo [*] Installing requirements-dev.txt into conda env
    call conda run -n %ENV_NAME% pip install -r requirements-dev.txt
)

echo.
echo [+] Conda developer environment ready

echo [>] Activate with:
echo     conda activate %ENV_NAME%

echo [>] Run Killer with:
echo     .\killer --help

popd
exit /b 0

:help
echo Killer Developer Environment Setup (Windows)
echo.
echo Usage:
echo   setup-dev-env.bat            ^(create/use .venv^)
echo   setup-dev-env.bat --conda    ^(create/use conda env^)
echo   setup-dev-env.bat --help
exit /b 0
