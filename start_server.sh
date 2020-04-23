#!/bin/bash
set -eu

cd docs
echo "open http://localhost:8888"
python3 -m http.server 8888 --bind 127.0.0.1
