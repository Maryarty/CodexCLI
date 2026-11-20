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
| [research/codex-tui-surface.md](research/codex-tui-surface.md) | screens, flows and terminal behaviours of the oracle |
| [research/codex-tui-features.csv](research/codex-tui-features.csv) | 312 rows: every view, popup, overlay, cell, slash command, keymap group, terminal capability |
| [research/codex-tui-architecture.md](research/codex-tui-architecture.md) | the oracle's runtime, state, rendering, reuse, tests |
| [research/rtwk-for-codex.md](research/rtwk-for-codex.md) | rtwk map, capability matrix, how to find API, proposed references |
| [research/state-management.md](research/state-management.md) | Rust state-management options compared for this port |
| `refs/` | generated references (ADR-008), written by `tools/rtwk-port/` |

Worktrees: `codex-rs/tui-rtwk` builds against the rtwk pin, a checkout of the
commit in [RTWK_REF](RTWK_REF) at `/Users/admin/.cache/codexcli/rtwk-chat-pin`. Its
`Cargo.toml` names the pin's crates by that absolute path: a relative path
through a symlink makes cargo read the outer `codex-rs` workspace root and
fail on `rust-version` inheritance. `tui-rtwk/tests/pin.rs` checks that the
checkout is at RTWK_REF. A new pin is a new commit in RTWK_REF, in this
crate's manifest paths and in the checkout, together.
