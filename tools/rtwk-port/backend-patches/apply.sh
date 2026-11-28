#!/bin/bash
# Carry the backend patch series onto a new upstream release.
#
#   apply.sh <upstream-tag-or-commit> [--worktree DIR] [--branch NAME] [--no-generate] [--verbatim]
#
# Catalog: docs/rtwk-port/backend-patches.md. What it does, in order:
#   1. resolves the ref (a missing `rust-v*` tag is fetched from the `upstream` remote) and opens a
#      NEW worktree on a NEW branch (never an existing one, never -B);
#   2. `git am --3way` of every SOURCE patch of series/, in order. A patch that touches only
#      generated files (app-server-protocol/schema, sdk/python/.../generated) is not applied: those
#      files are made by the generators of the new release (step 3). On a conflict it prints the
#      patch, the conflicted files, `git apply --check` of every patch not yet applied against the
#      last good tree, aborts the am and exits 1;
#   3. regenerates the app-server schema (stable + experimental) and the Python SDK types with the
#      release's own `app-server-protocol/scripts/write_schema_fixtures.py` (needs cargo, and uv for
#      the SDK) and commits what changed. `cargo` is the one in PATH: put a wrapper first in PATH to
#      run it under build slots. codex-rs/Cargo.lock is restored after (cargo rewrites the local
#      package versions in it; the port builds against its own lock);
#   4. prints the new tip and the steps for the port (pin move, BACKEND_REF, catalog row).
# --verbatim applies EVERY patch including the generated ones and skips step 3: on the series
# BASE it must give the tree of TIP (= BACKEND_REF), the proof that series/ is complete.
set -u
here=$(cd "$(dirname "$0")" && pwd)
series=$here/series
repo=$(git -C "$here" rev-parse --show-toplevel)

target= wt= branch= generate=1 verbatim=0
while [ $# -gt 0 ]; do
  case "$1" in
    --worktree) wt=$2; shift 2 ;;
    --branch) branch=$2; shift 2 ;;
    --no-generate) generate=0; shift ;;
    --verbatim) verbatim=1; generate=0; shift ;;
    -h | --help) sed -n 2,22p "$0"; exit 0 ;;
    -*) echo "unknown option $1" >&2; exit 2 ;;
    *) [ -z "$target" ] || { echo "one ref only" >&2; exit 2; }; target=$1; shift ;;
  esac
done
[ -n "$target" ] || { echo "usage: apply.sh <upstream-tag-or-commit> [--worktree DIR] [--branch NAME] [--no-generate] [--verbatim]" >&2; exit 2; }

sha=$(git -C "$repo" rev-parse --verify --quiet "$target^{commit}")
if [ -z "$sha" ]; then
  case "$target" in
    rust-v*)
      echo "fetching tag $target from upstream"
      git -C "$repo" fetch upstream "refs/tags/$target:refs/tags/$target" || { echo "cannot fetch $target" >&2; exit 2; }
      sha=$(git -C "$repo" rev-parse --verify "$target^{commit}") || exit 2 ;;
    *) echo "$target is not a commit of $repo" >&2; exit 2 ;;
  esac
fi
name=$(printf '%s' "$target" | tr -c 'A-Za-z0-9.\n' '-')
[ -n "$wt" ] || wt=$HOME/.cache/codexcli/backend-patched-$name
[ -n "$branch" ] || branch=codex/backend-patches-$name
[ ! -e "$wt" ] || { echo "$wt exists; give another --worktree" >&2; exit 2; }
if git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"; then
  echo "branch $branch exists; give another --branch" >&2
  exit 2
fi
git -C "$repo" worktree add -q -b "$branch" "$wt" "$sha" || exit 2
echo "base $target = $sha, worktree $wt, branch $branch"

# generated: every file of the patch is a generated artifact
is_generated() {
  local paths
  paths=$(git -C "$wt" apply --numstat "$1" | cut -f3)
  [ -n "$paths" ] || return 1
  printf '%s\n' "$paths" | grep -v -E '^(codex-rs/app-server-protocol/schema/|sdk/python/src/[^/]+/generated/)' > /dev/null && return 1
  return 0
}

