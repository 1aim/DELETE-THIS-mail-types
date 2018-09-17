#!/usr/bin/bash

set -eu

cargo test --all-features "$@"
cargo clean -p mail-base
cargo build "$@"
rustdoc --test README.md -L./target/debug/deps
