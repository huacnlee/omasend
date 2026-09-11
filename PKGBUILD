# Maintainer: Jason Lee <huacnlee@gmail.com>
pkgname=omasend
pkgver=0.1.3
pkgrel=1
pkgdesc='Omarchy-native LocalSend client'
arch=('x86_64' 'aarch64')
url='https://github.com/huacnlee/omasend'
license=('MIT')
depends=(
  'fontconfig'
  'freetype2'
  'gcc-libs'
  'glibc'
  'hicolor-icon-theme'
  'libxkbcommon'
  'libxcb'
  'omarchy'
  'openssl'
  'vulkan-icd-loader'
  'wayland'
  'zstd'
)
makedepends=('cargo' 'clang' 'cmake' 'pkgconf')
optdepends=('wl-clipboard: paste clipboard content on Wayland')
options=('!debug')

# This recipe is built from a repository checkout, as used by Omarchy's local
# package workflow and by `makepkg` run from a clone.
source=()

build() {
  export CARGO_TARGET_DIR="$srcdir/target"
  cd "$startdir"
  cargo build --release --locked
}

check() {
  export CARGO_TARGET_DIR="$srcdir/target"
  cd "$startdir"
  cargo test --locked --no-default-features
}

package() {
  cd "$startdir"
  install -Dm755 "$srcdir/target/release/omasend" "$pkgdir/usr/bin/omasend"
  install -Dm644 packaging/omasend.desktop \
    "$pkgdir/usr/share/applications/omasend.desktop"
  install -Dm644 assets/omasend.png \
    "$pkgdir/usr/share/icons/hicolor/1024x1024/apps/omasend.png"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
