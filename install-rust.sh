#!/bin/sh

set -ex

detect_arch() {
	deb_arch=$(dpkg --print-architecture)
	case $deb_arch in
	amd64)
		printf "x86_64-unknown-linux-gnu"
		;;
	aarch64 | arm64)
		printf "aarch64-unknown-linux-gnu"
		;;
	armhf)
		printf "armv7-unknown-linux-gnueabihf"
		;;
	armel)
		printf "arm-unknown-linux-gnueabi"
		;;
	i386)
		printf "i686-unknown-linux-gnu"
		;;
	*)
		printf "unknown"
		;;
	esac
}

rust_triple=$(detect_arch)
[ "$rust_triple" != "unknown" ] || (echo "Unknown architecture: $deb_arch" && exit 1)

home=$(pwd)
target="target/$rust_triple/release"

# Download rustup-init
mkdir -p "$target" &&
	cd "$target" &&
	curl -LO "https://static.rust-lang.org/rustup/dist/$rust_triple/rustup-init" &&
	chmod 755 rustup-init

cd "$home"
curl -LO "https://static.rust-lang.org/rustup/dist/$rust_triple/rustup-init.sha256"

cat rustup-init.sha256

# Verify checksum
sha256sum --check rustup-init.sha256

# Run rustup-init
exec "./$target/rustup-init" -v -y --default-toolchain "stable-$rust_triple"
