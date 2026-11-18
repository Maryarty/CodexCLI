FIX

# VS1 review: the Agent command center (`codex-rtwk agents`), range 74acf02ce..714c3e32a

Reviewer: codex-reviewer (Opus), 2026-11-18, read-only on `claude/tui-rtwk` at 714c3e32a.
Scope: `codex-rs/tui-rtwk` core `screens/agents/*`, view `src/screens/agents/*`, `src/shell*`,
`host/`, tests `tests/agents*.rs`, `tests/shell_agents.rs`, `core/tests/agents*.rs`, layers/rules
tests, the parity pack `docs/rtwk-port/parity/agents-live.md`, process.md §5, fleet brief. The kit
(`tests/kit`, `tests/oracle_snap`, `kit_proof.rs`) and `perf/*` got a lighter read.
Evidence run: `nextest -p codex-tui-core --test agents --test agents_live`: 52/52 pass.
Known items (colours, Delete label, Ev::Account, pinned paging, hidden cursor, unsubscribe,
--help, PENDING in default profile, flaky debounce test, G35, G36) are not repeated here.

Why FIX: four defects with failing inputs (pin completion, same-flush stale reads in keys and
clicks, an underflow at narrow widths, leaked search tickets), a deep clone of `ScreenStates` per
streamed reasoning delta, and view mechanics that rtwk already has (layout arithmetic, word wrap,
ellipsis, dividers, an inline modal in place of `show_dialog`). A landed defect is fixed forward,
so this is a list of follow-up work, not a revert.

Counts: defect 5, should-fix 14, note 17. A dedicated "Size" section follows lens 5.

## Defects

**D1. Pin completion toggles instead of setting the requested state** (defect)
`core/src/screens/agents/refresh.rs:257-265`, commit 2364357f8 / 646f838d2.
`(Ctx::Pin { thread }, Ok(_))` flips membership (`contains` then remove, else `insert(0, ..)`).
The oracle (`tui/src/app/agents_overview_actions.rs:150-160`, `complete_agents_overview_pin`) sets
the state by the requested `pinned`, pushes new pins at the end, then refreshes the list.
Failing input: TogglePin on X (pin). A first-page `Listed` that already has X in `pins` arrives
before the Pin reply, then the reply comes back `Ok`. X now shows as unpinned while the server
has it pinned. Also, a fresh pin goes first in rank instead of last, and the failure text has no
`Failed to pin task: ` / `Failed to unpin task: ` prefix (refresh.rs:247-250).
`pin_action_updates_shared_section_state` (core/tests/agents.rs:1329) only checks `contains`,
so none of this is caught.
Fix: `Ctx::Pin { thread, pinned }`. Set the membership idempotently, push at the end, and queue
a `List` refresh as the oracle does. Add a test for the order Pin-intent → Listed(with X) →
reply, which is the R9/R10 permutation test this slice is missing.

**D2. j/k (and Up/Down under the search line) step from a stale model** (defect, ADR-002 cond. 4)
`src/screens/agents/mod.rs:635-658`, `src/screens/agents/view.rs:130-139`, commit 011bf8908.
The next row is computed from `center.selected_entry()` of the watched model. The model changes
only after the emitted `Select` is applied at the flush.
Failing input: `jj` arriving in one terminal read (fast typing, or key repeat while a frame is
busy). Both presses compute the neighbour of the same row, so the highlight moves one row
instead of two.
Fix: step from the controller, which updates synchronously: `self.selection.cursor()`, then
`self.selection.set_cursor(..)`. The existing `Listening`/`Msg::Cursor` path tells core. The
step remains the G35 stopgap, but it no longer makes a decision from a same-flush read.

**D3. A click decides "open or select" from a stale model, and the intent carries no id** (defect, ADR-002 cond. 4, R14)
`src/screens/agents/mod.rs:690-698`; `Intent::Accept` / `Archive` / `Delete` / `Hide` / `Stop` /
`Fork` carry no thread (`core/src/screens/agents/mod.rs:427-446`).
Failing input, in one flush: Down (the list moves its cursor to B, and `Msg::Cursor` queues
`Select(B)`), then a click on A, the row that was highlighted. `Msg::Row(A)` sees the stale
`selected == A` and sends `Accept`. Core applies `Select(B)` and then `Accept`, so it opens B,
the row the user did not click.
Fix: compare with `self.selection.cursor()`. Send `Intent::Open(ThreadId)` (and the same for the
other actions) with the id taken from the widget, so core never resolves "the highlighted one"
on its own (R14: ids into intents).

