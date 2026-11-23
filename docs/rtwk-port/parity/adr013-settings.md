# adr013-settings — parity

Shard counts: oracle small_dialogs_1, small_dialogs_2, picker, chat_popups_4: no red before or after (the gate's 3212 tests, 0 failed); ratchet file `adr013_ratchets/settings.txt` deleted.
Decisions: settings is migrated in place (Stage 1), not yet a feature slice: `core/src/features/mod.rs`,
  `screens/mod.rs` (`screen_list!`), `actions.rs`/`open.rs` are held by other tasks (Needs below). No
  `AppCtx::Settings`; the write goes as `Intent::Request(Edit)` into the model, the app takes it with
  `take_edits()` (executor sketch in the orphan `core/src/app/settings.rs`, to become `Ask::ConfigWrite`).
  UI state (cursor, draft, preview, open sheet) is in `src/sheets/settings/{draft,keys,...}.rs`; keys go through
  the keymap (`LIST_*`, ADR-006), no `KeyCode::`. Gap rows added: none (the differences below are Needs).
Binary: scenario settings-theme (scratch: a turn, `/theme`, two Downs; not committed), both binaries, 100x30,
  checkpoints turn-2 (picker open) and turn-3 (cursor moved, preview follows): picker rows EQUAL after two fixes
  found by this run (the "(current)" row and initial cursor on the adaptive default theme when the config names
  none; the preview panel centred below the panel's padding row, as the live oracle has it). Differing rows left:
  - footer hint: ours `Press enter to confirm or esc to go back`, live oracle `enter select · esc back` (the
    oracle swaps the standard popup hint for the keymap hint in `apply_standard_popup_hint`; its unit snapshots,
    which the shard tests match, show the standard one). Need 9.
  - not this family: the session path in the header, the random tip row (+ its blank row) at the top of the
    chat, `· Mock scenario` in the status line (the oracle titles the thread), two blank rows above the popup.
Mouse: the theme picker rows and the setup rows take taps (`GestureDetector`), tests in `tests/sheets/settings.rs`.
Rows for the lander / Needs for others:
  1. lift to `core/src/features/settings` (no `AppCtx::Settings`, write via `cx.effect(Ask::ConfigWrite)`);
  2. remove the `settings => Settings` row of `screen_list!` (`screens/mod.rs`, resume);
  3. page-side `open(cx, kind)` (`actions.rs` pilot-plugins, `open.rs` dialogs-pets);
  4. `core/src/app/settings.rs` is an undeclared orphan: executor of `take_edits`, status-line/title setup from
     chat facts, chat follow-up after a saved status line/title;
  5. apply the host feed: `host/src/settings.rs` is committed (facts of the config: `[tui] theme`, `.tmTheme`
     stems of `{codex_home}/themes`, `~` display of that directory, memory switches only with
     `Feature::MemoryTool`; 2 unit tests); the wiring (`mod settings;` in `host/src/main.rs`, one `emit` of
     `SettingsEv::Facts` next to `AppEv::Tui` in `host/src/in_process.rs`) clashed with adr013-pilot-plugins'
     area: patch `side-session-notes/local_bbf75807-…/adr013-settings-host-wiring.patch` (`git apply`; built and
     tested with it). Until then `/theme` has no config theme and `/memories` opens nothing in the binary
     (as before this task: the real host never sent facts);
  6. a fixture step for feature facts (experimental features sheet);
  7. Config and Keymap offers have no sheet (old `Activate`/`KeymapPick` dropped);
  8. barrier-click dismissal of the theme picker does not restore the preview theme (as before);
  9. live-oracle keymap footer hint versus the snapshot-matched standard hint (above).
