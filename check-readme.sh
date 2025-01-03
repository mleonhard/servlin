#!/usr/bin/env bash
set -e
set -x
cat Readme.md |perl -0777 -pe 's/(# Cargo Geiger Safety Report).+?```.+?```/$1/s' >Readme.md.pruned
cargo readme --no-title --no-indent-headings >Readme.md.tmp
diff Readme.md.pruned Readme.md.tmp || (
  set +x
  echo "ERROR: $(pwd)/Readme.md is stale" >&2
  exit 1
)
rm -f Readme.md.pruned Readme.md.tmp
git rm -f --ignore-unmatch Readme.md.pruned Readme.md.tmp
