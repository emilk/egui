#!/usr/bin/env bash
# This script searches for the last CI run with your branch name, downloads the test_results artefact
# and replaces your existing snapshots with the new ones.
# Make sure you have the gh cli installed and authenticated before running this script.
# If prompted to select a default repo, choose the emilk/egui one

set -eu

BRANCH=$(git rev-parse --abbrev-ref HEAD)

RUN_ID=$(gh run list --branch "$BRANCH" --workflow "Rust" --json databaseId -q '.[0].databaseId')

echo "Downloading test results from run $RUN_ID from branch $BRANCH"

# remove any existing .new.png that might have been left behind
find . -type d -path "*/tests/snapshots*" | while read dir; do
    find "$dir" -type f -name "*.new.png" | while read file; do
        rm "$file"
    done
done


gh run download "$RUN_ID" --name "test-results" --dir tmp_artefacts

# move the snapshots to the correct location, overwriting the existing ones
rsync -a tmp_artefacts/ .

rm -r tmp_artefacts

# rename the .new.png files to .png
find . -type d -path "*/tests/snapshots*" | while read dir; do
    find "$dir" -type f -name "*.new.png" | while read file; do
        mv -f "$file" "${file%.new.png}.png"
    done
done

echo "Done!"
