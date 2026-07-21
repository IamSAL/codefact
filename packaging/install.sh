#!/bin/sh
# codefact installer. Downloads a prebuilt, checksum-verified binary.
# Usage: curl -fsSL .../install.sh | sh
set -eu

REPO="${codefact_REPO:-OWNER/codefact}"
VERSION="${codefact_VERSION:-latest}"
BINDIR="${codefact_BINDIR:-/usr/local/bin}"

os="$(uname -s)"; arch="$(uname -m)"
case "$os" in
  Darwin) os=apple-darwin ;;
  Linux)  os=unknown-linux-gnu ;;
  *) echo "unsupported OS: $os" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) echo "unsupported arch: $arch" >&2; exit 1 ;;
esac
target="${arch}-${os}"

if [ "$VERSION" = "latest" ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  base="https://github.com/$REPO/releases/download/$VERSION"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
for bin in codefact; do
  url="$base/${bin}-${target}"
  echo "↓ $url"
  curl -fsSL "$url" -o "$tmp/$bin"
  curl -fsSL "$url.sha256" -o "$tmp/$bin.sha256"
  ( cd "$tmp" && sha256sum -c "$bin.sha256" 2>/dev/null || shasum -a 256 -c "$bin.sha256" )
  chmod +x "$tmp/$bin"
  install "$tmp/$bin" "$BINDIR/$bin" 2>/dev/null || sudo install "$tmp/$bin" "$BINDIR/$bin"
done
echo "✓ installed to $BINDIR. Prereqs: install 'iii' (iii.dev) and 'claude'. Then: codefact init"
