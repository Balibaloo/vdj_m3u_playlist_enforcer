#!/usr/bin/env bash
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)
OUTDIR="$ROOT/packaging/dist"

mkdir -p "$OUTDIR"

echo "Building release binary..."
cargo build --release --manifest-path "$ROOT/Cargo.toml"

BINARY="$ROOT/target/release/vdj-m3u-playlist-enforcer"
if [ ! -f "$BINARY" ]; then
  echo "Release binary not found: $BINARY" >&2
  exit 1
fi

PKGNAME="vdj-m3u-playlist-enforcer-$(date +%Y%m%d%H%M%S)"
PKGDIR="$OUTDIR/$PKGNAME"
mkdir -p "$PKGDIR"

echo "Copying files..."
cp "$BINARY" "$PKGDIR/vdj-m3u-playlist-enforcer"
cp -r "$ROOT/config/example-config.toml" "$PKGDIR/"
mkdir -p "$PKGDIR/systemd"
cp -r "$ROOT/systemd"/* "$PKGDIR/systemd/" 2>/dev/null || true

echo "Creating tarball..."
pushd "$OUTDIR" >/dev/null
	tar czf "$PKGNAME.tar.gz" "$PKGNAME"
popd >/dev/null

echo "Package created: $OUTDIR/$PKGNAME.tar.gz"
echo "Contents:"
tar -tzf "$OUTDIR/$PKGNAME.tar.gz"

echo "To install on a system (example):"
echo "# extract"
echo "sudo tar xzf /path/to/$PKGNAME.tar.gz -C /opt/"
echo "# move binary"
echo "sudo install -m 0755 /opt/$PKGNAME/vdj-m3u-playlist-enforcer /usr/bin/vdj-m3u-playlist-enforcer"
echo "# copy config"
echo "sudo install -m 0644 /opt/$PKGNAME/example-config.toml /etc/vdj-m3u-playlist-enforcer/config.toml"
echo "# copy systemd unit and enable"
echo "sudo install -Dm644 /opt/$PKGNAME/systemd/vdj-m3u-playlist-enforcer.service /etc/systemd/system/vdj-m3u-playlist-enforcer.service"
echo "sudo systemctl daemon-reload"
echo "sudo systemctl enable --now vdj-m3u-playlist-enforcer.service"

exit 0