**D4. Subtraction underflow at narrow widths** (defect)
`src/screens/agents/view.rs:432` and `:597`: `let title_width = inner - 4 - ...` where
`inner = list_width - 1` and `list_width = width - 4` (`layout.rs:35-41`).
Failing input: a terminal 8 columns wide or less, with at least one task. A dev/test build panics
with `attempt to subtract with overflow` (overflow checks are on in `[profile.dev]`). A release
build wraps, and the `as i32` gives a negative `SizedBox`.
Fix: `saturating_sub`. Better, the Row/Expanded layout in S1, which makes the arithmetic go away.
Add a test at width 8.

**D5. Every debounced or superseded search leaks a ticket** (defect, R5 end paths)
`src/shell/search.rs:56-58,71`; `core/src/app.rs:423-437` (`send_agents` inserts into
`app.outgoing`); nothing removes the entry.
An aborted job resolves to `None`, so no `AppEv::Reply` arrives for its ticket, and
`AppCtx::Screen(Search{..})` stays in `outgoing` for the life of the connection.
Failing input: typing `abcdef` in the search line leaves 5 entries for good. Each keystroke adds
one more.
Fix: the aborted job answers with `Reply::error(Superseded)`, or core drops the previous search's
ticket when `restart_history` bumps `search_seq`. Add an end-path test.
(The view's `Ctx::NewWorktree` arm, refresh.rs:198, is unreachable for the opposite reason: no
ticket is minted for it, app.rs:425-428.)

## Lens 1: rtwk first (process.md §5)

**S1. The two-pane screen is hand-laid in cells: a port of `center_layout`** (should-fix)
`src/screens/agents/layout.rs:31-54` (`areas`); `mod.rs:747-789` (a `LayoutBuilder` that hands
width/height to every builder); `view.rs:86-91` (`cell`/`pad` with fixed widths; 32 `as i32`
casts in view.rs); `view.rs:963-972` (a vertical rule drawn as `body_height` one-cell rows);
`view.rs:281` (`"─".repeat(inner)`); `view.rs:430-486` (row columns at computed widths).
This is the ratatui Rect arithmetic that §2 says becomes a widget's `build`.
Fix: `Padding` for the 2-cell inset. A `Row` of `Expanded(list)`, a 3-cell gap, and
`SizedBox(38)` for the details shown from 90 body cells (only that threshold needs the width:
one `LayoutBuilder` read). rtwk has no Divider widget. The vertical rule is the gap's
`BoxWidget::border(Border::new().left(dim))`, and the header rule is
`BoxWidget::border(Border::new().bottom(dim))` around the tab row
(`rtwk-widgets/src/layout/box_.rs:44,276`), so no width is needed for either. Row columns as `Row` with `Expanded(title)` plus fixed
`SizedBox`es for status/age. D4 goes away with it.

**S2. Hand word wrap and ellipsis** (should-fix)
`layout.rs:133-164` (`wrap`, used by details and both dialogs: view.rs:648-660, 1163-1170,
1274-1282), `layout.rs:166-187` (`ellipsize`: view.rs:249, 644-646, 806-808).
rtwk `Text::wrap(TextWrap::Word)` and `Text::overflow(TextOverflow::Ellipsis)` do both. Layout
then measures the dialog height, so no rows need precomputing. If a snapshot shows rtwk cutting
differently (the comment at layout.rs:175, "trailing blanks kept", hints at that), the difference
is an rtwk gap row with both renderings, not a local helper.
`center_truncate_path` (layout.rs:283) is oracle behaviour rtwk does not have. Keeping it is
right.

