#!/bin/sh
# Rebuild the browser runtime: the wasm module and the minified shim.
#
# Run this whenever anything under `src/` or `../next-rust-server/src/wasm_runtime/shim.js`
# changes. It writes
#   crates/next-rust-server/src/wasm_runtime/runtime.wasm
#   crates/next-rust-server/src/wasm_runtime/shim.min.js
# and prints the hashes to paste into `client_runtime.rs`. The tests
# `wasm_runtime_matches_source` and `shim_matches_source` fail until you do.
#
# Needs: rustup target add wasm32-unknown-unknown, wasm-opt (binaryen), npx.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out="$root/crates/next-rust-server/src/wasm_runtime"
target=${CARGO_TARGET_DIR:-$root/target}
mkdir -p "$out"

# The workspace's release profile is tuned for the server (thin LTO, four
# codegen units, unwinding). None of that is right for a module the browser
# downloads, and a profile in this crate's own manifest would be ignored —
# cargo reads profiles from the workspace root only. So they go in the
# environment, where they apply to this build and to nothing else. Measured:
# leaving them out costs 250 KB of the 400 KB module.
echo "> cargo build --release --target wasm32-unknown-unknown"
CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
CARGO_PROFILE_RELEASE_LTO=fat \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_PANIC=abort \
CARGO_PROFILE_RELEASE_STRIP=true \
CARGO_PROFILE_RELEASE_DEBUG=false \
CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false \
CARGO_PROFILE_RELEASE_INCREMENTAL=false \
  cargo build -p next-rust-runtime --release --target wasm32-unknown-unknown

raw="$target/wasm32-unknown-unknown/release/next_rust_runtime.wasm"
# `wasm-opt` needs these enabled explicitly: rustc emits bulk-memory and
# non-trapping float casts by default, and binaryen's validator rejects them
# otherwise (it then writes nothing, which is easy to mistake for success).
echo "> wasm-opt -Oz"
wasm-opt -Oz \
  --enable-bulk-memory --enable-nontrapping-float-to-int \
  --enable-sign-ext --enable-mutable-globals \
  --strip-debug --strip-producers --strip-target-features \
  "$raw" -o "$out/runtime.wasm"

echo "> terser shim.js"
node --check "$out/shim.js"
npx --yes terser "$out/shim.js" --module --compress passes=3 --mangle toplevel=true -o "$out/shim.min.js"

ls -l "$out/runtime.wasm" "$out/shim.min.js"
echo
echo "Now run:  cargo test -p next-rust-server --lib client_runtime"
echo "If minified_shim_matches_source fails, paste the hash it prints into"
echo "RUNTIME_SHIM_SOURCE_HASH in crates/next-rust-server/src/client_runtime.rs."
echo "The module needs no hash: the_wasm_runtime_exports_what_the_shim_calls"
echo "checks it is a real module with the exports the shim calls."
