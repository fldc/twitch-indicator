# Maintainer: Fredrik Storm <fredrik@fldc.se>
pkgname=twitch-indicator-git
pkgver=r7.d9a2350
pkgrel=1
pkgdesc="A Linux system tray application that monitors followed Twitch streams"
arch=('x86_64')
url="https://github.com/fldc/twitch-indicator"
license=('MIT')
depends=('gtk3' 'libappindicator-gtk3' 'openssl')
makedepends=('rust' 'cargo' 'pkg-config' 'git')
provides=('twitch-indicator')
conflicts=('twitch-indicator')
source=("git+$url.git")
sha256sums=('SKIP')

pkgver() {
    cd "$srcdir/${pkgname%-git}"
    printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
    cd "$srcdir/${pkgname%-git}"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --release --locked --all-features --target-dir=target
}

check() {
    cd "$srcdir/${pkgname%-git}"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --release --locked --all-features --target-dir=target
}

package() {
    cd "$srcdir/${pkgname%-git}"
    
    # Install the binary
    install -Dm755 target/release/twitch-indicator "$pkgdir/usr/bin/twitch-indicator"
    
    # Install the desktop file
    install -Dm644 twitch-indicator.desktop "$pkgdir/usr/share/applications/twitch-indicator.desktop"
    
    # Install license
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    
    # Install README
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
