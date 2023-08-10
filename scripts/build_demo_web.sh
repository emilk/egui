#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

./scripts/setup_web.sh

# This is required to enable the web_sys clipboard API which eframe web uses
# https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.Clipboard.html
# https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html
export RUSTFLAGS=--cfg=web_sys_unstable_apis

CRATE_NAME="egui_demo_app"

 # NOTE: persistence use up about 400kB (10%) of the WASM!
FEATURES="http,persistence,web_screen_reader"

OPEN=false
OPTIMIZE=false
BUILD=debug
BUILD_FLAGS=""
WEB_GPU=false

while test $# -gt 0; do
  case "$1" in
    -h|--help)
      echo "build_demo_web.sh [--release] [--webgpu] [--open]"
      echo ""
      echo "  --release: Build with --release, and enable extra optimization step"
      echo "             Runs wasm-opt."
      echo "             NOTE: --release also removes debug symbols which are otherwise useful for in-browser profiling."
      echo ""
      echo "  --webgpu:  Build a binary for WebGPU instead of WebGL"
      echo "             Note that the resulting wasm will ONLY work on browsers with WebGPU."
      echo ""
      echo "  --open:    Open the result in a browser"
      exit 0
      ;;

    --release)
      shift
      OPTIMIZE=true
      BUILD="release"
      BUILD_FLAGS="--release"
      ;;

    --webgpu)
      shift
      WEB_GPU=true
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

OUT_FILE_NAME="egui_demo_app"

if [[ "${WEB_GPU}" == true ]]; then
  FEATURES="${FEATURES},wgpu"
else
  FEATURES="${FEATURES},glow"
fi

FINAL_WASM_PATH=docs/${OUT_FILE_NAME}_bg.wasm

# Clear output from old stuff:
rm -f "${FINAL_WASM_PATH}"

echo "Building rust…"

(cd crates/$CRATE_NAME &&
  cargo build \
    ${BUILD_FLAGS} \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features ${FEATURES}
)

# Get the output directory (in the workspace it is in another location)
# TARGET=`cargo metadata --format-version=1 | jq --raw-output .target_directory`
TARGET="target"

echo "Generating JS bindings for wasm…"
TARGET_NAME="${CRATE_NAME}.wasm"
WASM_PATH="${TARGET}/wasm32-unknown-unknown/$BUILD/$TARGET_NAME"
wasm-bindgen "${WASM_PATH}" --out-dir docs --out-name ${OUT_FILE_NAME} --no-modules --no-typescript

# if this fails with "error: cannot import from modules (`env`) with `--no-modules`", you can use:
# wasm2wat target/wasm32-unknown-unknown/release/egui_demo_app.wasm | rg env
# wasm2wat target/wasm32-unknown-unknown/release/egui_demo_app.wasm | rg "call .now\b" -B 20 # What calls `$now` (often a culprit)

# to get wasm-strip:  apt/brew/dnf install wabt
# wasm-strip ${FINAL_WASM_PATH}

if [[ "${OPTIMIZE}" = true ]]; then
  echo "Optimizing wasm…"
  # to get wasm-opt:  apt/brew/dnf install binaryen
  wasm-opt "${FINAL_WASM_PATH}" -O2 --fast-math -o "${FINAL_WASM_PATH}" # add -g to get debug symbols
fi

echo "Finished ${FINAL_WASM_PATH}"

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
