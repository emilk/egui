#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

# Pre-requisites:
rustup target add wasm32-unknown-unknown

# For generating JS bindings:
# cargo install wasm-bindgen-cli --version 0.2.84
# We use a patched version containing this critical fix: https://github.com/rustwasm/wasm-bindgen/pull/3310
# See https://github.com/rerun-io/wasm-bindgen/commits/0.2.84-patch
cargo install wasm-bindgen-cli --git https://github.com/rerun-io/wasm-bindgen.git --rev 13283975ddf48c2d90758095e235b28d381c5762
