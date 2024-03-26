#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

if [[ $* == --skip-setup ]]
then
  echo "Skipping setup_web.sh"
else
  echo "Running setup_web.sh"
  ./scripts/setup_web.sh
fi

CRATE_NAME="egui_demo_app"
FEATURES="glow,http,persistence"

# This is required to enable the web_sys clipboard API which eframe web uses
# https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.Clipboard.html
# https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html
export RUSTFLAGS=--cfg=web_sys_unstable_apis

echo "Building rust…"
BUILD=debug # debug builds are faster

(cd crates/$CRATE_NAME &&
  cargo build \
    --quiet \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features ${FEATURES}
)

TARGET="target"

echo "Generating JS bindings for wasm…"

rm -f "${CRATE_NAME}_bg.wasm" # Remove old output (if any)

TARGET_NAME="${CRATE_NAME}.wasm"
wasm-bindgen "${TARGET}/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
  --out-dir . --no-modules --no-typescript

# Remove output:
rm -f "${CRATE_NAME}_bg.wasm"
rm -f "${CRATE_NAME}.js"
