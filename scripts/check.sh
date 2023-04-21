#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -x

# Checks all tests, lints etc.
# Basically does what the CI does.

cargo install cargo-cranky # Uses lints defined in Cranky.toml. See https://github.com/ericseppanen/cargo-cranky

# web_sys_unstable_apis is required to enable the web_sys clipboard API which eframe web uses,
# as well as by the wasm32-backend of the wgpu crate.
export RUSTFLAGS="--cfg=web_sys_unstable_apis -D warnings"
export RUSTDOCFLAGS="-D warnings" # https://github.com/emilk/egui/pull/1454

cargo check --all-targets
cargo check --all-targets --all-features
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
cargo cranky --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc # slow - checks all doc-tests
cargo fmt --all -- --check

cargo doc --lib --no-deps --all-features
cargo doc --document-private-items --no-deps --all-features

(cd crates/eframe && cargo check --no-default-features --features "glow")
(cd crates/eframe && cargo check --no-default-features --features "wgpu")
(cd crates/egui && cargo check --no-default-features --features "serde")
(cd crates/egui_demo_app && cargo check --no-default-features --features "glow")
(cd crates/egui_demo_app && cargo check --no-default-features --features "wgpu")
(cd crates/egui_demo_lib && cargo check --no-default-features)
(cd crates/egui_extras && cargo check --no-default-features)
(cd crates/egui_glow && cargo check --no-default-features)
(cd crates/egui-winit && cargo check --no-default-features --features "wayland")
(cd crates/egui-winit && cargo check --no-default-features --features "winit/x11")
(cd crates/emath && cargo check --no-default-features)
(cd crates/epaint && cargo check --no-default-features --release)
(cd crates/epaint && cargo check --no-default-features)

(cd crates/eframe && cargo check --all-features)
(cd crates/egui && cargo check --all-features)
(cd crates/egui_demo_app && cargo check --all-features)
(cd crates/egui_extras && cargo check --all-features)
(cd crates/egui_glow && cargo check --all-features)
(cd crates/egui-winit && cargo check --all-features)
(cd crates/emath && cargo check --all-features)
(cd crates/epaint && cargo check --all-features)

./scripts/wasm_bindgen_check.sh

cargo cranky --target wasm32-unknown-unknown --all-features -p egui_demo_app --lib -- -D warnings

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
