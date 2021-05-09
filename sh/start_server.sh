#!/bin/bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

# Starts a local web-server that serves the contents of the `doc/` folder,
# i.e. the web-version of `egui_demo_app`.

echo "ensuring basic-http-server is installed..."
cargo install basic-http-server

echo "staritng server..."
echo "serving at http://localhost:8888"

(cd docs && basic-http-server --addr 127.0.0.1:8888 .)
# (cd docs && python3 -m http.server 8888 --bind 127.0.0.1)
