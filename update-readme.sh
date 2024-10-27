#!/usr/bin/env bash
set -e
package="$(basename "$PWD")"
usage() {
  echo "${package}: ERROR: $1" >&2
  echo usage: "${package}" '[--directory DIRECTORY] [--filename FILENAME]' >&2
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

time (
  # "--target not used?"
  # https://github.com/rust-secure-code/cargo-geiger/issues/95
  set -x
  cargo geiger --update-readme --readme-path "$filename" --output-format GitHubMarkdown --all-features
  set +x
)
set +e
echo "Done."
