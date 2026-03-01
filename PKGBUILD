pkgname=vdj-m3u-playlist-enforcer
pkgver=0.1.0
pkgrel=1
pkgdesc="Convert VirtualDJ .vdjfolder playlists into .m3u"
arch=('x86_64')
url="https://github.com/Balibaloo/vdj_m3u_playlist_enforcer"
license=('MIT')
depends=('systemd')
makedepends=('rust' 'cargo')
source=()
sha256sums=()

build() {
  local target_dir="$srcdir/target"
  cd "$srcdir"
  CARGO_TARGET_DIR="$target_dir" cargo build --release --locked
}

package() {
  local target_dir="$srcdir/target"
  local root_src="$(dirname "$srcdir")"

  install -Dm755 "$target_dir/release/vdj_m3u_playlist_enforcer" "$pkgdir/usr/bin/vdj-m3u-playlist-enforcer"
  install -Dm644 "$root_src/config/example-config.toml" "$pkgdir/etc/vdj-m3u-playlist-enforcer/example-config.toml"

  install -Dm644 "$root_src/systemd/vdj-m3u-playlist-enforcer.service" \
    "$pkgdir/usr/lib/systemd/system/vdj-m3u-playlist-enforcer.service"
  install -Dm644 "$root_src/systemd/vdj-m3u-playlist-enforcer-scan.service" \
    "$pkgdir/usr/lib/systemd/system/vdj-m3u-playlist-enforcer-scan.service"
  install -Dm644 "$root_src/systemd/vdj-m3u-playlist-enforcer-scan.timer" \
    "$pkgdir/usr/lib/systemd/system/vdj-m3u-playlist-enforcer-scan.timer"
}

install=vdj-m3u-playlist-enforcer.install