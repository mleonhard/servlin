#!/usr/bin/env bash
(
  set -e
  set -x
  ./check.sh "$@"
) || exit 1

if ! (git branch --show-current | grep -q -E '^main$'); then
  echo "Current git branch is not main."
  exit 1
fi

# Get version line from Cargo.toml.
version=$(cargo pkgid |cut -d '#' -f 2)
# Check the value.
if ! (echo "$version" | grep -q -E '^[0-9]+\.[0-9]+\.[0-9]+$'); then
  echo "Cargo.toml has invalid version '$version'"
  exit 1
fi

# Create git tag pointing at HEAD, if it doesn't already exist.
tag="v$version"
if [ -n "$(git tag --list "$tag")" ]; then
  if [ -n "$(git tag --list "$tag" --points-at HEAD)" ]; then
    echo "git tag '$tag' already exists and points at HEAD"
  else
    echo "git tag '$tag' already exists and doesn't point at HEAD.  Did you forget to bump the version in Cargo.toml?"
    exit 1
  fi
else
  echo "git tag '$tag' does not exist.  Creating it."
  (
    set -x
    git tag -m "$tag" "$tag"
  ) || exit 1
fi

set -e
set -x
git push --follow-tags
cargo +stable publish "$@"
