# ADR-013 pilot: /plugins on the new boundary (tm `adr013-pilot-plugins`)

Branch `fleet/adr013-pilot-plugins`; commits e2a568d65 (WIP), 9dcf125c5, c30dd46fe, d878ed566, merge 6236f05bc
of `claude/tui-rtwk`, then the pack commit. Gate: `land-check.sh` on 6236f05bc, ALL fail=0.

## What is green

- Core `core/tests/plugins.rs`: 29 tests (the verbs, a double press collapsing on its `OpKey`, a late reply, a cwd change,
  failed ops, the wish, the setup URL, sections, switches, add/remove/upgrade, the reconnect).
- Tester `tests/pages/plugins.rs`: 7 (the `Back to plugins` row returns to the browser with cursor, tab and query kept; Esc on the
  browser and on the detail closes the sheet, as the oracle does; a tap on "Back" goes back, a following Esc closes).
  `tests/rtwk_o1.rs`: 3 pins of the nested-navigator pattern (see "taught").
- Oracle shard `chat_popups_3` (`summary.py --run chat_popups_3`): 48 pass, 2 n/a, 3 pending, 0 red. The 3 pending are the
  apps and hooks tests of other owners; the rest are `sweep:` lines.
- Rules: 110 (`adr013_*`, `layers`, `hub_enums_only_shrink`). `adr013_ratchets/plugins.txt` lost its four lines (the browser's
  `menu`, `shown`, `tab` draw state and `Intent::Tab`). `hub_budget.txt`: `screens/select.rs` `Note`, `Picked`, `Menu`, `Intent`
  and `app.rs AppEv` each 1 lower (the old `/plugins` variants); the framework's hubs grew by what every feature gets, with the
  reason written beside them: `Out` +1 (`Wish`; the setup URL went on as `Ask::OpenUrl` through `Out::Effect`, `Ask` +1), `app.rs Intent` +2 (`Interest`, `WishShown`), `Feature` +1 (`on_interest`).

## Binary beside the oracle (pty 100x30, `CODEX_HOME` of `mock-model/home.py`, oracle with `--no-daemon -C <work>`)

