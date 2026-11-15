#!/bin/sh
# Killer installer: curl -fsSL <url>/install.sh | sh
# Detects OS/arch, downloads the release archive, verifies SHA256, installs to ~/.killer/bin.
set -eu

REPO="${KILLER_REPO:-arunkatherashala/Killer}"
VERSION="${KILLER_VERSION:-latest}"
PREFIX="${KILLER_HOME:-$HOME/.killer}"
DRY_RUN=0

usage() {
  cat <<USAGE
Usage: install.sh [--version vX.Y.Z] [--prefix DIR] [--dry-run] [--help]

Installs killer and killer_super into DIR/bin (default: ~/.killer/bin).

Options:
  --version TAG   release tag to install (default: latest)
  --prefix DIR    install root (default: \$HOME/.killer)
  --dry-run       print what would be done, download nothing
  --help          show this help

Environment: KILLER_VERSION, KILLER_HOME, KILLER_REPO override the defaults;
KILLER_BASE_URL replaces the whole download directory URL (mirrors, local testing).
USAGE
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || { echo "--version needs a value" >&2; exit 2; }; VERSION="$2"; shift 2 ;;
    --prefix)  [ $# -ge 2 ] || { echo "--prefix needs a value" >&2; exit 2; }; PREFIX="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

detect_target() {
  os="${KILLER_OS:-$(uname -s)}"
  arch="${KILLER_ARCH:-$(uname -m)}"
  case "$os" in
    Linux)  o=linux ;;
    Darwin) o=macos ;;
    *) echo "unsupported OS: $os (use install.ps1 on Windows)" >&2; return 1 ;;
  esac
  case "$arch" in
    x86_64|amd64)  a=x64 ;;
    arm64|aarch64) a=arm64 ;;
    *) echo "unsupported architecture: $arch" >&2; return 1 ;;
  esac
  # Only these targets are published by the release workflow.
  case "$o-$a" in
    linux-x64|macos-x64|macos-arm64) printf '%s-%s\n' "$o" "$a" ;;
    *) echo "no prebuilt binary for $o-$a; build from source with cargo" >&2; return 1 ;;
  esac
}

download() { # url dest
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$1" -O "$2"
  else
    echo "need curl or wget" >&2
    return 1
  fi
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    echo "need sha256sum or shasum" >&2
    return 1
  fi
}

# Tags end up in URLs and file names: only [A-Za-z0-9._-] is accepted (no slashes, "..", spaces, quotes, ...).
valid_tag() {
  case "$1" in
    ""|*[!A-Za-z0-9._-]*|.|..|-*) return 1 ;;
  esac
  case "$1" in *..*) return 1 ;; esac
  return 0
}

if [ "$VERSION" != "latest" ] && ! valid_tag "$VERSION"; then
  echo "invalid version '$VERSION': only letters, digits, '.', '_' and '-' are allowed" >&2
  exit 2
fi

TARGET="$(detect_target)" || exit 1

if [ "$VERSION" = "latest" ]; then
  if [ "$DRY_RUN" -eq 1 ]; then
    TAG="<latest tag>"
  else
    tmp_json="$(mktemp)"
    if ! download "https://api.github.com/repos/$REPO/releases/latest" "$tmp_json"; then
      # GitHub's "latest" skips pre-releases; fall back to the newest release of any kind.
      echo "no full release yet; using the newest pre-release (pass --version vX.Y.Z to choose)" >&2
      if ! download "https://api.github.com/repos/$REPO/releases?per_page=1" "$tmp_json"; then
        echo "could not query releases (pass --version vX.Y.Z)" >&2
        rm -f "$tmp_json"
        exit 1
      fi
    fi
    TAG="$(sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' "$tmp_json" | head -n 1)"
    rm -f "$tmp_json"
    [ -n "$TAG" ] || { echo "could not determine latest tag" >&2; exit 1; }
  fi
else
  TAG="$VERSION"
fi

if [ "$DRY_RUN" -eq 0 ] && ! valid_tag "$TAG"; then
  echo "refusing release tag '$TAG': only letters, digits, '.', '_' and '-' are allowed" >&2
  exit 1
fi

NAME="killer_super-$TAG-$TARGET"
BASE="${KILLER_BASE_URL:-https://github.com/$REPO/releases/download/$TAG}"
BIN_DIR="$PREFIX/bin"

if [ -n "${KILLER_BASE_URL:-}" ]; then
  echo "NOTE: KILLER_BASE_URL is set; downloading from $KILLER_BASE_URL instead of github.com/$REPO." >&2
  echo "      The .sha256 file is fetched from the same place, so it detects corruption but does not prove the origin is trustworthy." >&2
fi
echo "target:   $TARGET"
echo "tag:      $TAG"
echo "archive:  $BASE/$NAME.tar.gz"
echo "checksum: $BASE/$NAME.tar.gz.sha256"
echo "install:  $BIN_DIR/killer and $BIN_DIR/killer_super"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "dry run: nothing downloaded or installed"
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM

download "$BASE/$NAME.tar.gz" "$work/$NAME.tar.gz"
download "$BASE/$NAME.tar.gz.sha256" "$work/$NAME.tar.gz.sha256"

want="$(cut -d' ' -f1 "$work/$NAME.tar.gz.sha256")"
got="$(sha256_of "$work/$NAME.tar.gz")"
if [ "$want" != "$got" ]; then
  echo "checksum mismatch: expected $want, got $got" >&2
  exit 1
fi
echo "checksum ok"

tar -xzf "$work/$NAME.tar.gz" -C "$work"
mkdir -p "$BIN_DIR"
for b in killer killer_super; do
  cp "$work/$NAME/$b" "$BIN_DIR/$b"
  chmod +x "$BIN_DIR/$b"
  echo "installed $BIN_DIR/$b"
done

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "add it to your PATH:  export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac
