#!/usr/bin/env bash
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -eux

# Checks all tests, lints etc.
# Basically does what the CI does.

RUSTDOCFLAGS="-D warnings" # https://github.com/emilk/egui/pull/1454

cargo check --workspace --all-targets
cargo check --workspace --all-targets --all-features
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc # slow - checks all doc-tests
cargo fmt --all -- --check

cargo doc -p emath -p epaint -p egui -p eframe -p epi -p egui_web -p egui-winit -p egui_glium -p egui_glow --lib --no-deps --all-features
cargo doc -p egui_web --target wasm32-unknown-unknown --lib --no-deps --all-features
cargo doc --document-private-items --no-deps --all-features

(cd eframe && cargo check --no-default-features)
(cd egui && cargo check --no-default-features --features "serde")
(cd egui_demo_app && cargo check --no-default-features)
(cd egui_demo_lib && cargo check --no-default-features)
(cd egui_extras && cargo check --no-default-features)
(cd egui_glium && cargo check --no-default-features)
(cd egui_glow && cargo check --no-default-features)
(cd egui_web && cargo check --no-default-features)
(cd egui-winit && cargo check --no-default-features)
(cd emath && cargo check --no-default-features)
(cd epaint && cargo check --no-default-features --release)
(cd epaint && cargo check --no-default-features)
(cd epi && cargo check --no-default-features)


(cd eframe && cargo check --all-features)
(cd egui && cargo check --all-features)
(cd egui_demo_app && cargo check --all-features)
(cd egui_extras && cargo check --all-features)
(cd egui_glium && cargo check --all-features)
(cd egui_glow && cargo check --all-features)
(cd egui_web && cargo check --all-features)
(cd egui-winit && cargo check --all-features)
(cd emath && cargo check --all-features)
(cd epaint && cargo check --all-features)
(cd epi && cargo check --all-features)

./sh/wasm_bindgen_check.sh

# cargo install cargo-deny
cargo deny check

# ------------------------------------------------------------
#

# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui
# Also try https://github.com/google/bloaty

# what compiles slowly?
# https://fasterthanli.me/articles/why-is-my-rust-build-so-slow
# (cd egui_demo_app && cargo clean && RUSTC_BOOTSTRAP=1 cargo build --release --quiet -Z timings)

# what compiles slowly?
# cargo llvm-lines --lib -p egui | head -20

echo "All checks passed."
