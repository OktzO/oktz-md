#!/usr/bin/env bash
# Lingkungan build Rust untuk rewrite ourin-md (JS -> Rust).
# Dipakai sebagai:  source scripts/build-env.sh
#
# /tmp pernah di-reset EMPAT kali (2026-10-10, 10-15, 10-16 dua kali):
# toolchain, CARGO_TARGET_DIR, dan seluruh skrip bantu di /tmp/opencode/ hilang.
# Repo dan semua commit tetap utuh karena ada di /home. Skrip bantu yang dulunya
# hanya ada di /tmp sekarang sudah ada di repo (scripts/*.mjs) dan tahan reset;
# hanya toolchain yang harus dipasang ulang lewat prosedur di bawah.
#
# Isi:
#  - RUSTUP_HOME/CARGO_HOME di /tmp (partisi home hanya sisa ~600 MB).
#  - C toolchain dari nix store, karena `ring` + `libsqlite3-sys` butuh cc/ld.
#  - CARGO_TARGET_DIR di /tmp, karena release + lto=fat menghasilkan target GB.
#  - clippy dipasang otomatis (profile minimal tidak menyertakannya).

set -e

export RUSTUP_HOME=/tmp/opencode/rustup
export CARGO_HOME=/tmp/opencode/cargo
export CARGO_TARGET_DIR=/tmp/opencode/target

GCC_BIN=/nix/store/kks0nbx2riwry5qsx0qr87qr0lswmhzi-gcc-wrapper-14.3.0/bin
BINUTILS_BIN=/nix/store/bw67x2q2qvhwkx8mgcpnnh5zpf709m1h-binutils-wrapper-2.43.1/bin

export PATH="$CARGO_HOME/bin:$GCC_BIN:$BINUTILS_BIN:$PATH"
export CC=gcc
export CXX=g++
export AR=ar
export CARGO_TERM_COLOR=never
export CARGO_NET_RETRY=5

# Pasang ulang toolchain kalau /tmp kembali di-reset.
if [ ! -x "$CARGO_HOME/bin/cargo" ]; then
  echo "toolchain hilang (reset /tmp?) -- memasang ulang" >&2
  mkdir -p /tmp/opencode
  curl -sSf https://sh.rustup.rs -o /tmp/opencode/rustup-init.sh
  chmod +x /tmp/opencode/rustup-init.sh
  RUSTUP_HOME=$RUSTUP_HOME CARGO_HOME=$CARGO_HOME \
    /tmp/opencode/rustup-init.sh -y --default-toolchain stable \
    --profile minimal --no-modify-path >/dev/null
fi

# clippy bukan bagian profile minimal.
"$CARGO_HOME/bin/cargo-clippy" --version >/dev/null 2>&1 \
  || "$CARGO_HOME/bin/rustup" component add clippy >/dev/null