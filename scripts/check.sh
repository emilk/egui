#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -x

# Checks all tests, lints etc.
# Basically does what the CI does.

cargo install cargo-cranky # Uses lints defined in Cranky.toml. See https://github.com/ericseppanen/cargo-cranky
cargo install typos-cli

# web_sys_unstable_apis is required to enable the web_sys clipboard API which eframe web uses,
# as well as by the wasm32-backend of the wgpu crate.
export RUSTFLAGS="--cfg=web_sys_unstable_apis -D warnings"
export RUSTDOCFLAGS="-D warnings" # https://github.com/emilk/egui/pull/1454

# Fast checks first:
typos
./scripts/lint.py
cargo fmt --all -- --check
cargo doc --quiet --lib --no-deps --all-features
cargo doc --quiet --document-private-items --no-deps --all-features

cargo cranky --quiet --all-targets --all-features -- -D warnings
./scripts/clippy_wasm.sh

cargo check --quiet  --all-targets
cargo check --quiet  --all-targets --all-features
cargo check --quiet  -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check --quiet  -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
cargo test  --quiet --all-targets --all-features
cargo test  --quiet --doc # slow - checks all doc-tests

(cd crates/eframe && cargo check --quiet --no-default-features --features "glow")
(cd crates/eframe && cargo check --quiet --no-default-features --features "wgpu")
(cd crates/egui && cargo check --quiet --no-default-features --features "serde")
(cd crates/egui_demo_app && cargo check --quiet --no-default-features --features "glow")
(cd crates/egui_demo_app && cargo check --quiet --no-default-features --features "wgpu")
(cd crates/egui_demo_lib && cargo check --quiet --no-default-features)
(cd crates/egui_extras && cargo check --quiet --no-default-features)
(cd crates/egui_glow && cargo check --quiet --no-default-features)
(cd crates/egui-winit && cargo check --quiet --no-default-features --features "wayland")
(cd crates/egui-winit && cargo check --quiet --no-default-features --features "x11")
(cd crates/emath && cargo check --quiet --no-default-features)
(cd crates/epaint && cargo check --quiet --no-default-features --release)
(cd crates/epaint && cargo check --quiet --no-default-features)

(cd crates/eframe && cargo check --quiet --all-features)
(cd crates/egui && cargo check --quiet --all-features)
(cd crates/egui_demo_app && cargo check --quiet --all-features)
(cd crates/egui_extras && cargo check --quiet --all-features)
(cd crates/egui_glow && cargo check --quiet --all-features)
(cd crates/egui-winit && cargo check --quiet --all-features)
(cd crates/emath && cargo check --quiet --all-features)
(cd crates/epaint && cargo check --quiet --all-features)

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
