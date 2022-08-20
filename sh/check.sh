#!/usr/bin/env bash
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -eux

# Checks all tests, lints etc.
# Basically does what the CI does.

cargo install cargo-cranky # Uses lints defined in Cranky.toml. See https://github.com/ericseppanen/cargo-cranky

RUSTFLAGS="-D warnings"
RUSTDOCFLAGS="-D warnings" # https://github.com/emilk/egui/pull/1454

cargo check --workspace --all-targets
cargo check --workspace --all-targets --all-features
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
cargo cranky --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc # slow - checks all doc-tests
cargo fmt --all -- --check

cargo doc -p eframe -p egui -p egui_demo_lib -p egui_extras -p egui_glium -p egui_glow -p egui-winit -p emath -p epaint --lib --no-deps --all-features
cargo doc --document-private-items --no-deps --all-features

(cd crates/eframe && cargo check --no-default-features --features "glow")
(cd crates/eframe && cargo check --no-default-features --features "wgpu")
(cd crates/egui && cargo check --no-default-features --features "serde")
(cd crates/egui_demo_app && cargo check --no-default-features --features "glow")
(cd crates/egui_demo_app && cargo check --no-default-features --features "wgpu")
(cd crates/egui_demo_lib && cargo check --no-default-features)
(cd crates/egui_extras && cargo check --no-default-features)
(cd crates/egui_glium && cargo check --no-default-features)
(cd crates/egui_glow && cargo check --no-default-features)
(cd crates/egui-winit && cargo check --no-default-features)
(cd crates/emath && cargo check --no-default-features)
(cd crates/epaint && cargo check --no-default-features --release)
(cd crates/epaint && cargo check --no-default-features)

(cd crates/eframe && cargo check --all-features)
(cd crates/egui && cargo check --all-features)
(cd crates/egui_demo_app && cargo check --all-features)
(cd crates/egui_extras && cargo check --all-features)
(cd crates/egui_glium && cargo check --all-features)
(cd crates/egui_glow && cargo check --all-features)
(cd crates/egui-winit && cargo check --all-features)
(cd crates/emath && cargo check --all-features)
(cd crates/epaint && cargo check --all-features)

./sh/wasm_bindgen_check.sh

# cargo install cargo-deny
cargo deny check

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
