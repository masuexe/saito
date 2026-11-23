#!/usr/bin/env bash
# Build and package the sandboxed historical WASM simulation kernel (`saito-sim-<line>.wasm.gz`)
# for the current workspace version.
#
# Requirements:
#   - rustup target add wasm32-wasip1
#   - WASI_SDK_PATH pointing to wasi-sdk 34 (see crates/saito-luau/README.md)
#
# Usage:
#   packaging/sim-kernel/build.sh [output-dir]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${1:-$ROOT/target/package/sim-kernel}"

if [[ -z "${WASI_SDK_PATH:-}" || ! -d "$WASI_SDK_PATH" ]]; then
  echo "error: set WASI_SDK_PATH to wasi-sdk 34 (see crates/saito-luau/README.md)" >&2
  exit 1
fi

VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
LINE="$(awk -F. '{ print $1 "." $2 }' <<<"$VERSION")"
NAME="saito-sim-${LINE}"

echo "Building ${NAME}.wasm (engine ${VERSION}, profile sim-kernel)..."
cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  -p saito-sim-guest \
  --target wasm32-wasip1 \
  --profile sim-kernel

RAW_WASM="$ROOT/target/wasm32-wasip1/sim-kernel/saito_sim_guest.wasm"
mkdir -p "$OUT_DIR"
OUT_WASM="$OUT_DIR/${NAME}.wasm"
cp "$RAW_WASM" "$OUT_WASM"

if command -v wasm-opt >/dev/null 2>&1; then
  echo "Optimizing ${NAME}.wasm with wasm-opt..."
  wasm-opt -O2 \
    --enable-exception-handling \
    --enable-bulk-memory \
    --enable-mutable-globals \
    --enable-nontrapping-float-to-int \
    --enable-sign-ext \
    --enable-reference-types \
    --enable-multivalue \
    "$OUT_WASM" -o "$OUT_WASM.opt"
  mv "$OUT_WASM.opt" "$OUT_WASM"
fi

SHA256="$(sha256sum "$OUT_WASM" | awk '{ print $1 }')"
OUT_GZ="$OUT_DIR/${NAME}.wasm.gz"
gzip -9 -c "$OUT_WASM" > "$OUT_GZ"
GZ_SIZE="$(wc -c < "$OUT_GZ" | tr -d ' ')"

cat > "$OUT_DIR/${NAME}.json" <<EOF
{
  "line": "${LINE}",
  "engine": "${VERSION}",
  "abi_version": 1,
  "sha256": "${SHA256}",
  "size": ${GZ_SIZE},
  "file": "${NAME}.wasm.gz"
}
EOF

echo "Packaged:"
echo "  wasm:   $OUT_WASM ($(wc -c < "$OUT_WASM" | tr -d ' ') bytes)"
echo "  gzip:   $OUT_GZ (${GZ_SIZE} bytes)"
echo "  sha256: ${SHA256}"
echo "  meta:   $OUT_DIR/${NAME}.json"
