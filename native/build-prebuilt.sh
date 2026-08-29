#!/usr/bin/env bash
# Build ourin-native prebuilt binaries (gnu + musl) and stage them into
# native/platforms/. Run on a real x86_64 Linux toolchain (Codespaces) —
# production panels never compile.
#
# Requirements: rustup (stable), gcc, g++, cmake, make, zip/unzip tools,
# network (downloads leptonica 1.87 + tesseract 5.5.2 + eng.traineddata once;
# cached in ~/.local/share/tesseract-rs).
set -euo pipefail
cd "$(dirname "$0")"

export TESSERACT_EMBED_LANGUAGES=eng   # eng only — "tur" bloats the binary
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"

mkdir -p platforms/x86_64-unknown-linux-gnu platforms/x86_64-unknown-linux-musl

echo "== build gnu =="
cargo build --release --target x86_64-unknown-linux-gnu
cp "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/libourin_native.so" \
   platforms/x86_64-unknown-linux-gnu/ourin_native.linux-x64-gnu.node

echo "== build musl =="
cargo build --release --target x86_64-unknown-linux-musl
cp "$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/libourin_native.so" \
   platforms/x86_64-unknown-linux-musl/ourin_native.linux-x64-musl.node

# Note: musl target needs musl-gcc wrapper OR zig: if the musl build fails
# due to missing musl toolchain, install musl-tools (Debian/Ubuntu) or run
# with CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="zig cc -target x86_64-linux-musl".
echo "== staged =="
ls -la platforms/*/
