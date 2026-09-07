#!/usr/bin/env bash
# Prepare a tag-specific changelog while keeping one source file per base version.
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
    printf 'Usage: %s TAG OUTPUT\n' "$0" >&2
    exit 1
fi

TAG="$1"
OUTPUT="$2"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ ! "$TAG" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)(-beta\.[1-9][0-9]*)?$ ]]; then
    printf 'error: release tag must be vX.Y.Z or vX.Y.Z-beta.N: %s\n' "$TAG" >&2
    exit 1
fi

BASE_VERSION="${BASH_REMATCH[1]}"
TAG_VERSION="${TAG#v}"
CARGO_VERSION="$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["package"]["version"])' "$ROOT/Cargo.toml")"
[[ "$BASE_VERSION" == "$CARGO_VERSION" ]] || {
    printf 'error: tag %s is not based on Cargo version %s\n' "$TAG" "$CARGO_VERSION" >&2
    exit 1
}

SOURCE="$ROOT/docs/release-notes/$BASE_VERSION.md"
[[ -f "$SOURCE" ]] || {
    printf 'error: release notes are missing: %s\n' "$SOURCE" >&2
    exit 1
}

IFS= read -r HEADING < "$SOURCE"
HEADING="${HEADING%$'\r'}"
[[ "$HEADING" == "# v$BASE_VERSION" ]] || {
    printf 'error: expected heading "# v%s" in %s\n' "$BASE_VERSION" "$SOURCE" >&2
    exit 1
}

mkdir -p "$(dirname "$OUTPUT")"
TEMP="$OUTPUT.tmp"
trap 'rm -f "$TEMP"' EXIT
awk -v heading="# v$TAG_VERSION" 'NR == 1 { print heading; next } { print }' "$SOURCE" > "$TEMP"
mv "$TEMP" "$OUTPUT"
