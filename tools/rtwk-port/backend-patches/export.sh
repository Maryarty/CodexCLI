#!/bin/bash
# Rewrite series/ from a patched backend branch: export.sh <base> <tip>
#
#   export.sh 97400b8a7da207a59f2fce7bcb320a6a2b4d4dc1 e8ddca3f2163186149ab1e79e132ce12f7e9e6b0
#
# `git format-patch base..tip` (full index and binary diffs so `git am --3way` works, no commit
# ids in the headers so a re-export of the same commits is byte-identical), then BASE and TIP.
# Run it after the patch changed on its backend branch, then edit docs/rtwk-port/backend-patches.md.
set -eu
[ $# -eq 2 ] || { echo "usage: export.sh <base> <tip>" >&2; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
base=$(git -C "$repo" rev-parse --verify "$1^{commit}")
tip=$(git -C "$repo" rev-parse --verify "$2^{commit}")
git -C "$repo" merge-base --is-ancestor "$base" "$tip" || { echo "$base is not an ancestor of $tip" >&2; exit 2; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git -C "$repo" format-patch -N --full-index --binary --no-stat --no-signature --zero-commit -o "$tmp" "$base..$tip" > /dev/null
rm -f "$here"/series/*.patch
mkdir -p "$here/series"
cp "$tmp"/*.patch "$here/series/"
printf '%s\n' "$base" > "$here/series/BASE"
printf '%s\n' "$tip" > "$here/series/TIP"
ls "$here/series"
