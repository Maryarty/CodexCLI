# chunk-popups-plugins — parity

Code (view side; core's `Plugins` model, catalog and wants were core-pass's):
- `src/sheets/select/plugins.rs`: the pages of the plugin model as picker lists: the browser (tabs with
  counts, installed-then-name rows, `[x]`/`[-]` marks, status column padded to the tab's widest, the tab's
  header and note, the hint line by the user's keymap, Ctrl-R/Ctrl-U hot keys on a marketplace tab), the
  detail (source, install state, install/uninstall/back rows, metadata rows), the loading/failed/confirm
  pages. `plugins/strings.rs`: the words (ADR-010).
- `src/sheets/select/prompt.rs`: the marketplace source prompt (shared with `/review`'s instructions).
- `src/sheets/select/dialog.rs`: routes `Menu::Plugins`; `Event::Tab/Hot` become `plugins::Intent`.
- `src/widgets/picker`: `Extras` (toggle stroke, hot keys, per-tab notes, reserved rows, kept position),
  `Flags.marked` + `Act::Nothing` (Space on a marked row with an empty query does not start a search),
  the tab the user is at survives list changes (`follow_tabs`, `tab_matching` for a section that loaded).
- `src/pages/session/actions.rs`: `/plugins` becomes `Open{target: Select}` + `OpenPlugins{cwd}`.

Tests: all 38 plugin oracle tests are ported and green (`chat_popups_2` NO_PLUGINS 4, `chat_popups_3`
PLUGINS 34); `tests/oracle/fixtures/chat_popups_3/seed.rs` holds the server seeds (plugin/list, plugin/read,
installs, marketplace calls). Mouse (`fixtures/chat_popups_3/mouse.rs`, 10): tap on a row opens the detail,
double tap on a row opens one detail and asks no install, tap on a tab / on `Add Marketplace` and its row,
the wheel over a 40-plugin list then a tap on a row it brought in, tap on `Install plugin` (a double tap asks
the install once), tap on `Back to plugins`, tap on `Remove marketplace` (a double tap removes once) and on
`Back to plugins` of the confirmation, tap on a remote row asks its detail from the remote marketplace.
Shards: chat_popups_3 45 pass / 2 n/a / 6 pending (APPS, HOOKS: not this task) / 0 red; chat_popups_2
13 pass / 6 n/a / 27 pending (permissions, windows sandbox: not this task). rules 66, clippy clean,
`shards.py --check` clean, screens+pages+shell 991 passed.

Decisions:
- The picker taps pick at once (the one-tap convention of `tests/pages/review.rs`); the second tap of a
  double tap lands on the page the first one opened, and no row there acts twice (tested).
- The oracle's centered removal confirmation floats over its composer at the pane's top; the port's
  `Dialog::bare` is anchored over the real composer. The test compares the dialog's columns (same as the
  `NO_FULL_ACCESS` note of chat_popups_2).
- The status column pads with the unicode width of the widest label of the tab (`width_of`, R15), not
  `chars().count()`.

Binary: own pty driver beside the oracle (`/tmp/plugins-drive`, throwaway; a repo with a local marketplace
`Local Demo` of 3 plugins; 100x36; rtwk `codex-rtwk` vs oracle `codex --no-daemon`; both go to the real app
server, which also syncs the 54-plugin OpenAI curated marketplace). Steps: `/plugins`, Down, tabs left/right,
search `dri`, detail of a row, install (loading, result, history line), detail again (installed), uninstall
(loading, history line), Add marketplace tab and its prompt, add of a bad source (loading, failure) and
`Try again`, Esc closes. The text of every step is EQUAL except the rows below. land-check: scenarios 39/41
(the two known reds, error-context-window and multi-agent), nextest 2986 passed.

Rows that differ from the oracle:
1. Esc on the plugin detail: fixed (adr013-pilot-fix). Esc follows the oracle per page: it closes `/plugins` on the
   detail, loading and failure pages (their way back is the `Back to plugins` row) and goes back only on the
   removal's confirmation and the add prompt. The old text (ours returned to the list, core `Plugins::back()`) no
   longer applies; the core `back()` Need is closed.
2. A one-line cell cut just after a space: the oracle prints `apps …`, ours `apps…` (rtwk's
   `TextOverflow::Ellipsis` drops the blank before the cut): `G-new-ellipsis-1` in rtwk-gaps.md.
3. Ctrl-U (and the other readline chords) in a picker search edit the query (rtwk `TextField`); the oracle's
   search line takes characters and Backspace only (on a marketplace tab Ctrl-U is upgrade, on All it does
   nothing). Picker-wide, not plugin-specific; left as is.
4. The fresh-marketplace note (`Debug Marketplace installed successfully.`) stays until a tab change; the
   oracle drops it after the first refresh of the list (core `fresh`).
5. Hyperlink style of the detail's `Learn more` (Role::Info, no underline), the loading header has no
   shimmer (dim): rtwk `Role::Link` / shimmer wanted.

Needs (core; held by core-slices-step0):
- `commands/dispatch.rs` (~502): `open(Target::Select, ..)` for `Plugins` (the view remaps
  `Outcome::Session{command: Plugins}` in `actions.rs` meanwhile).
- `picks()` in `core/src/app/select.rs`: `Picked::Open(url)` -> `Command::OpenUrl`.
- Refresh the plugin mentions after `Plugins::take_written()`.
- `Plugins::back()` per row 1; `fresh` per row 4; `Row.installed` (the view reads `catalog.row`).
