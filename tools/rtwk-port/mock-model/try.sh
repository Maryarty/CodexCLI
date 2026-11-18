#!/usr/bin/env bash
# try.sh <scenario> rtwk|oracle : serve a scenario, write a temp CODEX_HOME,
# run the binary in this terminal. Ctrl+C in the binary quits; the server and
# the temp dir are cleaned up. Never touches ~/.codex (CODEX_HOME is a temp dir).
#
#   RTWK_BIN / ORACLE_BIN   binary override (else bins.py looks for a built one)
#   KEEP=1                  keep the temp dir
#   ./try.sh --list         list the scenarios
#   ./try.sh router rtwk    one session; the word you type picks the response type
#                           (exec, patch, plan, image, web, mcp, reasoning, ..., all)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY=(python3 -I)

if [[ "${1:-}" == "--list" || $# -ne 2 ]]; then
  "${PY[@]}" "$HERE/serve.py" --list
  [[ "${1:-}" == "--list" ]] || { echo "usage: try.sh <scenario> rtwk|oracle" >&2; exit 2; }
  exit 0
fi
SCENARIO="$1"; WHICH="$2"
case "$WHICH" in rtwk|oracle) ;; *) echo "second argument: rtwk or oracle" >&2; exit 2;; esac
BIN="$("${PY[@]}" "$HERE/bins.py" "$WHICH")" || exit 1

ROOT="$(mktemp -d -t rtwk-mock-model)"
URLFILE="$ROOT/url"
"${PY[@]}" "$HERE/serve.py" "$SCENARIO" --loop --url-file "$URLFILE" >"$ROOT/serve.out" 2>"$ROOT/serve.err" &
SERVER=$!
cleanup() {
  kill "$SERVER" 2>/dev/null || true
  wait "$SERVER" 2>/dev/null || true
  if [[ -n "${KEEP:-}" ]]; then echo "kept: $ROOT (server log: $ROOT/serve.err)"; else rm -rf "$ROOT"; fi
}
trap cleanup EXIT
for _ in $(seq 1 100); do [[ -s "$URLFILE" ]] && break; sleep 0.1; done
[[ -s "$URLFILE" ]] || { echo "the mock server did not start:" >&2; cat "$ROOT/serve.err" >&2; exit 1; }
URL="$(cat "$URLFILE")"
{ read -r CODEX_HOME_DIR; read -r WORKDIR; } < <("${PY[@]}" "$HERE/home.py" "$SCENARIO" "$URL" "$ROOT")

echo "mock: $URL   scenario: $SCENARIO   binary: $BIN" >&2
if [[ "$SCENARIO" == router ]]; then echo "type:" >&2; else echo "type, in order:" >&2; fi
"${PY[@]}" "$HERE/serve.py" "$SCENARIO" --prompts >&2
echo "(Ctrl+C to quit; mock requests are logged in $ROOT/serve.err)" >&2
cd "$WORKDIR"
set +e
case "$WHICH" in
  oracle) env CODEX_HOME="$CODEX_HOME_DIR" OPENAI_API_KEY=dummy "$BIN" -c analytics.enabled=false -C "$WORKDIR" ;;
  rtwk)   env CODEX_HOME="$CODEX_HOME_DIR" OPENAI_API_KEY=dummy "$BIN" ;;
esac
STATUS=$?
set -e
exit $STATUS
