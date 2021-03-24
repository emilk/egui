#!/bin/bash
set -eu

CRATE_NAME="egui_demo_app"

# This is required to enable the web_sys clipboard API which egui_web uses
# https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.Clipboard.html
# https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html
export RUSTFLAGS=--cfg=web_sys_unstable_apis

# Clear output from old stuff:
rm -f docs/${CRATE_NAME}_bg.wasm

echo "Building rust…"
BUILD=release
FEATURES="http,persistence,screen_reader"

(
  cd egui_demo_app && cargo build \
    -p ${CRATE_NAME} \
    --release \
    --lib \
    --target wasm32-unknown-unknown \
    --features ${FEATURES}
)

echo "Generating JS bindings for wasm…"
TARGET_NAME="${CRATE_NAME}.wasm"
wasm-bindgen "target/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
  --out-dir docs --no-modules --no-typescript

# brew install wabt # to get wasm-strip
# wasm-strip docs/${CRATE_NAME}_bg.wasm

echo "Optimizing wasm…"
# brew install binaryen # to get wasm-opt
wasm-opt docs/egui_demo_app_bg.wasm -O2 --fast-math -o docs/egui_demo_app_bg.wasm # add -g to get debug symbols
echo "Finished docs/${CRATE_NAME}_bg.wasm"

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  # Linux, ex: Fedora
  xdg-open http://localhost:8888/index.html
elif [[ "$OSTYPE" == "msys" ]]; then
  # Windows
  start http://localhost:8888/index.html
else
  # Darmin aka MacOS or something else
  open http://localhost:8888/index.html
fi
