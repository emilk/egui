#!/bin/bash
set -eu

# Starts a local web-server that servs the contents of the `doc/` folder,
# i.e. the web-version of `egui_demo_app`.

cargo install basic-http-server

echo "open http://localhost:8888"

(cd docs && basic-http-server --addr 127.0.0.1:8888 .)
# (cd docs && python3 -m http.server 8888 --bind 127.0.0.1)
