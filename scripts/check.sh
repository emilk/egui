#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -x

# Checks all tests, lints etc.
# Basically does what the CI does.

# cargo +1.92.0 install --quiet typos-cli

export RUSTFLAGS="-D warnings"
export RUSTDOCFLAGS="-D warnings" # https://github.com/emilk/egui/pull/1454

# Fast checks first:
typos
./scripts/lint.py
cargo fmt --all -- --check
cargo doc --quiet --lib --no-deps --all-features
cargo doc --quiet --document-private-items --no-deps --all-features
cargo clippy --quiet --all-targets --all-features -- -D warnings
cargo clippy --quiet --all-targets --all-features --release -- -D warnings # we need to check release mode too

./scripts/clippy_wasm.sh

cargo check --quiet  --all-targets
cargo check --quiet  --all-targets --all-features
cargo check --quiet  -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check --quiet  -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
# TODO(#5297) re-enable --all-features once the tests work with the unity feature
cargo test  --quiet --all-targets --all-features
cargo test  --quiet --doc # slow - checks all doc-tests

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cargo check --quiet -p eframe --no-default-features --features "glow","x11"
    cargo check --quiet -p eframe --no-default-features --features "glow","wayland"
    cargo check --quiet -p eframe --no-default-features --features "wgpu","x11"
    cargo check --quiet -p eframe --no-default-features --features "wgpu","wayland"
else
    cargo check --quiet -p eframe --no-default-features --features "glow"
    cargo check --quiet -p eframe --no-default-features --features "wgpu"
fi

cargo check --quiet -p egui --no-default-features --features "serde"

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cargo check --quiet -p egui_demo_app --no-default-features --features "glow","x11"
    cargo check --quiet -p egui_demo_app --no-default-features --features "glow","wayland"
    cargo check --quiet -p egui_demo_app --no-default-features --features "wgpu","x11"
    cargo check --quiet -p egui_demo_app --no-default-features --features "wgpu","wayland"
else
    cargo check --quiet -p egui_demo_app --no-default-features --features "glow"
    cargo check --quiet -p egui_demo_app --no-default-features --features "wgpu"
fi

cargo check --quiet -p egui_demo_lib --no-default-features
cargo check --quiet -p egui_extras --no-default-features
cargo check --quiet -p egui_glow --no-default-features
cargo check --quiet -p egui-winit --no-default-features --features "wayland"
cargo check --quiet -p egui-winit --no-default-features --features "x11"
cargo check --quiet -p emath --no-default-features
cargo check --quiet -p epaint --no-default-features --release
cargo check --quiet -p epaint --no-default-features

cargo check --quiet -p eframe --all-features
cargo check --quiet -p egui --all-features
cargo check --quiet -p egui_demo_app --all-features
cargo check --quiet -p egui_extras --all-features
cargo check --quiet -p egui_glow --all-features
cargo check --quiet -p egui-winit --all-features
cargo check --quiet -p emath --all-features
cargo check --quiet -p epaint --all-features

./scripts/wasm_bindgen_check.sh

./scripts/cargo_deny.sh

# TODO(emilk): consider using https://github.com/taiki-e/cargo-hack or https://github.com/frewsxcv/cargo-all-features

# ------------------------------------------------------------
#

# For finding bloat:
# cargo bloat --release --bin egui_demo_app -n 200 | rg egui
# Also try https://github.com/google/bloaty

# what compiles slowly?
# cargo clean && time cargo build -p eframe --timings
# https://fasterthanli.me/articles/why-is-my-rust-build-so-slow

# what compiles slowly?
# cargo llvm-lines --lib -p egui | head -20

echo "All checks passed."
