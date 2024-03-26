#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/"
set -x

for example_name in *; do
    if [ -d "$example_name" ]; then
        cargo run --quiet -p $example_name
    fi
done
