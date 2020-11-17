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

export RUSTFLAGS=--cfg=web_sys_unstable_apis # required for the clipboard API

# Clear output from old stuff:
rm -rf docs/example_web.wasm

echo "Build rust:"
# cargo build -p example_web --target wasm32-unknown-unknown
cargo build --release -p example_web --target wasm32-unknown-unknown

echo "Generate JS bindings for wasm:"
FOLDER_NAME=${PWD##*/}
TARGET_NAME="example_web.wasm"
wasm-bindgen "target/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
  --out-dir docs --no-modules --no-typescript

open http://localhost:8888/example.html
