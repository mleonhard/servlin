#!/usr/bin/env bash
set -e
set -x
time cargo check --all-targets --all-features
time cargo build --all-targets --all-features
time cargo fmt -- --check
time cargo clippy --all-targets --all-features -- -D clippy::pedantic
time cargo test --all-targets --all-features
time cargo test --doc --all-features
./check-readme.sh
time cargo publish --dry-run "$@"
echo "$0 finished"
