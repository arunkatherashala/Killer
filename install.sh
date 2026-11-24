#!/bin/sh
# Killer installer: curl -fsSL <url>/install.sh | sh
# Detects OS/arch, downloads the release archive, verifies SHA256, installs to ~/.killer/bin.
#
# Integrity: the archive's .sha256 comes from the same place as the archive, so on its own it only catches
# corruption. For an independent check the installer also verifies the archive against a checksum that does NOT
# come from the release assets: --sha256 HEX (copy it from the release notes or from checksums/<tag>.sha256 in the
# repository), or the repository's checksums/<tag>.sha256 file, fetched from raw.githubusercontent.com.
# See SOURCE/docs/project/SECURITY.md ("Download and package integrity") for what this does and does not protect.
set -eu

REPO="${KILLER_REPO:-arunkatherashala/Killer}"
VERSION="${KILLER_VERSION:-latest}"
PREFIX="${KILLER_HOME:-$HOME/.killer}"
DRY_RUN=0
EXPECT_SHA="${KILLER_SHA256:-}"
REQUIRE_PINNED="${KILLER_REQUIRE_PINNED:-0}"
CHECKSUMS_URL="${KILLER_CHECKSUMS_URL:-}"

usage() {
  cat <<USAGE
Usage: install.sh [--version vX.Y.Z] [--prefix DIR] [--sha256 HEX] [--require-pinned] [--dry-run] [--help]

Installs killer and killer_super into DIR/bin (default: ~/.killer/bin).

Options:
  --version TAG   release tag to install (default: latest)
  --prefix DIR    install root (default: \$HOME/.killer)
  --sha256 HEX    expected SHA-256 of the archive, taken from an independent source (release notes or
                  checksums/<tag>.sha256 in the repository); the install fails if the archive differs
  --require-pinned  fail unless an independent checksum (--sha256 or the repository file) was checked
  --dry-run       print what would be done, download nothing
  --help          show this help

Environment: KILLER_VERSION, KILLER_HOME, KILLER_REPO override the defaults;
KILLER_BASE_URL replaces the whole download directory URL (mirrors, local testing);
KILLER_SHA256, KILLER_REQUIRE_PINNED=1 and KILLER_CHECKSUMS_URL (a file of 'sha256  archive-name' lines, default:
the repository's checksums/<tag>.sha256 on the main branch) control the independent checksum.
USAGE
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || { echo "--version needs a value" >&2; exit 2; }; VERSION="$2"; shift 2 ;;
    --prefix)  [ $# -ge 2 ] || { echo "--prefix needs a value" >&2; exit 2; }; PREFIX="$2"; shift 2 ;;
    --sha256)  [ $# -ge 2 ] || { echo "--sha256 needs a value" >&2; exit 2; }; EXPECT_SHA="$2"; shift 2 ;;
    --require-pinned) REQUIRE_PINNED=1; shift ;;
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

# 64 hex digits, any case; printed lower case
EXPECT_SHA="$(printf '%s' "$EXPECT_SHA" | tr 'A-F' 'a-f')"
if [ -n "$EXPECT_SHA" ]; then
  case "$EXPECT_SHA" in
    *[!0-9a-f]*) echo "invalid --sha256: expected 64 hex digits" >&2; exit 2 ;;
  esac
  [ "${#EXPECT_SHA}" -eq 64 ] || { echo "invalid --sha256: expected 64 hex digits" >&2; exit 2; }
fi

# the hash for archive $2 in a 'sha256  name' (or 'sha256 *name') file $1
pinned_hash_for() {
  awk -v n="$2" '{ f = $2; sub(/^\*/, "", f); if (f == n) { print tolower($1); exit } }' "$1"
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
# Independent checksum source. A mirror (KILLER_BASE_URL) has no default: pass --sha256 or KILLER_CHECKSUMS_URL.
EXPLICIT_CHECKSUMS=0
[ -n "$CHECKSUMS_URL" ] && EXPLICIT_CHECKSUMS=1
if [ -z "$EXPECT_SHA" ] && [ -z "$CHECKSUMS_URL" ] && [ -z "${KILLER_BASE_URL:-}" ]; then
  CHECKSUMS_URL="https://raw.githubusercontent.com/$REPO/main/checksums/$TAG.sha256"
fi
if [ -n "$EXPECT_SHA" ]; then
  echo "pinned:   sha256 $EXPECT_SHA (from --sha256)"
elif [ -n "$CHECKSUMS_URL" ]; then
  echo "pinned:   $CHECKSUMS_URL"
else
  echo "pinned:   none"
fi
echo "install:  $BIN_DIR/killer and $BIN_DIR/killer_super"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "dry run: nothing downloaded or installed"
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM

download "$BASE/$NAME.tar.gz" "$work/$NAME.tar.gz"
download "$BASE/$NAME.tar.gz.sha256" "$work/$NAME.tar.gz.sha256"

want="$(cut -d' ' -f1 "$work/$NAME.tar.gz.sha256" | tr 'A-F' 'a-f')"
got="$(sha256_of "$work/$NAME.tar.gz")"
if [ "$want" != "$got" ]; then
  echo "checksum mismatch: expected $want, got $got" >&2
  exit 1
fi
echo "checksum ok (release .sha256: detects corruption, same origin as the archive)"

# Independent check: a checksum that was not served with the archive.
pinned="$EXPECT_SHA"
if [ -z "$pinned" ] && [ -n "$CHECKSUMS_URL" ]; then
  if download "$CHECKSUMS_URL" "$work/pinned.sums" 2>/dev/null; then
    pinned="$(pinned_hash_for "$work/pinned.sums" "$NAME.tar.gz")"
    if [ -z "$pinned" ]; then
      echo "the pinned checksum file $CHECKSUMS_URL has no entry for $NAME.tar.gz" >&2
      exit 1
    fi
  elif [ "$EXPLICIT_CHECKSUMS" -eq 1 ] || [ "$REQUIRE_PINNED" -eq 1 ]; then
    echo "could not fetch the pinned checksum file $CHECKSUMS_URL" >&2
    exit 1
  else
    echo "WARNING: no pinned checksum for $TAG at $CHECKSUMS_URL; the archive was only compared with the .sha256 from the same release." >&2
    echo "         Pass --sha256 <hash from the release notes> for an independent check, or --require-pinned to refuse this." >&2
  fi
fi
if [ -n "$pinned" ]; then
  if [ "$pinned" != "$got" ]; then
    echo "checksum mismatch: the pinned checksum is $pinned but the archive hashes to $got" >&2
    exit 1
  fi
  echo "pinned checksum ok"
elif [ "$REQUIRE_PINNED" -eq 1 ]; then
  echo "--require-pinned: no independent checksum was available, refusing to install" >&2
  exit 1
fi

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