**S3. The confirmation and failure panels are inline modals, not `show_dialog`** (should-fix)
`mod.rs:579-612` (routes every key by hand while a panel shows), `mod.rs:522-532` plus
`Msg::Refocus` (focus juggling), `view.rs:1131-1251` (`Stack`/`Positioned`, centered by hand).
`rtwk::overlay::show_dialog` with `DialogOptions { alignment, barrier_alpha, .. }` and the same
custom panel as the builder provides the key trap, Escape, focus return and the barrier. The
dialog returns `Option<bool>` (`close_dialog`), and the opener turns it into the intent with the
thread id (R14). Core would then not need `Model.confirm` (UI state, see L1). For the refusal,
the model keeps `failure` and a presenter opens the dialog, as with the request presenter.

**S4. Two-way sync of the selection: core shadows the list cursor** (should-fix)
`core/src/screens/agents/mod.rs:333` (`selected`), `:336` (`browse`); `src/screens/agents/mod.rs:499-506`
(model → controller) and `:679-689` (controller → `Select` → model).
§2 puts "a view's cursor in a list" in widget state, and the fleet brief says "no app-side shadow".
Two sources of truth synced both ways are what make D2 and D3 possible.
Fix: the controller owns the cursor. Intents carry the id (D3). The oracle's selection-after-
removal rule (`selection_after_removal`) becomes a pure function core exports and the view applies
to the controller. The decision belongs to the person, because the pack (agents-live.md:81)
claims "Model holds no UI-widget state" while the code holds it.

**S5. Rows paint their own selection; the list is rebuilt from fresh closures every build** (note)
`view.rs:529-575`: a `ListenableBuilder` per row on the controller, with
`selected_style(Style::new())`. `SelectionList::new` gets new closures on every build, so the list
never compares equal.
Use `SelectionList::with_builders` with `Rc` closures kept across builds (see S9's memo). The
custom `›` marker needs the item builder to know selected/cursor. Rather than leaving a
workaround, file an rtwk need ("item builder told selected/cursor") next to G35.

**S6. A GestureDetector per row duplicates SelectionList's own tap** (note)
`mod.rs:720-728`, `view.rs:165-178`. A tap already moves the cursor and selects
(`rtwk-controls/src/selection_list.rs:565`). The only extra behaviour is "a click on the
highlighted row opens it". Build that from the controller (a tap whose key equals the cursor before
the tap), not from n handlers made in every build.

**S7. The help columns are padded with spaces** (note)
`layout.rs:375-443`. The 3/2/1-column flow rule is behaviour, but the padding is layout. Use a
`Row` of `Column`s with `SizedBox` widths and `Padding` for the 4-cell gap.

## Lens 2: Layers

**L1. UI-only state lives in the core model** (should-fix; no rule catches it, so a missing-rule finding too)
`core/src/screens/agents/mod.rs:323-336`: `confirm`, `filter` (a tab index), `searching`,
`rename`, `selected`, `help`, `browse`.
§2 and A4.4 put this in widget state or `UiApp.ui`. `help: bool` is the plainest case.
`the_ui_model_is_the_core_model_plus_ui_state` (tests/layers.rs:283) and the R14 rules check
names, not this.
Fix: move `help`, `confirm` (via S3) and the cursor (via S4) out. `search` (the query) and
`grouping` (persisted config) are legitimately core. For the rule, a core-side test can assert
that screen models expose no field from a denylist (`help`, `cursor`, `scroll`, `focus`,
`selected`), or each screen can split `Model` into domain and UI parts.

**L2. Protocol types reach the view through core's projection** (should-fix; the rule passes the crossing)
`core/src/screens/agents/center.rs:33-34` (`RowData { thread: Thread }`, the protocol type). The
view reads its fields at `src/screens/agents/view.rs:476,699,727,1107` and `mod.rs:556`, and
`ThreadUsage` fields at `src/screens/agents/usage.rs:36-49`.
`views_never_name_protocol_shell_tokio_or_io` (tests/layers.rs:137-166) greps for
`codex_app_server_protocol`, so field access through a core re-export passes.
Fix: `RowData` projects what the view shows (title, name, cwd display, updated_at, preview), and
`Usage` exposes the figures. As a rule, view sources must not contain `.thread.` / `.estimate.`,
or core's view-facing structs get a compile-time check of their field types.

