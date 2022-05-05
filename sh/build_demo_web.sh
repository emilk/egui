#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

CRATE_NAME="egui_demo_app"
FEATURES="http,persistence,screen_reader"

OPEN=false
FAST=false

while test $# -gt 0; do
  case "$1" in
    -h|--help)
      echo "build_demo_web.sh [--fast] [--open]"
      echo "  --fast: skip optimization step"
      echo "  --open: open the result in a browser"
      exit 0
      ;;

    # Skip running `wasm-opt`.
    # --fast also preserves debug symbols, which is great for profiling.
    --fast)
      shift
      FAST=true
      ;;

    --open)
      shift
      OPEN=true
      ;;

    *)
      break
      ;;
  esac
done

# This is required to enable the web_sys clipboard API which eframe web uses
# https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.Clipboard.html
# https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html
export RUSTFLAGS=--cfg=web_sys_unstable_apis

# Clear output from old stuff:
rm -f "docs/${CRATE_NAME}_bg.wasm"

echo "Building rust…"
BUILD=release

(cd $CRATE_NAME &&
  cargo build \
    --release \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features ${FEATURES}
)

# Get the output directory (in the workspace it is in another location)
TARGET=`cargo metadata --format-version=1 | jq --raw-output .target_directory`

echo "Generating JS bindings for wasm…"
TARGET_NAME="${CRATE_NAME}.wasm"
WASM_PATH="${TARGET}/wasm32-unknown-unknown/$BUILD/$TARGET_NAME"
wasm-bindgen "${WASM_PATH}" --out-dir docs --no-modules --no-typescript

# if this fails with "error: cannot import from modules (`env`) with `--no-modules`", you can use:
# wasm2wat target/wasm32-unknown-unknown/release/egui_demo_app.wasm | rg env
# wasm2wat target/wasm32-unknown-unknown/release/egui_demo_app.wasm | rg "call .now\b" -B 20 # What calls `$now` (often a culprit)

# to get wasm-strip:  apt/brew/dnf install wabt
# wasm-strip docs/${CRATE_NAME}_bg.wasm

if [[ "${FAST}" = false ]]; then
  echo "Optimizing wasm…"
  # to get wasm-opt:  apt/brew/dnf install binaryen
  wasm-opt "docs/${CRATE_NAME}_bg.wasm" -O2 --fast-math -o "docs/${CRATE_NAME}_bg.wasm" # add -g to get debug symbols
fi

echo "Finished docs/${CRATE_NAME}_bg.wasm"

if [[ "${OPEN}" == true ]]; then
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
