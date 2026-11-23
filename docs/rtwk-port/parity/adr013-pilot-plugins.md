# ADR-013 pilot: /plugins on the new boundary (tm `adr013-pilot-plugins`)

Branch `fleet/adr013-pilot-plugins`; commits e2a568d65 (WIP), 9dcf125c5, c30dd46fe, d878ed566, merge 6236f05bc
of `claude/tui-rtwk`, then the pack commit. Gate: `land-check.sh` on 6236f05bc, ALL fail=0.

## What is green

- Core `core/tests/plugins.rs`: 29 tests (the verbs, a double press collapsing on its `OpKey`, a late reply, a cwd change,
  failed ops, the wish, the setup URL, sections, switches, add/remove/upgrade, the reconnect).
- Tester `tests/pages/plugins.rs`: 6 (Esc on the detail returns to the browser with cursor, tab and query kept; Esc on the
  browser closes the sheet; a tap on "Back" is Esc). `tests/rtwk_o1.rs`: 3 probes of the nested navigator (see "taught").
- Oracle shard `chat_popups_3` (`summary.py --run chat_popups_3`): 48 pass, 2 n/a, 3 pending, 0 red. The 3 pending are the
  apps and hooks tests of other owners; the rest are `sweep:` lines.
- Rules: 110 (`adr013_*`, `layers`, `hub_enums_only_shrink`). `adr013_ratchets/plugins.txt` lost its four lines (the browser's
  `menu`, `shown`, `tab` draw state and `Intent::Tab`). `hub_budget.txt`: `screens/select.rs` `Note`, `Picked`, `Menu`, `Intent`
  and `app.rs AppEv` each 1 lower (the old `/plugins` variants); the framework's hubs grew by what every feature gets, with the
  reason written beside them: `Out` +2 (`Wish`, `OpenUrl`), `app.rs Intent` +2 (`Interest`, `WishShown`), `Feature` +1 (`on_interest`).

## Binary beside the oracle (pty 100x30, `CODEX_HOME` of `mock-model/home.py`, oracle with `--no-daemon -C <work>`)

