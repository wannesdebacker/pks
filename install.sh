#!/bin/sh
# Installs the latest pks release.
#   curl -fsSL https://raw.githubusercontent.com/wannesdebacker/pks/main/install.sh | sh
#
# Env:
#   PKS_VERSION      version to install, e.g. v0.2.0 (default: latest)
#   PKS_INSTALL_DIR  where to put the binary (default: ~/.local/bin)

set -eu

REPO="wannesdebacker/pks"
INSTALL_DIR="${PKS_INSTALL_DIR:-$HOME/.local/bin}"

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || err "need '$1' but it is not installed"
}

need curl
need tar
need uname

case "$(uname -s)" in
  Darwin) os="apple-darwin" ;;
  Linux) os="unknown-linux-gnu" ;;
  *) err "unsupported OS: $(uname -s). On Windows, download the .zip from https://github.com/$REPO/releases" ;;
esac

case "$(uname -m)" in
  arm64 | aarch64) arch="aarch64" ;;
  x86_64 | amd64) arch="x86_64" ;;
  *) err "unsupported architecture: $(uname -m)" ;;
esac

target="$arch-$os"
asset="pks-$target.tar.gz"

if [ -n "${PKS_VERSION:-}" ]; then
  url="https://github.com/$REPO/releases/download/$PKS_VERSION/$asset"
else
  url="https://github.com/$REPO/releases/latest/download/$asset"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf 'downloading %s\n' "$url"
curl -fsSL "$url" -o "$tmp/$asset" || err "download failed (is there a release for $target?)"
curl -fsSL "$url.sha256" -o "$tmp/$asset.sha256" || err "checksum download failed"

(
  cd "$tmp"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$asset.sha256" >/dev/null
  else
    shasum -a 256 -c "$asset.sha256" >/dev/null
  fi
) || err "checksum mismatch"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
mv "$tmp/pks" "$INSTALL_DIR/pks"
chmod +x "$INSTALL_DIR/pks"

printf 'installed pks to %s/pks\n' "$INSTALL_DIR"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) printf '\n%s is not on your PATH. Add this to your shell profile:\n  export PATH="%s:$PATH"\n' "$INSTALL_DIR" "$INSTALL_DIR" ;;
esac
