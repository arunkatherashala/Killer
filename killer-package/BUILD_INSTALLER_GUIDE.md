# 🚀 Building a Professional NSIS Installer

## What's This?

This folder contains everything needed to build a professional Windows installer for Killer v1.1.

---

## Files Included

```
├── killer-installer.nsi      ← NSIS installation script
├── build-installer.bat       ← Compiler script (Windows)
├── BUILD_INSTALLER_GUIDE.md  ← This file
└── [killer executable + docs/examples]
```

---

## Step-by-Step: Build Your Installer

### Step 1: Install NSIS Compiler

1. Go to: https://nsis.sourceforge.io/
2. Download latest version
3. Run installer, use default settings
4. Verify: `C:\Program Files\NSIS\makensis.exe` exists

### Step 2: Build the Installer

Simply run:
```bash
build-installer.bat
```

Wait ~30 seconds...

### Step 3: Distribute

Share the output file: **`Killer-v1.1-Setup.exe`**

---

## What the Installer Does

✅ **Installation**
- Installs to `C:\Program Files\Killer`
- Auto-adds `killer.exe` to Windows PATH
- Creates Start Menu shortcuts
- Adds uninstall option to Control Panel

✅ **User Experience**
- Professional setup wizard
- Read documentation option
- License agreement
- Success message with next steps

✅ **Uninstall**
- Clean removal from PATH
- All files deleted
- Registry entries cleaned
- Shortcuts removed

---

## How Users Install

### User's Experience:

1. **Download**: `Killer-v1.1-Setup.exe` (from you)
2. **Run**: Double-click the .exe
3. **Click**: "Install" → "Finish"
4. **Done**: Open any command prompt, type `killer.exe`

---

## Customization (Advanced)

Edit `killer-installer.nsi` to customize:

```nsi
; Company name
!define COMPANY "Your Company"

; Installer output name
OutFile "MyCompany-Killer-v1.1-Setup.exe"

; Install path
InstallDir "$PROGRAMFILES\MyCompany\Killer"

; Custom start menu name
CreateDirectory "$SMPROGRAMS\MyCompany\Killer"
```

---

## Troubleshooting

### Issue: "makensis.exe not found"
**Fix**: Install NSIS from https://nsis.sourceforge.io/

### Issue: Permission denied
**Fix**: Run Command Prompt as Administrator, then run `build-installer.bat`

### Issue: Want custom branding?
**Fix**: Edit `killer-installer.nsi` with your company name/logo

---

## Final Launcher Command

Once users install, they can use Killer anywhere:

```bash
C:\> killer.exe hello.killer

# Or if PATH is set:
killer.exe hello.killer
```

---

## Distribution

Upload `Killer-v1.1-Setup.exe` to:
- 📧 Email to teams
- ☁️ Google Drive / OneDrive
- 🔗 Website download link
- 📦 GitHub releases

---

**Ready to build? Run: `build-installer.bat`** 🚀
