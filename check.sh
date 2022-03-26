#!/usr/bin/env bash
cd "$(basename "$PWD")"
echo PWD=$PWD
set -e
set -x
time cargo check
time cargo check --all-targets --all-features
time cargo build
time cargo build --all-targets --all-features
time cargo fmt -- --check
time cargo clippy -- -D clippy::pedantic
time cargo clippy --all-targets --all-features -- -D clippy::pedantic
time cargo test
time cargo test --all-targets --all-features
./check-readme.sh
time cargo publish --dry-run "$@"
echo "$0 finished"
