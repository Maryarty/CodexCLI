#!/bin/sh
# fmt-all.sh [--check]: every rustfmt a tui-rtwk branch needs, in ONE call (person 2026-11-25, ceremony cut 8).
# Formats exactly the .rs files under codex-rs/tui-rtwk that THIS checkout changed against the tip
# (claude/tui-rtwk): committed on the branch, staged, unstaged or untracked. That covers what `cargo fmt`
# misses (build.rs-declared #[path] modules, macro-declared test modules) and never reformats files the
# branch did not touch (the tip is not rustfmt-clean everywhere: a whole-tree run would conflict).
# Runs on the checkout of the CURRENT directory. `--check` checks only (exit 1 on a diff).
set -eu
top=$(git rev-parse --show-toplevel)
cd "$top"
base=$(git merge-base HEAD claude/tui-rtwk)
files=$( { git diff --name-only --diff-filter=d "$base" -- codex-rs/tui-rtwk; \
           git ls-files --others --exclude-standard -- codex-rs/tui-rtwk; } \
         | grep '\.rs$' | grep -v -e '/fixtures/' -e '/snapshots/' | sort -u)
[ -n "$files" ] || { echo "fmt-all: no changed .rs files"; exit 0; }
# One file at a time through stdin: rustfmt given a PATH also formats every out-of-line `mod x;` child
# of it (a changed lib.rs / mod.rs reformatted untouched files); from stdin it formats that file only.
rc=0; tmp=$(mktemp)
for f in $files; do
  [ -f "$f" ] || continue
  if ! rustfmt --edition 2024 --config-path codex-rs/tui-rtwk/rustfmt.toml --emit stdout < "$f" > "$tmp" 2>/dev/null; then echo "fmt-all: rustfmt failed on $f"; rc=1; continue; fi
  cmp -s "$f" "$tmp" && continue
  if [ "${1:-}" = --check ]; then echo "fmt-all: not formatted: $f"; rc=1; else cat "$tmp" > "$f"; fi
done
rm -f "$tmp"
exit $rc
