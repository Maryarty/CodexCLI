# backend-patches

The patches the backend carries (docs/rtwk-port/backend-patches.md is the catalog).

- `series/*.patch`: `git format-patch` of the patch branch from `series/BASE` to `series/TIP`
  (`TIP` is `docs/rtwk-port/BACKEND_REF`; `tests/rules/backend_patches.rs` checks it).
- `apply.sh <upstream-tag-or-commit>`: new worktree + branch on the release, `git am --3way` of
  the source patches (patches that touch only generated schema/SDK files are skipped), the
  release's `write_schema_fixtures.py` (stable and experimental) and the SDK generator, one
  commit of what changed; prints what did not land. `--verbatim` on `series/BASE` applies all
  patches and compares the tree with `TIP`.
- `export.sh <base> <tip>`: rewrites `series/` from a patched branch.

Run from anywhere in the repository; both scripts need git only (the generators need cargo and uv).
