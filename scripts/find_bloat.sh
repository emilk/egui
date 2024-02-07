#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

cargo bloat --release --bin egui_demo_app -n 200 | rg "egui "

cargo llvm-lines -p egui_demo_lib | rg egui | head -30