**L3. The view times a domain request** (should-fix)
`src/screens/agents/mod.rs:508-521`: `did_change_dependencies` emits `NeedUsage` whenever
(thread, fetched_at, pending, now) changes. That is two flushes per change, and the cadence
depends on a mounted view.
Core sees every input (`Ev::Clock`, `Select`, the usage answer), but `apply` drops its `fx`
(`core/src/screens/agents/mod.rs:571-574`).
Fix: call `refresh_usage` from `apply(Clock)`, from `update(Select)` and after the answer, and
send through `fx`. The view then holds no `usage_seen`.

**L4. The R14 controller allowlist grows by file** (note)
`tests/rules.rs:415-419` adds `src/screens/agents/mod.rs`. Every screen with a list or field will
need its own line. Scope the rule instead: controllers are made only inside a `State` of
`src/screens/**` or in `ui/thread_ui.rs`.

**L5. The scripted server ships in the library** (note)
`src/shell/fake.rs` (390 lines) and `Backend::Fake` are compiled into `codex-tui-rtwk`. Put them
behind a `test-support` feature.

## Lens 3: Code quality

**Q1. A second copy of a carried function escapes the drift test** (should-fix)
`core/src/screens/agents/sources.rs:300-309` copies `supports_shared_pinning`, which
`carried.rs:98-104` already carries under an `// oracle:` marker checked by `tests/carried.rs`.
Use `super::carried::supports_shared_pinning`.

**Q2. Dead code** (note)
- `core/src/screens/agents/notifications.rs:105-106` (`let buffer = ...clone(); let _ = buffer;`)
- `refresh.rs:198-202` (the `Ctx::NewWorktree` arm, unreachable; see D5)
- `refresh.rs:212-214` (`let _ = thread;`)
- `Intent::Refresh` (`update.rs:43`) is never sent. Nothing re-lists after open, while the oracle
  refreshes after a pin and on refresh notifications.

**Q3. Fourteen copies of one debug entry** (note)
`src/screens/agents/mod.rs:40-211`: each entry repeats the same `EntryOpen::Route` closure,
differing only in `can_return`. One `entry(name, description, script, can_return)` helper turns
170 lines into about 25.

**Q4. Size** (should-fix): see the "Size" section below.

**Q5. Comments that lie** (note)
- `core/tests/agents.rs:541, 601, 732, 1392, 1479` still say "RED: ..." for tests that pass.
- The pack's core "Gaps" table (`docs/rtwk-port/parity/agents-live.md:70-81`) still says
  "Discovery is one source", "the host function is not written", "`Ev::Backend{worktrees}` is
  never sent" and "Model holds no UI-widget state". The shell section and the code say otherwise.

**Q6. Errors stringified at the seam, then parsed back** (note)
`host/src/in_process.rs:256-259` turns the client error into text. `sources.rs:327-334`
(`refused`) then parses `(code N)` back out of it. `src/shell/workspace.rs:123` puts
`{error:?}` (Debug) into user text.
Fix: keep a structured error (`code`, `message`) across the `send` closure.

**Q7. `Eq` claimed by hand** (note)
`core/src/screens/agents/mod.rs:233-234, 339, 379, 531` add `impl Eq` to types holding protocol
`Thread`, which does not derive `Eq`. That holds only while no field is a float. Prefer comparing
by id/revision where `Eq` is needed (e.g. `ScreenStates` for the provider).

**Q8. The worktree manager is shared through a "last made" slot** (note)
`host/src/workspace.rs:26-28, 122-127, 139-144`. Binding depends on which `w` ran last. Core's
`blocked()` makes concurrent `w` impossible today. Still, return the manager with the `Checkout`
rather than through shared mutable state.

## Lens 4: Concurrency and performance

