# ADR-013 settings slice: S15 as `core/src/features/settings` (tm `adr013-settings-slice`)

Branch `fleet/adr013-settings-slice`; commits 46bca2df9 (the lift), c27133684 (tests, host write), merge d32324f89 of
`claude/tui-rtwk`, then the fix and pack commits. Gate left to the batch: `check --workspace --tests` clean, rules 113/113.

## What changed

- Core: `core/src/features/settings/{mod,catalog,seed,persist,items,hooks}.rs`, one line in `feature_list!`
  (`settings => Settings`). The screens row, `core/src/screens/settings*` and the orphan `core/src/app/settings.rs`
  (`take_edits` executor, setup follow-up) are gone. State: the facts the host read, the offers, the syntax theme in force, the
  log of requests. A request that is a write of the config file is `cx.effect(Ask::ConfigWrite, Ctx::Write(subject))`;
  `effect_done` follows what was saved or refused (theme back to the config's, `Note::SaveFailed` in the history).
- `core/src/protocol/config.rs`: `config/batchWrite` of the edits and how the answer reads (2 tests).
- Fixtures: the `fact` step (`core/src/fixtures/script.rs` header) reads a feature's fact through the trait's `SEED`
  (`features::seed_fact`); all settings fixtures use it.
- UI half `src/features/settings/` (moved from `src/sheets/settings/`): `open(cx, Args)` by kind, `verb`, `debug_dir`
  (the debug registry's `settings` dir, entries unchanged); `src/pages/session/open.rs` leaves `Target::Settings` to it.
- Barrier-click dismiss of the theme picker restores the preview as Esc does (`dispose_with`, one `SyntaxTheme(None)`).
- Footer of the theme picker: `enter select · esc back` (the oracle gives `standard_popup_hint_line`, which the bottom pane turns
  into the list keymap's hints: `apply_standard_popup_hint`).
- Host: `host/src/settings.rs` (`read`, `write`), `mod settings;`, the settings fact at start and after each write, and
  `Ask::ConfigWrite` answered in the pump (`config_write_job`: app server write, config reloaded, then `ChatEv::Status`,
  `AppEv::Tui`, the settings fact, then the answer). `/theme` sees the config theme, `/memories` opens.

## Gap rows

1. `/config` (and `/tui`) and `/keymap` have no sheet: the offers (`ConfigPanel`, `KeymapPicker`) and their domain tests exist;
   `Screen::build` shows the empty-slot line for them. Needs a follow-up task: the sheets as the oracle draws them.
2. The experimental-features write is not a host ask: `Edit::FeaturesSave` is recorded and marks the dialog saving, but no
   `Ask` carries it (the oracle's `write` is a batch write plus readback); `Fact::FeaturesSaved` has no host sender.
3. A write that lands under a layer that overrides it (`WriteStatus::OkOverridden`) says nothing (oracle: a warning).
4. The skills, hooks and experimental catalogues still arrive only as seeds; no host sender reads them from the server.
5. The pty run beside the oracle was not made (porter rules); `/theme` is the scenario for the batch.

## Key tests

`core::settings` (core/tests/settings.rs: the write cycle `a_saved_theme_or_switches_are_facts_and_a_refused_theme_restores_the_config_s`,
`a_saved_status_line_or_title_changes_no_fact_of_the_feature`, the catalogues), `core::protocol::config`,
`screens::sheets::settings` (tests/sheets/settings.rs), `unit::sheets::settings::draft`, host `settings::tests`, oracle shards
`small_dialogs_1`, `small_dialogs_2` (settings entries), `chat_popups_4` (`/memories`), `rules`.
