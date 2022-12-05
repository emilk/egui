#!/usr/bin/env bash
# This script generates screenshots for all the examples in examples/

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

cd examples
for VARIABLE in $(ls -1d */ | sed 's/\/$//'); do
    EFRAME_SCREENSHOT_TO="$VARIABLE/screenshot.png" cargo run -p $VARIABLE
done
