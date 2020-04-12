#!/bin/bash
set -eu

cd docs
echo "open localhost:8000"
python3 -m http.server 8000 --bind 127.0.0.1
