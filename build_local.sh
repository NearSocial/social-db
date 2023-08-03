#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e

cd "$(dirname $0)"
mkdir -p res

rustup target add wasm32-unknown-unknown
RUSTFLAGS='-C link-arg=-s' cargo build --package contract --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/contract.wasm ./res/social_db_local.wasm
