#!/bin/bash
set -eu

# Pre-requisites:
rustup target add wasm32-unknown-unknown
if ! wasm-bindgen --version; then
	cargo clean
	cargo install -f wasm-bindgen-cli
	cargo update
fi

# BUILD=debug
BUILD=release

# Clear output from old stuff:
rm -rf docs/*.wasm

echo "Build rust:"
# cargo build -p demo_web --target wasm32-unknown-unknown
cargo build --release -p demo_web --target wasm32-unknown-unknown

echo "Generate JS bindings for wasm:"
FOLDER_NAME=${PWD##*/}
TARGET_NAME="demo_web.wasm"
wasm-bindgen "target/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
  --out-dir docs --no-modules --no-typescript

open http://localhost:8888