No `/plugins` scenario exists in `mock-model/scenarios/` and `check.py` only writes `.ansi` captures, so the run is a throwaway
driver (a small VT screen over the pty bytes; keys `/plugins`, Enter, arrows, Esc, typed text); both binaries ran the same key
script and the last frame of each step was compared row by row. The same in both: the browser (header, tab strip, search row, rows,
the selected row's hint, footer), the Installed and other tabs, the search, the detail (title, status line, description, `Back to
plugins`/`Install plugin`, metadata, skills, hooks, apps), the add-marketplace tab and its prompt page (`enter submit · esc back`),
Esc on the browser (closes), Esc on the add page (back to the browser).

Differing rows:
1. Esc on the detail page: the oracle closes the whole popup (its detail `SelectionView` has no `on_cancel`; the way back is the
   `Back to plugins` row); ours pops to the browser with the cursor kept. This is the sheet's behaviour from before the pilot
   (`Plugins::back`) and the O1 assignment ("Esc pops, cursor kept"); the Tester tests pin it. A decision for the orchestrator:
   keep, or close the sheet on Esc from the detail.
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
- UI half `src/features/plugins/`: `mod.rs` (`KIND`, `impl FeatureView for Plugins`, `Args`, `open(cx, args)`), `sheet.rs`,
  `pages.rs` (the routes of the sheet's navigator, one `StatefulWidget` each), `lists.rs` (what each page shows, as the picker's
  lists), `strings.rs` (all the words). The shared pieces every feature uses: `widgets/interest.rs`, `widgets/sheet.rs`
  (`Sheet`, `Opened`, `Door`), `widgets/directives.rs`, `app/presenters/mod.rs` (the one presenter).
- The two list files (`core/src/features/mod.rs`, `src/features/mod.rs`, `feature_list!` and the `view_*` macros) are the only
  places that name a feature; everything else asks the list (`features::opens_sheet(command)`, `verb(cx, intent)`).

The verb enum, `plugins::Intent`: verbs with ids, never gestures: `Run{cwd}`, `Read{key}`, `Install{key}`, `Uninstall{key}`,
`SetEnabled{key,enabled}`, `AddMarketplace{cwd,source}`, `RemoveMarketplace{cwd,name}`, `UpgradeMarketplace{cwd,name}`,
`OpenSetupPage{key,app}`, `EndSetup{key,how}`. A widget sends one with `verb(cx, Intent::..)`; the feature decides.

Remote and op shape: what the server answered is `Remote<T>` (`Loading(Option<T>)` keeps the last good answer on show, `Ready`,
`Failed(words)`), kept per key (`catalog` per cwd, `detail` per `PluginKey`), never "the current X". A change the user asked for is
an `Op{at: Gen, status: Running | Done(Outcome) | Failed(words)}` in a map by `OpKey` (`Install(key)`, `Add{cwd}`, ...): a second press
of the same key finds `Running` and does nothing (the double press collapses); a reply is applied by key and generation, so a late
reply of an older `at` is dropped. A page that waits for an op keeps `awaiting: Option<(OpKey, Gen)>` (the generation before it asked)
and reads the op's status as newer than that, so it never reacts to an op it did not ask for.

The sheet: a modal route (`show_dialog` with `sheet_options`), a `StatefulWidget` with `eq` that is a `Sheet` (it declares
`alive(&Seen)`, here "the feature is on"; `adr013_sheets_declare_alive` checks it) whose body is
`Interest::new(Listing{cwd}, Navigator::new().home(bottom(Home{..})).back_key(None))`. The browser (`Home`) is the first route; detail,
add, remove, upgrade and auth pages are pushed onto the sheet's navigator, each a page with its own `State`. The browser is never
rebuilt, so Esc pops with the cursor, tab and query where they were (the picker's own state is the browser's `State`; no
`TabController` was needed). `Home` registers the navigator in the `Door` (`Opened{kind, scope, nav}`) so the presenter can push a page for
a wish. `open(cx, Args{cwd})` takes ids and nothing else.

Interest and wish: a widget that shows a feature's data wraps itself in `Interest::new(feature::Interest::.., child)`; it is the only
sender of `Intent::Interest`, raising the counter at mount and lowering it at dispose; the feature's `on_interest` loads when the
counter goes 0 to 1, and a reconnect asks again for an interest whose list is known. The sheet asks for nothing itself. A wish
(`wish(Wish::Auth(key))`) is a `SurfaceWish` with a `seq`; the presenter shows it once through `FeatureView::shown` and sends
`WishShown(seq)`.

Tests split: (1) core `core/tests/plugins.rs`, no UI: each verb, a double press, a late reply, a cwd change, a failed op, a wish, the
reconnect; (2) Tester `tests/pages/plugins.rs`: the route model (Esc, a tap, the cursor kept); (3) the oracle shard
(`tests/oracle/<shard>.rs` and its fixtures): the oracle's own screens through the real app; (4) one probe per rtwk finding
(`tests/rtwk_o1.rs`); (5) rules: the family's ratchet file (`tests/rules/adr013_ratchets/<family>.txt`) and `hub_budget.txt` shrink.

## What the pilot taught (for the next families)

The boundary holds when the UI half keeps no copy of the feature's data: every page selects `FeatureStates` and the feature's
selectors and sends a verb, so the old sheet's `Menu`/`Want`/`Pending` hub state went away and what the sheet holds is only what is
drawn (the cursor, the tab, `awaiting`). What cost the time: (a) the rules make the list the only place that names a feature, so
everything a feature needs from the UI (open a sheet, show a wish, a fact built in the host) must be a generic hook (`FeatureView`,
`Fact::config`) and not a call that names `features::<f>`; add the hooks first; (b) rtwk's nested `Navigator` in an `IntrinsicHeight`
cannot size a page that a `LayoutBuilder` builds (the picker's pane reads its height from its constraints, and a `LayoutBuilder` has no
intrinsic height before it is built, Flutter's rule too), so the sheet is a cap-high navigator with its pages aligned at the bottom
(`pages::bottom`), pinned by `tests/rtwk_o1.rs` as the repro for rtwk; (c) `Remote::Loading(Some(old))` matters for lists: a refresh
behind an op must not blank the page; (d) the tab ids of a placeholder list and of the loaded list differ (`tab_matching`, `tab_now`), so
a surface that restores a tab by id needs a resolver, not equality; (e) reconnect: until `attach` fails the in-flight asks, an in-flight
list is not asked again (F6; the core test says `pending(F6)` for that half); (f) R1 bans `.follow(`, `.channel(`, `.subscribe(`,
`cx.spawn(` outside the shells, so a feature's op tracking is a private `track`/`await_op` over the app's step, not a spawn.

## Gap rows and Needs

- Gap: `ScopeChanged` (the oracle closes the popup when the cwd changes) is not carried: the sheet's `alive` has no scope input yet.
- Gap: a failed list refresh replaces the shown catalog by the error (`Remote::Failed`); the stale catalog is not kept on show.
- Closed by the pilot: the "no OpenUrl door" gap; the setup URL is `Out::OpenUrl`.
- Need (adr013-features-trait): `Feature::Menu/Pick/Prompt/choose/picked/prompt_submitted` stay for the other families (their lines
  remain in `adr013_ratchets/features.txt`); the `FeatureView` hooks (`SHEET`, `open_sheet`, `shown`, `menu_list`, `prompt_words`) are
  what that task generalises.
- Need (core, F6): `attach` fails the in-flight requests so a reconnect asks again for an interest whose list is in flight.
- Need (rtwk, the person files): the O1 repro `tests/rtwk_o1.rs`: a navigator in `IntrinsicHeight` cannot size a `LayoutBuilder` page.
- Need (mock-model): a `/plugins` scenario and a grid renderer for `.ansi` captures (the throwaway driver of this run is not committed).
- Need (decision): differing row 1, Esc on the detail.
