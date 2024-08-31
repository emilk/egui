#!/usr/bin/env bash

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
#set -x

export RUSTFLAGS="-D warnings"
cargo +1.76.0 install --quiet cargo-hack
# We maybe should also check "egui-wgpu" and "egui_demo_app"
members=("ecolor" "eframe" "egui" "egui_demo_lib" "egui_extras" "egui_glow" "egui-winit" "emath" "epaint" "epaint_default_fonts")

for member in "${members[@]}"; do
    echo "Checking $member"
    cargo hack check --each-feature --no-dev-deps --quiet --clean-per-run --manifest-path "crates/${member}/Cargo.toml"
done

echo "All checks passed!"
