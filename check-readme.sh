#!/usr/bin/env bash
set -e
set -x
cargo readme >Readme.md.tmp
diff Readme.md Readme.md.tmp || (
  echo "ERROR: Readme.md is stale" >&2
  exit 1
)
rm -f Readme.md.tmp
git rm -f --ignore-unmatch Readme.md.tmp
