#!/bin/bash
set -eu

# Usage: build_demo_web.sh [--open]

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

cargo build \
  -p ${CRATE_NAME} \
  --release \
  --lib \
  --target wasm32-unknown-unknown \
  --features ${FEATURES}

echo "Generating JS bindings for wasm…"
TARGET_NAME="${CRATE_NAME}.wasm"
wasm-bindgen "target/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
  --out-dir docs --no-modules --no-typescript

# to get wasm-strip:  apt/brew/dnf install wabt
# wasm-strip docs/${CRATE_NAME}_bg.wasm

echo "Optimizing wasm…"
# to get wasm-opt:  apt/brew/dnf install binaryen
wasm-opt docs/${CRATE_NAME}_bg.wasm -O2 --fast-math -o docs/${CRATE_NAME}_bg.wasm # add -g to get debug symbols
echo "Finished docs/${CRATE_NAME}_bg.wasm"

if [[ "${1:-}" == "--open" ]]; then
  if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux, ex: Fedora
    xdg-open http://localhost:8888/index.html
  elif [[ "$OSTYPE" == "msys" ]]; then
    # Windows
    start http://localhost:8888/index.html
  else
    # Darwin/MacOS, or something else
    open http://localhost:8888/index.html
  fi
fi
