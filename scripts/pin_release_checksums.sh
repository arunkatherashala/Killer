#!/bin/sh
# Record a release's archive checksums in the repository so installers can verify downloads against a source that
# is independent of the release assets.
#
# Usage: scripts/pin_release_checksums.sh vX.Y.Z path/to/SHA256SUMS [output-dir]
#
# Writes <output-dir>/vX.Y.Z.sha256 (default: checksums/ at the repository root; lines of 'sha256  archive-name',
# only the archives named killer_super-<tag>-*) and prints a block to paste into the release notes body. Commit the
# file on main: install.sh and install.ps1 fetch it from raw.githubusercontent.com and compare it with the archive.
# Do this only after checking the SHA256SUMS against the CI run's own artifacts (the workflow log prints them).
set -eu
[ $# -ge 2 ] || { echo "usage: $0 <tag> <SHA256SUMS file> [output dir]" >&2; exit 2; }
tag="$1"; sums="$2"
case "$tag" in ""|*[!A-Za-z0-9._-]*|*..*|-*) echo "invalid tag '$tag'" >&2; exit 2 ;; esac
[ -f "$sums" ] || { echo "no such file: $sums" >&2; exit 2; }
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="${3:-$root/checksums}"
out="$dir/$tag.sha256"
mkdir -p "$dir"
# 'hash  name' or 'hash *name' lines for this tag's archives, hash lower-cased, name without the binary marker
grep -E "^[0-9a-fA-F]{64} [ *]killer_super-$tag-" "$sums" | awk '{ h = tolower($1); n = $2; sub(/^\*/, "", n); print h "  " n }' > "$out.tmp" || true
if [ ! -s "$out.tmp" ]; then
  rm -f "$out.tmp"
  echo "no killer_super-$tag-* entries found in $sums" >&2
  exit 1
fi
mv "$out.tmp" "$out"
echo "wrote $out" >&2
echo "Paste into the release notes:"
echo
echo "SHA-256 of the release archives (also in the repository at checksums/$tag.sha256):"
echo
echo '```'
cat "$out"
echo '```'
