#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

set -x

# Pre-requisites:
rustup target add wasm32-unknown-unknown

# For generating JS bindings:
if ! cargo install --list | grep -q 'wasm-bindgen-cli v0.2.97'; then
    cargo install --force --quiet wasm-bindgen-cli --version 0.2.97
fi