No `/plugins` scenario exists in `mock-model/scenarios/` and `check.py` only writes `.ansi` captures, so the run is a throwaway
driver (a small VT screen over the pty bytes; keys `/plugins`, Enter, arrows, Esc, typed text); both binaries ran the same key
script and the last frame of each step was compared row by row. The same in both: the browser (header, tab strip, search row, rows,
the selected row's hint, footer), the Installed and other tabs, the search, the detail (title, status line, description, `Back to
plugins`/`Install plugin`, metadata, skills, hooks, apps), the add-marketplace tab and its prompt page (`enter submit · esc back`),
Esc on the browser (closes), Esc on the add page (back to the browser).

Differing rows:
1. Esc: fixed in the fix round, it follows the oracle page by page (reflection 01a209bd). The oracle's popup replaces the
   browser in place, so Esc closes on the detail, the loading and failure pages and the removal's loading and failure; it goes
   back only on the removal's confirmation (`on_cancel` reopens the list). The way back from the detail is its `Back to plugins`
   row (the cursor, tab and query are kept). Still different: the add-marketplace page. The oracle pushes the prompt over the
   browser and the loading/failure pages over the prompt, so Esc there goes back one step; ours has the same stack for the prompt
   (Esc pops to the browser) and closes on its loading/failure pages as the oracle does; the prompt-to-failure step is not driven
   in the binary run.
2. Truncation `…`: when the cut falls after a space the oracle keeps it (`debug iOS apps …`, `proteins, …`), ours drops it
   (`iOS apps…`, `proteins,…`). It is the picker's `TextOverflow::Ellipsis` (`widgets/picker/rows.rs`, rtwk trims before the
   ellipsis), not the plugins' lists; owner dialogs-pane. The shard's own truncation test passes because its cut is inside a word.
3. Footer `GPT-6-Sol default fast` (ours) against `GPT-6-Sol default` (oracle) in the chat under the sheet: the service-tier word,
   not the plugins'; owner chat-status.
4. Row 0 shows the logo art only in ours: inline scrollback in my VT approximation, not a difference of the app. The catalog is
   the curated marketplace's network sync: one rtwk run of five showed `Installed 0 of 0 available plugins` while the sync had not
   finished (the list, not the sheet).

## The template of a feature (for every family task of `adr013-migrate`)

File layout (`plugins` is the example):
- Core, the domain only (no UI word, no widget, no IO): `core/src/features/plugins/mod.rs` (the `Feature` impl with `Intent`,
  `Command`, `Reply`, `Note`, `Wish`, `Interest`, `Fact`; `on_interest`; the fact builder `Fact::config`), `state.rs` (the state:
  `Remote<Catalog>` per cwd, `Remote<Detail>` per `PluginKey`, ops in a map by `OpKey`; the selectors the UI reads: `catalog(cwd)`,
  `tabs`, `detail(key)`, `op(key)`, `op_at(key)`), `wire.rs` (the protocol's request and response to the domain's types),
  `catalog.rs` (the catalog's pure logic: sorting, sections, status labels, tab ids).
- UI half `src/features/plugins/`: `mod.rs` (`impl FeatureView for Plugins`, `Args`, `open(cx, args)`), `sheet.rs`,
  `pages.rs` (the routes of the sheet's navigator, one `StatefulWidget` each), `lists.rs` (what each page shows, as the picker's
  lists), `strings.rs` (all the words). The shared pieces every feature uses: `widgets/interest.rs`, `widgets/sheet.rs`
  (`Sheet`), `widgets/directives.rs`, `app/presenters/mod.rs` (the one presenter).
- The two list files (`core/src/features/mod.rs`, `src/features/mod.rs`, `feature_list!` and the `view_*` macros) are the only
  places that name a feature; everything else asks the list (`features::opens_sheet(command)`, `verb(cx, intent)`).

The verb enum, `plugins::Intent`: verbs with ids, never gestures: `Run{cwd}`, `Read{key}`, `Install{key}`, `Uninstall{key}`,
`SetEnabled{key,enabled}`, `AddMarketplace{cwd,source}`, `RemoveMarketplace{cwd,name}`, `UpgradeMarketplace{cwd,name}`,
`OpenSetupPage{key,app}`, `EndSetup{key,how}`. A widget sends one with `verb(cx, Intent::..)`; the feature decides.

Remote and op shape: what the server answered is `Remote<T>` (`Loading(Option<T>)` keeps the last good answer on show, `Ready`,
`Failed(RpcError)`; the words are the UI's, `strings::failure`), kept per key (`catalog` per cwd, `detail` per `PluginKey`), never "the current X". A change the user asked for is
an `Op{at: Gen, status: Running | Done(Outcome) | Failed(RpcError)}` in a map by `OpKey` (`Install(key)`, `Add{cwd}`, ...): a second press
of the same key finds `Running` and does nothing (the double press collapses); a reply is applied by key and generation, so a late
reply of an older `at` is dropped. A page that waits for an op keeps `awaiting: Option<(OpKey, Gen)>` (the generation before it asked)
and reads the op's status as newer than that, so it never reacts to an op it did not ask for.

The sheet: a modal route (`show_dialog` with `sheet_options`), a `StatefulWidget` with `eq` that is a `Sheet` (it declares
`alive(&Seen)`, here "the feature is on"; `adr013_sheets_declare_alive` checks it) whose body is
`Interest::new(Listing{cwd}, Navigator::new().home(bottom(Home{..})).back_key(None))`. The browser (`Home`) is the first route; detail,
add, remove, upgrade and auth pages are pushed onto the sheet's navigator, each a page with its own `State`. The browser is never
rebuilt, so a page that goes back leaves the cursor, tab and query where they were (the picker's own state is the browser's
`State`; no `TabController` was needed). Esc follows the oracle per page: it closes where the oracle's page has no `on_cancel`
and pops only where the oracle goes back (the removal's confirmation, the add prompt). `open(cx, Args{cwd})` takes ids and
nothing else.

Typed keys: a directory is `Cwd` (`string_id!`), a plugin row `RowKey{market, plugin}` (its picker id is the string form,
`RowKey::parse` reads it back), a plugin is `PluginKey{cwd, row}`, an op `OpKey` over them, a wish's scope `Scope::{Cwd, Thread}`.
No `String` stands for an id in the feature's state, ops, interests or wishes.

Interest and wish: a widget that shows a feature's data wraps itself in `Interest::new(feature::Interest::.., child)`; it is the only
sender of `Intent::Interest`, raising the counter at mount and lowering it at dispose; the feature's `on_interest` loads when the
counter goes 0 to 1, and a reconnect asks again for an interest whose list is known. The sheet asks for nothing itself. A wish
(`wish(scope, Wish::Auth(key))`) is a `SurfaceWish` with a `seq` and a `Scope`; the presenter (`app/presenters`) shows it once
through `FeatureView::shown` and sends `WishShown(seq)`. Delivery has no registry of open sheets: the data of the wish is the
feature's own (`State::setups_in(cwd)`), so a sheet that is open shows the page from its own slice (`Home` pushes the page of a
setup it has not shown, from `did_change_dependencies`); the presenter only decides whether a sheet is open, by the interest
counter core keeps (`Shown.held_by`, the sheet's `Interest`, with the wish's scope inside it): none held, it opens
`Shown.open`, a sheet that then shows the setup like the open one. This replaces the `Door`/`RefCell` registry; its race (a
wish selected by a sheet while `WishShown` removes it in the same flush) cannot happen because the sheet does not read the
wish queue.

Tests split: (1) core `core/tests/plugins.rs`, no UI: each verb, a double press, a late reply, a cwd change, a failed op, a wish, the
reconnect; (2) Tester `tests/pages/plugins.rs`: the route model (Esc per page, a tap, the cursor kept); (3) the oracle shard
(`tests/oracle/<shard>.rs` and its fixtures): the oracle's own screens through the real app; (4) one probe per rtwk finding
(`tests/rtwk_o1.rs`); (5) rules: the family's ratchet file (`tests/rules/adr013_ratchets/<family>.txt`) and `hub_budget.txt` shrink.

## What the pilot taught (for the next families)

The boundary holds when the UI half keeps no copy of the feature's data: every page selects `FeatureStates` and the feature's
selectors and sends a verb, so the old sheet's `Menu`/`Want`/`Pending` hub state went away and what the sheet holds is only what is
drawn (the cursor, the tab, `awaiting`). What cost the time: (a) the rules make the list the only place that names a feature, so
everything a feature needs from the UI (open a sheet, show a wish, a fact built in the host) must be a generic hook (`FeatureView`,
`Fact::config`) and not a call that names `features::<f>`; add the hooks first; (b) a nested `Navigator` in a sheet: a `Navigator` is an
`Overlay`, so it takes the largest size it is given, and a `LayoutBuilder` rightly refuses intrinsic queries (Flutter's rule too).
The pattern is a navigator as tall as the sheet's cap with its pages aligned at the bottom (`Align(BOTTOM_CENTER)`,
`pages::bottom`), pinned by `tests/rtwk_o1.rs`; it is how the oracle's bottom pane sits, not a workaround; (c) `Remote::Loading(Some(old))` matters for lists: a refresh
behind an op must not blank the page; (d) the tab ids of a placeholder list and of the loaded list differ (`tab_matching`, `tab_now`), so
a surface that restores a tab by id needs a resolver, not equality; (e) reconnect: `attach` fails the in-flight asks (F6, landed), so a reconnect asks again for every interest held; (f) R1 bans `.follow(`, `.channel(`, `.subscribe(`,
`cx.spawn(` outside the shells, so a feature's op tracking is a private `track`/`await_op` over the app's step, not a spawn.

## Gap rows and Needs

- Gap: `ScopeChanged` (the oracle closes the popup when the cwd changes) is not carried: the sheet's `alive` has no scope input yet.
- Gap: a failed list refresh replaces the shown catalog by the error (`Remote::Failed`); the stale catalog is not kept on show.
- Gap: wishes do not expire by scope (§3): a wish queued for a directory stays until it is shown; a cwd change does not drop it.
- Gap: two wishes made in one flush, with no sheet open, may open two sheets (the presenter reads the interest counter once per
  wish before the first sheet has mounted).
- Closed by the pilot: the "no OpenUrl door" gap; the setup URL is `Ask::OpenUrl` through `Out::Effect` (precedent
  `Ask::OpenDesktop`), its failure a `Say::PageFailed` from `Feature::effect_done`.
- Need (adr013-features-trait): the SHEET slash-command table becomes `Target::Feature` with typed `OpenArgs`, so
  `pages/session/actions.rs` no longer builds a `Scope` for it.
- Touches outside the area of the fix, one line each: `src/pages/session/actions.rs` (the `open_sheet` call takes a `Scope`),
  `src/app/debug/seeded.rs` (the `Door` provider is gone), `core/tests/app_wiring.rs` (`Done::Desktop` is `Done::Opened`).
- Need (adr013-features-trait): `Feature::Menu/Pick/Prompt/choose/picked/prompt_submitted` stay for the other families (their lines
  remain in `adr013_ratchets/features.txt`); the `FeatureView` hooks (`SHEET`, `open_sheet`, `shown`, `menu_list`, `prompt_words`) are
  what that task generalises.
- Need (mock-model): a `/plugins` scenario and a grid renderer for `.ansi` captures (the throwaway driver of this run is not committed).

