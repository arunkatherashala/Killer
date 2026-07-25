# Environment Integration Guide

This guide covers ecosystem usage around Killer (outside core language syntax).

## One-Command Developer Setup (Recommended)

Killer now includes cross-platform automation scripts:

- Windows: `scripts/installers/setup-dev-env.bat`
- macOS/Linux: `scripts/installers/setup-dev-env.sh`

These scripts support:

- `venv` (default)
- `conda` (`--conda` mode)
- optional `pyenv` detection (Unix)
- optional `requirements-dev.txt` installation

### Windows

```powershell
scripts\installers\setup-dev-env.bat
```

Conda mode:

```powershell
scripts\installers\setup-dev-env.bat --conda
```

### macOS / Linux

```bash
chmod +x scripts/installers/setup-dev-env.sh
./scripts/installers/setup-dev-env.sh
```

Conda mode:

```bash
./scripts/installers/setup-dev-env.sh --conda
```

## Python Environment Tools

These tools are optional and useful when Killer runtime is Python-backed:

- `venv` for per-project isolation
- `conda` for managed environments
- `pip` for Python package installation
- `pyenv` for Python version management

## Recommended Setup (Windows)

```powershell
scripts\installers\setup-dev-env.bat
.\.venv\Scripts\Activate.ps1
.\killer --version
```

## Recommended Setup (macOS/Linux)

```bash
./scripts/installers/setup-dev-env.sh
source .venv/bin/activate
./killer --version
```

## Conda Setup (Optional)

Windows:

```powershell
scripts\installers\setup-dev-env.bat --conda
conda activate killer-dev
```

macOS/Linux:

```bash
./scripts/installers/setup-dev-env.sh --conda
conda activate killer-dev
```

## pyenv Notes (Optional)

- If `pyenv` is installed, Unix setup script auto-detects it.
- If a `.python-version` file exists, that version is used by `pyenv` workflows.
- You can still run pure `venv` or `conda` flows without `pyenv`.

## Notes

- Killer language features do not depend on Python syntax tools directly.
- Environment tools are for runtime/tooling workflows where Python-backed execution is used.
- Standalone installer path remains the default recommended end-user setup.