**P1. A whole `ScreenStates` clone per observed notification, including every reasoning delta** (should-fix, high)
`core/src/app.rs:305-306` (`observe` behind `Rc::make_mut(&mut app.screens)` for
`ReasoningSummaryTextDelta` and nine other kinds, notifications.rs:16-30) and
`core/src/app.rs:265-270` (`wanted_dirs` behind `make_mut` after every `Reply` or `Screen` event,
chat replies and clock ticks included).
The root provider holds an `Rc` of `screens` (`src/ui/view.rs:66`), so the count is ≥2 at apply
time and every `make_mut` deep-clones every screen's model, including all of the center's
threads. The new `Rc` then notifies every `watch::<ScreenStates>` (agents: mod.rs:488, 568, 718),
which recomputes the center. During a streamed reasoning this happens per delta, even with the
center closed: the delta handler changes nothing after the first header (notifications.rs:104),
but the clone has already happened.
Fix: decide on `&Model` first (pure `wants(&Model, &Notification)`, `wanted_dirs` computed
read-only) and call `make_mut` only for a real change. Narrow the screen to
`select::<ScreenStates, _>` of its own part. Longer term, put each screen's model in its own `Rc`
inside `ScreenStates`.

**P2. The center is derived 2-3 times per change, and each derivation clones every row** (should-fix)
- `src/screens/agents/mod.rs:498, 572, 719` call `Model::center()`, and `update.rs` calls it again
  per intent.
- `center.rs:152-167` clones every root `Thread` into `RowData` and computes `Details` for every
  row, though only the highlighted row's details are drawn. `details.rs:67-74` also sorts inside
  its loop.
- `center.rs:194-217`: `sort_by_key` builds owned `String` keys per comparison, from
  `cwd.display().to_string()` and `project_key().clone()`.

Fix: one derivation per model revision (keep `Rc<Center>` keyed on the model's `Rc` pointer, or
use `ViewCx::memo` at the root per §2/R7), `RowData` over `Rc<Thread>` or borrowed ids, details
only for the selected row, and `sort_by_cached_key`.

**P3. Two-source paging re-reads a partly shown page and skips by count** (should-fix)
`core/src/screens/agents/sources.rs` header and `:226-236` (`skip = part.skip + shown`).
Between two "Show more" presses, a task that became active moves to the top under `RecencyAt`.
The re-read page then shifts by one, and `skip` hides a row that was never shown. That row stays
missing until the center is reopened. A shift the other way duplicates a row (merge dedupes it).
The oracle keeps the unshown rows it fetched. `Paging.buffer` (mod.rs:255) already exists for
that. Keeping the rows also halves the requests per page.

**P4. The 180 ms debounce (`shell::search::Latest`)** (note; the leak is D5)
The mechanism is sound: abort-before-replace, the delay only on `first`, the command pump stays
concurrent through `FuturesUnordered` (in_process.rs:194-234), and continuation pages are not
delayed.
Two remarks:
- Aborting also kills a request already sent to the server, which matches the oracle.
- `reject(client, &key).await` runs inline in the `select!` arm (in_process.rs:217), so a slow
  reject stalls notifications and in-flight replies. Push it into `inflight` as well (it needs a
  request handle rather than `&client`).

