#!/usr/bin/env bash
set -e
set -x
time cargo check --all-targets --all-features
(cd bench && time cargo check --all-targets --all-features)
time cargo build --all-targets --all-features
(cd bench && time cargo build --all-targets --all-features)
time cargo fmt --all
(cd bench && time cargo fmt --all)
time cargo clippy --all-targets --all-features --allow-dirty --allow-staged --fix -- -D clippy::pedantic
(cd bench && time cargo clippy --all-targets --all-features --allow-dirty --allow-staged --fix -- -D clippy::pedantic)
time cargo test --all-targets --all-features
time cargo test --doc --all-features
./check-readme.sh
time cargo publish --dry-run "$@"
echo "$0 finished"
