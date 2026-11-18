#!/bin/bash
# Regenerates gallery-codes.jsonl: for each of the pin's gallery pages, its inspector
# options and the Code-tab Rust at the initial options. Needs cargo (a build of the
# gallery's modules, a few minutes when cold); run only when the pin changes.
#   tools/rtwk-port/skill/refresh-codes.sh [PIN_DIR]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
pin=${1:-${RTWK_PIN:-$HOME/.cache/codexcli/rtwk-pin}}
work=${TMPDIR:-/tmp}/rtwk-skill-dump-codes
mkdir -p "$work/src"
sed "s|@PIN@|$pin|g" "$here/dump-codes/Cargo.toml.in" > "$work/Cargo.toml"
cp "$pin/Cargo.lock" "$work/Cargo.lock"
cp "$pin/rust-toolchain.toml" "$work/rust-toolchain.toml"
: > "$work/mods.rs"
for m in gallery pages reference_catalog reference_code reference_feedback reference_inspector \
         reference_lit reference_motion reference_paint reference_palette reference_room \
         reference_split reference_stage reference_tokens route_replay; do
  if [ "$m" = pages ]; then p="$pin/crates/rtwk-gallery/src/pages/mod.rs"; else p="$pin/crates/rtwk-gallery/src/$m.rs"; fi
  printf '#[path = "%s"]\nmod %s;\n' "$p" "$m" >> "$work/mods.rs"
done
sed -e "/@MODS@/{r $work/mods.rs" -e 'd;}' "$here/dump-codes/main.rs.in" > "$work/src/main.rs"
cd "$work"
~/.cache/rtwk/acargo run -q 2>"$work/build.log" >"$work/out.jsonl" || { tail -40 "$work/build.log"; exit 1; }
{
  head=$(git -C "$pin" rev-parse --short=8 HEAD 2>/dev/null || echo unknown)
  inputs=$(python3 -I "$here/generate.py" --print-inputs-hash --pin "$pin")
  printf '{"pin":"%s","inputs":"%s","generator":"tools/rtwk-port/skill/refresh-codes.sh"}\n' "$head" "$inputs"
  cat "$work/out.jsonl"
} > "$here/gallery-codes.jsonl"
wc -l "$here/gallery-codes.jsonl"
