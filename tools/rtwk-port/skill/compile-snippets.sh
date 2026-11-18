#!/bin/bash
# Compiles and runs the recipe snippets (snippets/*.rs, each with its test) in a scratch crate
# under $TMPDIR against the pinned rtwk. Nothing is written into the repository.
#   tools/rtwk-port/skill/compile-snippets.sh [PIN_DIR]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
pin=${1:-${RTWK_PIN:-$HOME/.cache/codexcli/rtwk-pin}}
work=${TMPDIR:-/tmp}/rtwk-skill-recipes
mkdir -p "$work/src"
cat > "$work/Cargo.toml" <<EOF
[package]
name = "recipes"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
rtwk = { path = "$pin/crates/rtwk" }
rtwk-tokio = { path = "$pin/crates/rtwk-tokio" }
tokio = { version = "1", features = ["rt-multi-thread", "time", "sync"] }
futures-core = "0.3"

[dev-dependencies]
rtwk-test = { path = "$pin/crates/rtwk-test" }

[workspace]
EOF
cp "$pin/Cargo.lock" "$work/Cargo.lock"
cp "$pin/rust-toolchain.toml" "$work/rust-toolchain.toml"
{
  echo '#![allow(dead_code)]'
  for f in "$here"/snippets/*.rs; do
    n=$(basename "$f" .rs)
    printf '#[path = "%s"]\nmod %s;\n' "$f" "$n"
  done
} > "$work/src/lib.rs"
cd "$work"
exec ~/.cache/rtwk/acargo test -q -- --test-threads=1
