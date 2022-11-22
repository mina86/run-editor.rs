#!/bin/sh

set -eux

cargo check
cargo check --no-default-features
cargo check --examples
cargo test
