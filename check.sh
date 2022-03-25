#!/usr/bin/env bash
package="$(basename "$PWD")"
set -e
set -x
time cargo check
time cargo check --all-targets --all-features
time cargo build
time cargo build --all-targets --all-features
time cargo fmt -- --check
time cargo clippy -- -D clippy::pedantic
time cargo clippy --all-targets --all-features -- -D clippy::pedantic
./check-readme.sh
time cargo test
time cargo test --all-targets --all-features
time cargo publish --dry-run "$@"
echo "$0 finished"