patches=("$series"/*.patch)
applied=() skipped=() failed=
i=0
for p in "${patches[@]}"; do
  pn=$(basename "$p")
  if [ "$verbatim" = 0 ] && is_generated "$p"; then
    skipped+=("$pn")
    i=$((i + 1))
    continue
  fi
  if out=$(git -C "$wt" am --3way "$p" 2>&1); then
    applied+=("$pn")
    i=$((i + 1))
    continue
  fi
  failed=$pn
  echo "---- does not apply: $pn"
  echo "$out"
  echo "conflicted files:"
  git -C "$wt" diff --name-only --diff-filter=U | sed 's/^/  /'
  git -C "$wt" am --abort > /dev/null 2>&1
  echo "---- git apply --check against the last good tree (diagnosis only)"
  for q in "${patches[@]:$i}"; do
    qn=$(basename "$q")
    if [ "$verbatim" = 0 ] && is_generated "$q"; then
      echo "  generated, regenerated instead: $qn"
    elif git -C "$wt" apply --check "$q" > /dev/null 2>&1; then
      echo "  clean:  $qn"
    else
      echo "  FAILS:  $qn"
      git -C "$wt" apply --check "$q" 2>&1 | sed 's/^/          /' | head -8
    fi
  done
  break
done

if [ -n "$failed" ]; then
  echo
  echo "NOT LANDED: $failed and every patch after it. applied before: ${applied[*]:-none}"
  echo "tree left at $(git -C "$wt" rev-parse --short HEAD) on $branch; fix the patch on a backend branch, then export.sh"
  exit 1
fi

gen_status=skipped
if [ "$generate" = 1 ]; then
  gen_status=ok
  schema=$wt/codex-rs/app-server-protocol/scripts/write_schema_fixtures.py
  if [ ! -f "$schema" ]; then
    gen_status="MISSING $schema"
  else
    for mode in --experimental ""; do
      echo "---- write_schema_fixtures.py $mode"
      # shellcheck disable=SC2086
      python3 "$schema" $mode || { gen_status="FAILED ($mode)"; break; }
    done
  fi
  git -C "$wt" checkout -- codex-rs/Cargo.lock 2> /dev/null
  git -C "$wt" add -A codex-rs/app-server-protocol/schema sdk/python 2> /dev/null
  if git -C "$wt" diff --cached --quiet; then
    echo "generators: no change to the checked-in schema/SDK files"
  else
    git -C "$wt" commit -q -m "backend-patches: regenerate app-server schema and SDK types for $target" \
      && echo "generators: changed files committed:" \
      && git -C "$wt" show --stat --format= HEAD | sed 's/^/  /'
  fi
fi

tip=$(git -C "$wt" rev-parse HEAD)
echo
echo "LANDED on $target: applied ${applied[*]:-none}; skipped as generated: ${skipped[*]:-none}; generators: $gen_status"
echo "tip $tip"
if [ "$verbatim" = 1 ]; then
  want=$(cat "$series/TIP")
  if [ "$(git -C "$wt" rev-parse 'HEAD^{tree}')" = "$(git -C "$repo" rev-parse --verify --quiet "$want^{tree}")" ]; then
    echo "verbatim: tree equals TIP $want"
  else
    echo "verbatim: tree DIFFERS from TIP $want"
    exit 1
  fi
elif [ "$gen_status" = ok ] || [ "$gen_status" = skipped ]; then
  cat << NEXT
next, for the port (docs/rtwk-port/README.md "Backend pin"):
  git -C ~/.cache/codexcli/backend-pin worktree add --detach ~/.cache/codexcli/backend-pin-${tip:0:8} $tip   # a new pin directory, no flag day
  point the backend paths of tui-rtwk/{,core/,host/}Cargo.toml at backend-pin-${tip:0:8}
  echo $tip > docs/rtwk-port/BACKEND_REF
  update the BP rows of docs/rtwk-port/backend-patches.md (base, commits, tip), and export.sh $sha $tip when the patch changed
NEXT
fi
[ "$gen_status" = ok ] || [ "$gen_status" = skipped ] || exit 1