**P5. The in-process app-server future and the ~8 GB rustc** (note: options, the decision is the person's)
`host/src/main.rs:1-4` (`recursion_limit = "256"`), `host/src/in_process.rs:180` (`Box::pin`).
`Box::pin` moves the future to the heap but does not avoid computing its layout. The host crate
still lays out `InProcessAppServerClient::start`'s nested future, so the cost recurs on every
host rebuild. Options:
- (a) A small lib crate (for example `tui-rtwk/server`) that owns `start_args`, `InProcess` and
  `run` and exports only `fn connector(..) -> Box<dyn Connector>`. The heavy future is then
  compiled once and cached, and edits to the UI or host no longer pay for it.
- (b) The app-server out of process (stdio, as the remote client does). The UI binary then links
  no server at all, which also makes the release build that the perf A/B needs possible:
  `f89aacd5f` compared a DEBUG rtwk build against the release oracle, a result that must not be
  cited as a perf claim.

Recommendation: (a) now as the minimal seam, (b) as the perf-wave decision.

**P6. Smaller costs** (note)
- `update.rs:349-366` (`wanted_dirs`) allocates every thread's cwd string after each
  Reply/Screen event.
- `refresh.rs:44-94` (`merge_page`) uses `Vec::contains` inside loops (O(n·m)), and so do
  `observe`'s `iter().find`s.
- The provider compares `ScreenStates` deeply (`PartialEq`, core/src/screens/mod.rs:87).

None of these matters at 100 threads. All three grow with history. Use `HashSet`s and ids.

**P7. Virtualization** (note)
`SelectionList` with `item_extent(1)` builds only the visible rows, which is right. The per-build
O(n) work outside it is `taps.rows` (mod.rs:720-728), `keys` (view.rs:516-520) and the full
`Center` (P2).

## Lens 5: Tests

**T1. The mandatory R tests were not extended** (should-fix)
This slice adds 13 `Ev` variants, about 28 intents and 11 `Ctx` kinds, but there is no
permutation test (R9/R10: a reply before or after a `Listed`, a `Notice` during a removal), no
end-path test (R5: every ticket resolves; D5), and no `assert_consistent` after each event (R7).
D1 and D5 are exactly what those tests catch.

**T2. Kit tests depend on wall-clock time** (should-fix)
`tests/kit/seed.rs:90-95` polls with `std::thread::sleep(5ms)` × 200 while the fake backend runs
on real tokio timers. `tests/shell_agents.rs:496-532` measures 100-300 ms real intervals. The
rtwk `Tester` clock does not govern `tokio::time`, which is the root cause of the varying red
counts already seen in the kit (and of the known flaky debounce test).
Fix: run the fake backend on a paused tokio clock (`tokio::time::pause` with `advance` from the
`Tester`'s `pump_for`), or drive the fake synchronously.

**T3. The oracle is the only source of expectations** (note: fine)
Screen tests compare with the oracle's `.snap` rows (`tests/agents.rs:43-63`, `tests/kit/compare.rs`).
The two comparison normalizations (platform key labels, wide-cell padding) are documented and
confined to the comparison. Several tests check only fragments (`screen.contains(..)`,
tests/agents.rs:131-138, 328-361) where a snapshot exists. Prefer the whole snapshot or a region
(`assert_snap_region`).

**T4. Shallow pin test** (note)
`core/tests/agents.rs:1329-1352` checks `contains`, which passes for both the toggle and the
oracle's set-by-flag semantics (D1). Assert the rank order after a pin, and the order with an
interleaved `Listed`.

## Size

The figures are code lines: no blanks, no `//` comments, no tests. Counted with
`grep -vE '^\s*$|^\s*//'` over line ranges at 714c3e32a.

| Part | Lines |
|---|---|
| UI: view.rs | 1,206 |
| UI: mod.rs | 692 |
| UI: layout.rs | 448 (57 of them tests) |
| UI: strings.rs | 320 |
| UI: usage.rs | 117 |
| Core (11 files) | 2,651 |
| Shell (`shell.rs`, `rpc`, `search`, `workspace`) | 346 |
| Fake (`shell/fake.rs`) | 322 |
| Host (`in_process`, `workspace`, `main`) | 491 |

Without the fake, that is about 5,900, against the oracle's 5,446 (`agents_overview*` plus
`agent_center`).

The view is the oracle's renderer retyped as cell arithmetic under a `LayoutBuilder`, so it
cannot come out smaller. Core carries, on top of the oracle's logic, the plumbing of the layer
split and an i18n table the oracle does not have.

### UI: blocks rtwk replaces (estimated saving)

| Block | Lines | What replaces it | Saved |
|---|---|---|---|
| `view.rs:74-95` (`cell`, `pad`, `span`) | 15 | the fixed-width helpers go once nothing is placed by cell count | ~15 |
| `view.rs:202-299` (`header`) | 96 | title `Text`; tab row as `Row` + `Expanded`; `filter_keys` hint at the end; rule as `BoxWidget::border(Border::new().bottom(..))`; no `used`/`gap` bookkeeping (strip stays, see G36) | ~50 |
| `layout.rs:59-112` (`strip`) | 50 | `rtwk::controls::TabBar::strip` scrolls, marks the cut edges and clicks; blocked by G36 (the oracle's filled, touching tabs) | ~50 once G36 closes |
| `view.rs:368-494` (`entry`) | 121 | `Row[marker, dot, Expanded(Text…overflow(Ellipsis)), status, age]` with fixed `SizedBox` widths; no `title_width` arithmetic (D4); selected fill from the list's `selected_style` once an item builder knows "selected" (see S5) | ~60 |
| `view.rs:495-643` (`list`) | 147 | `SelectionList::with_builders` kept across builds; no per-row `ListenableBuilder`, `keys` vector or `taps` (S5, S6); the overflow arrows stay | ~75 |
| `view.rs:96-180` (`RowKey`, `neighbour`, `Taps`) | 67 | ids from the list's own keys; `neighbour` only while G35 stands; `Taps` goes with S6 | ~40 (25 more after G35) |
| `view.rs:644-667` + `668-817` (`clip`, `wrapped`, `details`, `clip_pair`) | 165 | a `Column` of `Text`s with `wrap(TextWrap::Word)` / `overflow(Ellipsis)`; label/value pairs as `Row[Text, Expanded(Text)]`; the 2-line prompt cap as `Text::max_lines(2)` with the elision mark (if rtwk's cut matches; else a gap row) | ~70 |
| `view.rs:925-999` (`body`) + `layout.rs:1-58` (`areas`) | 114 | `Padding` + `Row[Expanded(list), gap with a left `Border`, SizedBox(38, details)]`, one width threshold | ~80 |
| `view.rs:1020-1100` (`help`) + `layout.rs:355-445` (`help_rows`) | 156 | a `Row` of `Column`s (3/2/1 picked from one width read), `Padding` for the 4-cell gap, `Text` per entry; the "resize" cut by `ClipRect`/max height | ~85 |
| `view.rs:1101-1308` (`confirm_name`, `dialog_row`, `confirm_dialog`, `failure_dialog`) | 195 | `overlay::show_dialog` with `DialogOptions` (centred, barrier); the panel a `BoxWidget` with `Text` wrap and a two-row `SelectionList`; no blank-fill rows, no height arithmetic (S3) | ~115 |
| `layout.rs:113-211` (`columns`, `head`, `wrap`, `ellipsize`, `front_truncate`) | 87 | `Text` wrap/overflow, `rtwk-unicode` widths; keep `front_truncate` only if the strip needs it until G36 | ~70 |
| `mod.rs:40-214` (`debug_entries`) | 172 | one `entry(name, description, script, can_return)` helper plus a table (Q3) | ~140 |
| `mod.rs:487-566` (`did_change_dependencies`) | 77 | model → controller sync goes with S4; the usage request moves to core (L3) | ~55 |
| `mod.rs:457-486` (`refocus_after_frame`, …) | 23 | `show_dialog` returns focus | ~20 |
| `mod.rs:567-713` (`update`) | 131 | no dialog key routing (`579-611`), no click "open or select" against the model (D3), no `NeedUsage` | ~50 |
| `mod.rs:392-456` + `714-795` (keys, `build`) | 138 | `Shortcuts`/`Actions` map in place of `intent_of`/`step_of`, `build` without width hand-off | ~45 |
| `strings.rs:22-285` (`Strings` + `EN`) | ~260 | every field is named twice (the struct, then `EN`); one `macro_rules!` that declares field and English text together, or a per-screen `&[(&str, &str)]` keyed table | ~120 |

UI total saved is about 1,100 (about 1,000 without G36 and G35), which brings 2,689 to
**about 1,600**.

### Core: duplication and plumbing that can collapse

| Block | Lines | Change | Saved |
|---|---|---|---|
| `Command` (`mod.rs:461-518`) and `Ctx` (`mod.rs:535-570`) | 90 | 13 and 13 variants that mostly carry the same fields. Let the ticket context be `Command` plus extras (`Ctx { command, extra }`), so `apply_answer` matches on the command it sent. `Ctx::NewWorktree` and the unreachable arm go (Q2). | ~40 |
| `wire.rs:220-307` (`seed`, `seed_notice`) | 84 | fixture-only parsing with local `Seed` structs; derive `serde::Deserialize` on a fixture enum that maps 1:1 to `Ev`, or let the fixture script speak protocol JSON through `decode_reply` | ~50 |
| `wire.rs:37-153` (`wire_request`) + `sources.rs:132-176` (`requests`) | 108 + ~40 | two builders of `ThreadListParams` (`list_params`, and the fan-out's own); one `list_params(cursor, source)` | ~20 |
| `sources.rs` (280) + `discovery.rs` (239) | 519 | two-source paging packs two cursors into one string (`Cursors::read`/`write`) and re-reads with `skip` (P3); history and search have parallel `show_buffered`/`page_request` vs `load_matches`/`take_matches`. Keeping fetched rows in `Paging.buffer` (as the oracle does) removes `skip`/`Part` bookkeeping, and one pager type serves both | ~110 |
| UI state in core: `update.rs:205-348` (`confirm`, `editing`, `cancel_editing`, parts of `blocked`), `Model` fields, `Confirm`/`Rename`/`help` arms in `update` | ~150 | moves to the view or `UiApp.ui` (L1, S3, S4), where `show_dialog`, `TextField` and the list controller already hold it | ~100 (moves part, deletes part) |
| `center.rs:225-260` (`project_key`, `project_heading`, `directory_key`) | ~35 | three variants of one key; one function that returns `(sort key, heading)` | ~20 |
| manual `impl Eq` (`mod.rs:233-234, 339, 379, 531`) and `apply_thread`/`ThreadPart`/`ThreadEv` stubs (`mod.rs:453-456, 583-587`) | ~15 | compare by revision (Q7); the empty thread-part stubs go if the screen registry allows | ~10 |

Core total saved is about 350, which brings 2,651 to **about 2,300**. The rest is domain logic
the oracle has too: notifications, merge, details, usage, carried helpers, and the protocol
wire (about 280).

### Shell and host

`fake.rs` (322) is test support and belongs behind a feature (L5). It is not product size. The
host is mostly the in-process start (P5); option (b) of P5 would shrink it to a spawn.

### Realistic target

UI about 1,600 + core about 2,300 + shell/host about 800 = **about 4,700, about 15 % under the
oracle's 5,446**. Counted against the oracle's like-for-like (renderer + state, no protocol
plumbing, no i18n table), the UI alone falls from about 2,700 to about 1,600.

Below about 4,300 is not realistic while three choices stand:

- **the core/shell split.** `Command`/`Reply`/`Ctx`/`wire`/`seed` cost about 400 lines the
  oracle does not pay; that is the price of ADR-001's layers, not waste.
- **the i18n `Strings` table.** About 140 lines remain after the macro.
- **the debug fixtures.**

What blocks the target:

- **G36:** the oracle's filled tab bar. The strip and header bookkeeping, about 100 lines, stays
  until `TabBar` can draw touching filled tabs.
- **G35:** `SelectionList` keeps its `ListController` private. `neighbour`/`step_of`, about 25
  lines, stays.
- **An rtwk need not yet filed:** an item builder told selected/cursor (S5). Without it, rows
  keep painting their own selection, about 30 lines.
- **Cut parity of `TextOverflow::Ellipsis` and `TextWrap::Word`** against the oracle's
  `truncate_line_with_ellipsis_if_overflow`/`word_wrap_lines`. If a snapshot differs, the
  helpers (about 70) stay until an rtwk gap closes. That must be shown by a snapshot test
  first, not assumed.
- **The decision on S4/L1** (who owns the cursor and dialog state). Without it, neither the
  core about 100 nor the view sync about 55 can go.

## What the person or orchestrator decides

1. S4/L1: whether core keeps the highlighted task at all. The alternative is ids in intents with
   the cursor owned by `SelectionController`. This is the root of D2/D3, and the pack currently
   claims the opposite of the code.
2. P5: the server seam. (a) is a separate lib crate; (b) is out of process.
3. S2: if rtwk's `TextOverflow::Ellipsis` or `TextWrap::Word` cut differently from the oracle,
   whether to file an rtwk gap row or keep the helpers.
4. Size: whether the target of about 4,700 (about 15 % under the oracle) is acceptable, or
   whether to drop the i18n table, the fixture seed or the core/shell plumbing to reach about
   4,000. Each of these is a design choice with the trade-off named above.
