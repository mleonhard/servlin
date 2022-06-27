#!/usr/bin/env bash
set -e
usage() {
  echo "$(basename "$0"): ERROR: $1" >&2
  echo "usage: $(basename "$0") [--filename FILENAME]" >&2
  exit 1
}

filename=Readme.md
while [ $# -gt 0 ]; do
  case "$1" in
  --filename)
    shift
    [ -n "$1" ] || usage "missing parameter to --filename argument"
    filename="$1"
    ;;
  *) usage "bad argument '$1'" ;;
  esac
  shift
done

echo "PWD=$(pwd)"
set -x
cargo readme --no-title --no-indent-headings >"$filename"
set +x

if grep --quiet 'Cargo Geiger Safety Report' src/lib.rs; then
  time (
    # "--target not used?"
    # https://github.com/rust-secure-code/cargo-geiger/issues/95
    # "WARNING: Dependency file was never scanned:... errors"
    # https://github.com/rust-secure-code/cargo-geiger/issues/145
    set -x
    cargo geiger --all-features --update-readme --readme-path "$filename" --output-format GitHubMarkdown --build-dependencies || true
    set +x
  )
fi
set +e
echo "Done."
