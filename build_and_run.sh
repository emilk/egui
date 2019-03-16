#!/bin/bash
set -eu

./build_wasm.sh
open "docs/index.html"

# TODO: release is only because of this bug: https://github.com/tomaka/glutin/pull/1099
# cargo run --release --bin example_glium

