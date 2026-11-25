#!/bin/sh
# fmt-all.sh [--check]: every rustfmt tui-rtwk needs, in ONE call (person 2026-11-25, ceremony cut 8):
#  1. `cargo fmt` (through acargo) for the crates' mod trees;
#  2. registry/fmt.sh for the modules build.rs declares (#[path] lists in OUT_DIR, cargo fmt skips them);
#  3. rustfmt for the test files (macro-declared test modules cargo fmt skips).
# Run from anywhere. `--check` checks only (exit 1 on a diff).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)/codex-rs/tui-rtwk
cd "$root"
rc=0
"$HOME/.cache/rtwk/acargo" fmt -- "$@" || rc=1
sh "$here/registry/fmt.sh" "$@" || rc=1
find tests -name '*.rs' -not -path '*/fixtures/*' -not -path '*/snapshots/*' -not -path '*/oracle_snap/*' -print0 \
    | xargs -0 rustfmt --edition 2024 "$@" || rc=1
exit $rc
