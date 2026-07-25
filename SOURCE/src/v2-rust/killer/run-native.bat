@echo off
setlocal

set VSCMD="C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
set CARGO=%USERPROFILE%\.cargo\bin\cargo.exe

if not exist %CARGO% (
  echo [!] cargo not found at %CARGO%
  echo [!] Install Rustup first: winget install Rustlang.Rustup
  exit /b 1
)

if not exist %VSCMD% (
  echo [!] Visual Studio Developer Command script not found
  echo [!] Install Visual Studio Build Tools with C++ Desktop workload
  exit /b 1
)

call %VSCMD% -arch=x64 >nul

if "%1"=="" (
  %CARGO% run -- --help
  exit /b %errorlevel%
)

REM Pass all args to killer-native, including --killer mode
%CARGO% run -- %*
exit /b %errorlevel%
