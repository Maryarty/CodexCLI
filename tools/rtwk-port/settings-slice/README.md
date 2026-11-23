# settings slice: scripts of the move of S15 onto `core/src/features/settings`

Run from `codex-rs/tui-rtwk`; each is rerunnable (a second run changes nothing).

- `split_core.py`: cuts the dialog types of the old `core/src/screens/settings.rs` into
  `core/src/features/settings/catalog.rs` and the seed reader into `seed.rs`.
- `fixtures.py`: rewrites the `screen`/`dir: settings` lines of the `.jsonl` fixtures (and the
  chat_popups_4 `memories_enabled` one) to `{"step":"fact","feature":"settings","fact":..}`.
- `ui_paths.py`: the UI half's moved files (`src/sheets/settings` -> `src/features/settings`):
  module paths, `FeatureStates` instead of the screens' model.
- `core_test.py`: the mechanical part of the port of `core/tests/settings.rs` (paths, names).
