#!/bin/bash
set -eu

# Pre-requisites:
rustup target add wasm32-unknown-unknown
if ! [[ $(wasm-bindgen --version) ]]; then
    cargo install wasm-bindgen-cli
fi

BUILD=debug
# BUILD=release

# Clear output from old stuff:
rm -rf docs/*.d.ts
rm -rf docs/*.js
rm -rf docs/*.wasm

echo "Build rust:"
cargo build --target wasm32-unknown-unknown

echo "Lint and clean up typescript:"
tslint --fix docs/*.ts

echo "Compile typescript:"
tsc

echo "Generate JS bindings for wasm:"

FOLDER_NAME=${PWD##*/}
TARGET_NAME="$FOLDER_NAME.wasm"
wasm-bindgen "target/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
  --out-dir docs --no-modules
  # --no-modules-global hoboho
