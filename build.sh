#!/bin/bash
set -eu

# Pre-requisites:
rustup target add wasm32-unknown-unknown
if ! [[ $(wasm-bindgen --version) ]]; then
    cargo install -f wasm-bindgen-cli
fi

if ! [[ -f docs/webassembly.d.ts ]]; then
	curl https://raw.githubusercontent.com/01alchemist/webassembly-types/master/webassembly.d.ts > docs/webassembly.d.ts
fi

BUILD=debug
# BUILD=release

# Clear output from old stuff:
# rm -rf docs/*.d.ts
rm -rf docs/*.js
rm -rf docs/*.wasm

function build_rust
{
	echo "Build rust:"
	cargo build --target wasm32-unknown-unknown

	echo "Generate JS bindings for wasm:"
	FOLDER_NAME=${PWD##*/}
	TARGET_NAME="$FOLDER_NAME.wasm"
	wasm-bindgen "target/wasm32-unknown-unknown/$BUILD/$TARGET_NAME" \
	  --out-dir docs --no-modules
	  # --no-modules-global hoboho
}

build_rust

echo "Compile typescript:"
tsc

# wait || exit $?

# 3.4 s
