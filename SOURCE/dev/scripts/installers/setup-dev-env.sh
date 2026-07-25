#!/bin/bash
# Killer Developer Environment Setup (macOS/Linux)
# Supports venv by default, optional conda mode

set -e

MODE="venv"
if [[ "$1" == "--conda" ]]; then
  MODE="conda"
fi

if [[ "$1" == "--help" || "$1" == "-h" ]]; then
  echo "Killer Developer Environment Setup (macOS/Linux)"
  echo
  echo "Usage:"
  echo "  ./setup-dev-env.sh           (create/use .venv)"
  echo "  ./setup-dev-env.sh --conda   (create/use conda env)"
  echo "  ./setup-dev-env.sh --help"
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if [[ "$MODE" == "conda" ]]; then
  if ! command -v conda >/dev/null 2>&1; then
    echo "[!] conda not found in PATH"
    echo "[!] Install Miniconda/Anaconda and rerun with --conda"
    exit 1
  fi

  ENV_NAME="killer-dev"
  if ! conda env list | awk '{print $1}' | grep -qx "$ENV_NAME"; then
    echo "[*] Creating conda environment: $ENV_NAME"
    conda create -y -n "$ENV_NAME" python=3.12
  else
    echo "[+] Conda environment already exists: $ENV_NAME"
  fi

  if [[ -f requirements-dev.txt ]]; then
    echo "[*] Installing requirements-dev.txt"
    conda run -n "$ENV_NAME" pip install -r requirements-dev.txt
  fi

  echo
  echo "[+] Conda developer environment ready"
  echo "[>] Activate with: conda activate $ENV_NAME"
  echo "[>] Run Killer with: ./killer --help"
  exit 0
fi

if command -v pyenv >/dev/null 2>&1; then
  echo "[*] pyenv detected: $(pyenv --version)"
  if [[ -f .python-version ]]; then
    echo "[+] Using project python version from .python-version"
  fi
fi

PYTHON_BIN=""
if command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
elif command -v python >/dev/null 2>&1; then
  PYTHON_BIN="python"
else
  echo "[!] Python not found in PATH"
  exit 1
fi

if [[ ! -d .venv ]]; then
  echo "[*] Creating .venv"
  "$PYTHON_BIN" -m venv .venv
else
  echo "[+] .venv already exists"
fi

echo "[*] Upgrading pip"
. .venv/bin/activate
python -m pip install --upgrade pip >/dev/null 2>&1 || true

if [[ -f requirements-dev.txt ]]; then
  echo "[*] Installing requirements-dev.txt"
  pip install -r requirements-dev.txt
fi

echo "[*] Verifying Killer launcher"
if [[ -f ./killer || -f ./killer.sh ]]; then
  ./killer --help >/dev/null 2>&1 || true
fi

echo
echo "[+] Developer environment ready"
echo "[>] Activate with: source .venv/bin/activate"
echo "[>] Run Killer with: ./killer --help"
