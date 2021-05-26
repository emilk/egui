#!/bin/bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

# Pre-requisites:
rustup target add wasm32-unknown-unknown
cargo install -f wasm-bindgen-cli
cargo update -p wasm-bindgen
