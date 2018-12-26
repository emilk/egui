#!/bin/bash
set -eu

echo "Lint and clean up typescript:"
tslint --fix docs/*.ts

echo "Cargo clippy"
cargo clippy
