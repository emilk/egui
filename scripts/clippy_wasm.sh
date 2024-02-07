#!/usr/bin/env bash
# This scripts run clippy on the wasm32-unknown-unknown target,
# using a special clippy.toml config file which forbids a few more things.

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -x

# Use scripts/clippy_wasm/clippy.toml
export CLIPPY_CONF_DIR="scripts/clippy_wasm"

cargo cranky --quiet --all-features --target wasm32-unknown-unknown --target-dir target_wasm -p egui_demo_app --lib -- --deny warnings
