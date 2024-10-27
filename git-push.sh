#!/usr/bin/env sh
set -e
set -x
../check-all.sh "$@"
git push --follow-tags
