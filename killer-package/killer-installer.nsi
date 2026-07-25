; ============================================================================
; Killer Language v1.1 - Professional Installer
; NSIS Installation Script
; ============================================================================
; Usage: Right-click this file → "Compile NSIS Script" 
; Output: Killer-v1.1-Setup.exe (in same directory)
; ============================================================================

!include "MUI2.nsh"
!include "x64.nsh"

; ============================================================================
; Installer Configuration
; ============================================================================

Name "Killer v1.1"
OutFile "Killer-v1.1-Setup.exe"
InstallDir "$PROGRAMFILES\Killer"
InstallDirRegKey HKLM "Software\Killer" "InstallDir"

RequestExecutionLevel admin

; MUI Settings
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "README.md"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

; ============================================================================
; Installer Sections
; ============================================================================

Section "Install"
  SetOutPath "$INSTDIR"
  
  ; Copy main executable
  File "killer.exe"
  
  ; Copy documentation
  SetOutPath "$INSTDIR\docs"
  File "docs\*.md"
  
  ; Copy examples
  SetOutPath "$INSTDIR\examples"
  File "examples\*.killer"
  
  ; Copy root documentation
  SetOutPath "$INSTDIR"
  File "README.md"
  File "QUICK_START.md"
  
  ; Store installation path in registry
  WriteRegStr HKLM "Software\Killer" "InstallDir" "$INSTDIR"
  
  ; Add to PATH environment variable
  ${If} $0 != ""
    ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH"
    ${If} $0 != ""
      ${If} $0 != ""
        WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH" "$INSTDIR;$0"
      ${EndIf}
    ${EndIf}
  ${EndIf}
  
  ; Create shortcuts in Start Menu
  CreateDirectory "$SMPROGRAMS\Killer"
  CreateShortCut "$SMPROGRAMS\Killer\Killer Documentation.lnk" "notepad.exe" "$INSTDIR\README.md"
  CreateShortCut "$SMPROGRAMS\Killer\Quick Start.lnk" "notepad.exe" "$INSTDIR\QUICK_START.md"
  CreateShortCut "$SMPROGRAMS\Killer\Examples Folder.lnk" "$INSTDIR\examples"
  CreateShortCut "$SMPROGRAMS\Killer\Uninstall Killer.lnk" "$INSTDIR\Uninstall.exe"
  
  ; Create uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  
  ; Add uninstall info to Control Panel
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Killer" "DisplayName" "Killer v1.1"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Killer" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Killer" "DisplayVersion" "1.1"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Killer" "Publisher" "Killer Team"

SectionEnd

; ============================================================================
; Uninstaller Section
; ============================================================================

Section "Uninstall"
  ; Remove from PATH
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH"
  ${StrStr} $1 $0 "$INSTDIR;"
  ${If} $1 != ""
    StrLen $2 "$INSTDIR;"
    StrLen $3 $0
    IntOp $3 $3 - $2
    StrCpy $0 $0 $2 0
    StrCpy $1 $0 "" $2
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH" "$1"
  ${EndIf}
  
  ; Remove files
  Delete "$INSTDIR\killer.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\QUICK_START.md"
  Delete "$INSTDIR\Uninstall.exe"
  
  ; Remove directories
  RMDir /r "$INSTDIR\docs"
  RMDir /r "$INSTDIR\examples"
  RMDir "$INSTDIR"
  
  ; Remove Start Menu shortcuts
  RMDir /r "$SMPROGRAMS\Killer"
  
  ; Remove registry entries
  DeleteRegKey HKLM "Software\Killer"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Killer"

SectionEnd

; ============================================================================
; Function to notify system of PATH change
; ============================================================================

Function .onInstSuccess
  MessageBox MB_OK "Killer v1.1 installed successfully!$\n$\nYou can now use 'killer.exe' from any command prompt.$\n$\nNext: Read QUICK_START.md to get started!"

FunctionEnd
