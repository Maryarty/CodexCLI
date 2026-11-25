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
echo "$files" | tr '\n' '\0' | xargs -0 rustfmt --edition 2024 "$@" 2>&1 | grep -v "^Warning: can't set" || true
[ "${1:-}" = --check ] && { echo "$files" | tr '\n' '\0' | xargs -0 rustfmt --edition 2024 --check >/dev/null 2>&1 || exit 1; }
exit 0
