#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e

cd "$(dirname $0)"

export NEAR_ENABLE_SANDBOX_LOG=1
cargo run --example set_method
