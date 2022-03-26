#!/usr/bin/env bash
cd "$(basename "$PWD")"
echo PWD=$PWD
set -e
set -x
rm -f Readme.md.tmp
./update-readme.sh --filename Readme.md.tmp
diff Readme.md Readme.md.tmp || (
  echo "ERROR: Readme.md is stale" >&2
  exit 1
)
rm -f Readme.md.tmp
git rm -f --ignore-unmatch Readme.md.tmp
