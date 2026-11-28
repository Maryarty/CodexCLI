# tui-rtwk — the Codex TUI on rtwk

`codex-rs/tui` (ratatui) carried one to one onto rtwk
(`/Users/admin/Documents/dev_projects/rtwk`) as `codex-rs/tui-rtwk`. The plan
and its state live in tm under the root task `tui-rtwk`; this directory holds
the design and the references agents read.

| file | what |
|---|---|
| [architecture.md](architecture.md) | decisions (ADR-001…009), skeleton, rtwk gaps, risks, open decisions |
| [ORACLE_REF](ORACLE_REF) | the upstream commit whose `codex-rs/tui` is the oracle |
| [RTWK_REF](RTWK_REF) | the rtwk commit the port builds against |
| [BACKEND_REF](BACKEND_REF) | the commit of this repository whose `codex-rs` backend crates the port builds against |
| [backend-patches.md](backend-patches.md) | the catalog of the patches the backend carries (BACKEND_REF is the tip of their series) and how to move them to a new release |
| [research/codex-tui-surface.md](research/codex-tui-surface.md) | screens, flows and terminal behaviours of the oracle |
| [research/codex-tui-features.csv](research/codex-tui-features.csv) | 312 rows: every view, popup, overlay, cell, slash command, keymap group, terminal capability |
| [research/codex-tui-architecture.md](research/codex-tui-architecture.md) | the oracle's runtime, state, rendering, reuse, tests |
| [research/rtwk-for-codex.md](research/rtwk-for-codex.md) | rtwk map, capability matrix, how to find API, proposed references |
| [research/state-management.md](research/state-management.md) | Rust state-management options compared for this port |
| `refs/` | generated references (ADR-008), written by `tools/rtwk-port/` |

Worktrees: `codex-rs/tui-rtwk` builds against the rtwk pin, a checkout of the
commit in [RTWK_REF](RTWK_REF) at `/Users/admin/.cache/codexcli/rtwk-pin-5001` (rtwk main; a new pin is a new
directory `rtwk-pin-<short sha>`, the old one is dropped when the last worktree has moved). Its
`Cargo.toml` names the pin's crates by that absolute path: a relative path
through a symlink makes cargo read the outer `codex-rs` workspace root and
fail on `rust-version` inheritance. `tui-rtwk/tests/pin.rs` checks that the
checkout is at RTWK_REF. A new pin is a new commit in RTWK_REF, in this
crate's manifest paths and in the checkout, together.

Backend pin: the codex backend crates (`codex-core`, `codex-protocol`,
`codex-app-server-*` and the rest) are path dependencies on one fixed checkout,
`/Users/admin/.cache/codexcli/backend-pin-e8ddca3f` (a detached worktree of this
repository at the commit in [BACKEND_REF](BACKEND_REF); the directory carries the first 8 digits
of the ref, as the rtwk pin does, so a new backend is a new directory and no flag day), not on `../..` of each
worktree. Every worktree then names the same backend packages at the same absolute
paths, so sccache (in `acargo`) serves the backend to each worktree's own
target (`codex-rs/tui-rtwk/target`). One target shared by all worktrees was
tried and dropped (2026-11-21): cargo names workspace units by their path
relative to the workspace, so worktrees overwrote each other's test binaries. `tests/rules/pin.rs` checks that
all three manifests point into the pin and the pin is at BACKEND_REF. The port
edits the backend only through the catalogued patches ([backend-patches.md](backend-patches.md),
person 2026-11-28): BACKEND_REF is the tip of the series in
`tools/rtwk-port/backend-patches/series` (`tests/rules/backend_patches.rs`). After an upstream
sync touches `codex-rs` outside `tui-rtwk`, or to a new release,
`tools/rtwk-port/backend-patches/apply.sh <tag>` carries the series onto it; then move the pin
and the file together:
`git -C ~/.cache/codexcli/backend-pin worktree add --detach ~/.cache/codexcli/backend-pin-<ref[..8]> <ref>`, the manifest paths and the
full commit in BACKEND_REF. Nothing builds a manifest against a pin of another worktree: a
temporary manifest overlay (paths into a staging checkout) is a local experiment, never committed.
