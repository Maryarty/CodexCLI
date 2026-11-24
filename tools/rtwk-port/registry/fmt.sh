#!/bin/sh
# rustfmt for the modules that build.rs declares (core/src/features/*, src/features/*,
# key_actions, the transcript's modes, the cells, core/tests): `cargo fmt` follows `mod` lines, not
# the `#[path]` modules of the lists `build.rs` writes into OUT_DIR. Run from anywhere; `--check` checks.
set -eu
root=$(cd "$(dirname "$0")/../../.." && pwd)/codex-rs/tui-rtwk
cd "$root"
find core/src/features src/features src/pages/session/key_actions src/pages/session/transcript \
    src/pages/session/cells core/tests -name '*.rs' -print0 \
    | xargs -0 rustfmt --edition 2024 "$@"
