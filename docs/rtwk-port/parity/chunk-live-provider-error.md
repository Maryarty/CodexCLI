# chunk-live-provider-error — parity

Code: `src/pages/session/live/provider_error.rs` (the notice: lock, title and words in a `BoxWidget` on the
prompt surface; the hints under it as a `Wrap` of `GestureDetector` taps; `Lock::of`; the keys),
`session_page.rs` (`ChatMsg::Provider{Continue,New,Others}`, the branch before `read_only()`, Esc),
`strings.rs` (ADR-010 words and the provider-name table), `tests/oracle_snap/mod.rs` (the kit reads a bare
snake_case line as a frame header: the oracle's `provider_errors.snap` frames are bare labels). Core was core-pass's
(`thread.provider_error()`, `SessionIntent::ContinueChat`).

Keys: `c` = `ContinueChat` (words "Read {name} and let’s continue where we left off.\n\nSaved conversation: {path}",
only when the thread has a rollout path), `n` = `New`, Esc = the chats to resume (a second Esc comes back to the
notice, the draft is still in the composer controller). Typing, a paste and Enter send nothing. The hints are tap targets
(beyond the oracle, ADR-011).

Shard counts (summary.py --run chat_live_1): pass 21 / n/a 5 / pending 30. `provider_error_preserves_draft_and_survives_thread_navigation`
stays `kit::pending`: probed with KIT_PROBE_PENDING=1, frames `provider_error_local_80`, `policy_provider_local_80`,
`policy_settings_local_80` and the Esc-Esc survival are green. The three mismatch frames are not:
- `provider_error_mismatch_local_80` / `_local_40`: the oracle names the provider by the config's display name
  (`Team Gateway`, a `model_providers` entry with id `custom-gateway`); ours has only the id, so the words and the wrap differ.
- `provider_error_mismatch_remote_80`: the oracle's `Configuration on {address}` row names the remote the launch connected to; ours has no row.
Everything else in those frames (lock row, title `Continue in a new chat`, 40-column wrap of the footer) is equal.

Needs (not in this area):
1. core: a typed projection of `ModelProviderError` on the thread (`Lock::of` reads the wire JSON now; NEED(core) comment).
2. core + host (`core/src/screens/chat.rs` `StatusFacts`, `host/src/facts.rs`): the config's `model_providers` display names and
   the launch's remote address with the local-daemon flag; then `Lock` takes the name and `notice(.., location)` gets the address, and the
   pending marker goes.
3. `tests/oracle/transcript_view_1.rs::provider_error_escape_opens_resume_picker` only types a draft and Esc: portable now.

Tests: `tests/pages/chat_provider_error.rs` (3): `c` starts a new chat whose composer holds the continue words, `n` starts a fresh
chat, and each hint is a tap target (continue, start fresh, other chats).

Binary: the notice cannot be driven in the real binary, neither rtwk nor the oracle: nothing in this repo's server
(app-server, core) produces `modelProviderMismatch` / `NotConfigured` / `PolicyConflict`; only the protocol type and the oracle's UI exist, and
the mock scenarios have no such answer. land-check's mock scenarios on the merged tip: 39/41, the two known reds (error-context-window, multi-agent).
No rows to list from a side-by-side.

Rows for the lander: none (no shard file changed; `shards.py --check` clean).
