#!/usr/bin/env bash
# This script generates screenshots for all the examples in examples/

set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

cd examples
for EXAMPLE_NAME in $(ls -1d */ | sed 's/\/$//'); do
    if [ ${EXAMPLE_NAME} != "hello_world_par" ] && # screenshot not implemented for wgpu backend
       [ ${EXAMPLE_NAME} != "multiple_viewports" ] &&
       [ ${EXAMPLE_NAME} != "screenshot" ] &&
       [ ${EXAMPLE_NAME} != "puffin_viewer" ] &&
       [ ${EXAMPLE_NAME} != "serial_windows" ];
    then
        echo ""
        echo "Running ${EXAMPLE_NAME}…"
        EFRAME_SCREENSHOT_TO="temp.png" cargo run -p ${EXAMPLE_NAME}
        pngcrush -rem allb -brute -reduce temp.png "${EXAMPLE_NAME}/screenshot.png"
        rm temp.png
    fi
done
