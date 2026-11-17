# Does ADR-003 remove Codex TUI state bugs as a class?

Research, read-only on both repositories. Date 2026-11-17. Oracle: `codex-rs/tui`
in /Users/admin/Documents/dev_projects/CodexCLI (all hashes below are upstream
openai/codex commits, checked one by one through the GitHub API: 508/508 of the
fix set and 24/24 of the supplementary set exist upstream). Design under test:
ADR-003 (architecture.md §ADR-003) and the variants M and R the person asked
for. Data: `commits.csv`, `issues.csv`, `fix-set.csv`, `subclass-counts.csv`,
`subclasses.tsv` in this directory.

## 1. Synthesis

1. **Under the strict test, ADR-003 as written removes no sub-class.** Three
   sub-classes lose the mechanism behind the oracle bugs: SC2a (replay on a
   thread switch), SC6a (state copies pushed into child widgets through
   setters), SC7a (display flags toggled by hand). These account for 15 of the
   55 state-mechanism commits in the fix set and 15 of the 34 state issues.
   Each one can still be written in the port: per-thread data put in an
   app-level field, app data kept in rtwk local state (`Obs`, `use_state`), a
   stored `status_visible` flag. A careful port avoids them. Nothing in the
   design rejects them. They are marked STILL POSSIBLE* (mechanism absent, still
   representable).
2. **With 14 design rules (§4), 28 of the 55 become IMPOSSIBLE by type or lint.
   25 more are CAUGHT by a test the rules make mandatory, and 2 stay possible.**
   That is 5.5% of the 508 commits removed as a class and 4.9% caught, not the
   "~22 points" the ADR cites. Of the 111 state_ui+race commits, only 39 are
   state-mechanism bugs. The other 72 are 34 non-runtime commits (tests,
   fixtures, build, copy, reverts), 19 backend/persistence/protocol, 8
   layout/style/input and 11 feature logic.
3. **The 508 subject filter undercounts the class.** Subjects shaped like
   "Preserve/Keep/Clear/Scope/Route ..." without a fix keyword give 106 more
   commits. A sample of 24 of them held 23 state-mechanism bugs, including the
   whole SC3 thread-routing series (8843960ba, 6f47fcb0b, 53a1f4c29). The
   class is larger than the fix set shows, and the rules cover it in the same
   proportions (10 IMPOSSIBLE, 12 CAUGHT, 1 possible out of 23).
4. **R or M.** A single hand-written reducer does not remove any sub-class that
   M or the mix leaves open. The oracle already routes 335 `AppEvent` variants
   through one dispatcher and still has every class. What R adds is
   testability: user intents become values, so the permutation tests for SC8b
   and SC9 can interleave them with backend events without a UI, and a session
   can be recorded and replayed as a list of values. The class removals come
   from where state lives and from the types used for ids, threads, lifecycles
   and transcripts (rules R1–R14). Those rules apply equally to M, R and the
   mix.
5. **Recommendation:** keep the mix and add rules R1–R14. Take R only at the
   boundary (R10): intents that touch backend-shared or cross-thread state
   (submit, steer, interrupt, approve, switch thread, RPC-issuing slash
   commands) are values in the same ordered stream as backend events. Local
   widget interaction (typing, scrolling, popup navigation) stays with rtwk.
   The person decides.

## 2. Method

- Fix set reproduced: `git log --no-merges -- codex-rs/tui codex-rs/tui2` =
  3,612 commits; the bug-classes.md §4 regex gives 508; classify.sh gives
  state_ui 87 + race 24 = 111 (`set111.txt`).
- Read for root cause: all 111, plus 24 state-flavoured fix-set commits from
  other classes (stale/switch/resume/duplicate... in the subject), plus 24
  commits outside the fix set (marked ° below). For each I read the commit
  message (Codex PRs carry Why/What sections) and, where the message was empty
  or vague, the TUI diff (b08bdd91e, baaf42b2e, 3f62b5cc6, 333663921,
  e4a202ea5, c0a84473a, 9aa11269a, f8b6b1db8, 1bd2d7a65). Total: 159 commits
  classified.
- Issues: `gh` search, openai/codex, label TUI, in:title, 15 keywords (stale,
  desync, out of sync, not updated, wrong thread, after resume, after switch,
  duplicate, race, doesn't update, not refresh, still shows, switching,
  disappear, reappear, stuck): 171 hits, 150 unique. From these I took 42
  state-looking issues and read their bodies. The mechanism comes from the
  reporter's analysis or from the linked fix; #30714's cause is unconfirmed.
- Verdict, strict: IMPOSSIBLE = the type system or a CI lint rejects the
  program; CAUGHT = a test the design makes mandatory fails; STILL POSSIBLE =
  anything else. "Careful code avoids it" counts as STILL POSSIBLE.
- Caveat: one reader, one primary sub-class per commit. Root causes come from
  commit text plus diff glances, not reproduction.

## 3. Sub-classes, evidence, verdicts

Counts: F = fix-set commits (of 135 read), S = outside fix set (of 24), I =
issues (of 42). "now" is ADR-003 as written (the mix); M and R differ from it
only where noted. ° = outside fix set.

| id | sub-class (root-cause mechanism) | F | S | I | now (mix = M) | R now | rule | after |
|---|---|---|---|---|---|---|---|---|
| SC1 | Late or superseded async reply applied: per-widget counters reset on widget replacement, aborted task's reply already queued, notification from an old runtime | 6 | 1 | 0 | STILL POSSIBLE | STILL POSSIBLE | R1 | IMPOSSIBLE |
| SC2a | Per-thread state lost/leaked on thread switch: one ChatWidget rebuilt and replayed from a bounded buffer | 4 | 5 | 10 | STILL POSSIBLE* | STILL POSSIBLE* | R2 | CAUGHT |
| SC2b | Per-thread widget-owned state (draft, kill buffer) lost or carried over on switch | 1 | 2 | 0 | STILL POSSIBLE | STILL POSSIBLE | R2 | CAUGHT |
| SC3 | Event/reply/effect routed to the wrong thread or backend (active thread used instead of origin; ids colliding across threads) | 1 | 4 | 4 | STILL POSSIBLE | STILL POSSIBLE (user intents: carry ThreadId) | R3 | IMPOSSIBLE |
| SC4a | Same item rendered twice or split: append-only history cells, started+completed or live+replayed or legacy+v2 each push a cell | 5 | 1 | 2 | STILL POSSIBLE | STILL POSSIBLE | R4 | IMPOSSIBLE |
| SC4b | Backend emits distinct items with the same text | 0 | 0 | 1 | STILL POSSIBLE | STILL POSSIBLE | — | STILL POSSIBLE |
| SC5a | Lifecycle-scoped transient state not cleared on every end path (complete / interrupt / fail / finalize) | 4 | 2 | 2 | STILL POSSIBLE | STILL POSSIBLE | R5 | IMPOSSIBLE |
| SC5b | Data dropped at a lifecycle end before flush | 0 | 1 | 0 | STILL POSSIBLE | STILL POSSIBLE | R5 | CAUGHT |
| SC6a | UI component keeps a copy of model state synced by setters/refresh calls (120 `set_*` setters in bottom_pane, 117 call sites) | 7 | 1 | 4 | STILL POSSIBLE* | STILL POSSIBLE* | R14 | IMPOSSIBLE |
| SC6b1 | Second source of truth read outside the model (auth.json, config default) | 1 | 0 | 1 | STILL POSSIBLE | STILL POSSIBLE | R12 | CAUGHT |
| SC6b2 | Remote data not re-fetched when it should be (refresh policy) | 1 | 1 | 4 | STILL POSSIBLE | STILL POSSIBLE | R6 | STILL POSSIBLE |
| SC6c | Derived value cached by hand for non-view consumers (/copy source, backtrack positions) | 2 | 0 | 1 | STILL POSSIBLE | STILL POSSIBLE | R7 | CAUGHT |
| SC7a | Display flag set imperatively and conflated (status indicator hidden also cleared task_running) | 4 | 0 | 1 | STILL POSSIBLE* | STILL POSSIBLE* | R7, R8 | CAUGHT |
| SC7b | Gating predicate with wrong semantics (what blocks the input queue) | 2 | 0 | 0 | STILL POSSIBLE | STILL POSSIBLE | R8 | CAUGHT |
| SC7c | One bool for two outcomes (view `complete` for accepted and cancelled) | 1 | 0 | 0 | STILL POSSIBLE | STILL POSSIBLE | R8 (convention) | STILL POSSIBLE |
| SC8a | Entity needed from a notification that can arrive after the response | 1 | 0 | 0 | STILL POSSIBLE | STILL POSSIBLE | R9 | CAUGHT |
| SC8b | Local state ahead of the backend: optimistic trim, cached turn id, missed reconcile | 3 | 0 | 1 | STILL POSSIBLE | STILL POSSIBLE | R9 | CAUGHT |
| SC9 | User input interleaved with a backend transition (submit during final stream, before session ready, held key across a mode change) | 4 | 1 | 1 | STILL POSSIBLE | CAUGHT | R10 | CAUGHT |
| SC10a | View state stale against changed data: selection by index after filtering, loading shown as empty, clock read in the view | 2 | 0 | 1 | STILL POSSIBLE | STILL POSSIBLE | R11 | IMPOSSIBLE |
| SC10b | Selection/order semantics after data changes (neighbour selection, stable order) | 1 | 3 | 0 | STILL POSSIBLE | STILL POSSIBLE | list tests | CAUGHT |
| SC11 | Blocking work on the event path | 2 | 0 | 1 | STILL POSSIBLE | STILL POSSIBLE | R12 | CAUGHT |
| SC13 | Wrong identifier used, or an id not unique in its scope (call_id for approval_id; turn-level for request-level; item id unique only per turn) | 3 | 1 | 0 | STILL POSSIBLE | STILL POSSIBLE | R13 | IMPOSSIBLE |
| SC14 | Feature logic error in a transition (not a state-mechanism class) | 16 | 1 | 0 | STILL POSSIBLE | STILL POSSIBLE | feature tests | STILL POSSIBLE |
| B | Backend / persistence / protocol | 20 | 0 | 7 | n/a | | | |
| L | Layout, style, input layer (bug-classes.md classes 2–11) | 8 | 0 | 1 | n/a | | | |
| N | Not a runtime bug: tests, fixtures, build, docs, copy, reverts | 36 | 0 | 0 | n/a | | | |

STILL POSSIBLE* = the oracle's mechanism is absent from the design, but the
same bug can still be written (see §1.1).

### Examples (3–5 each; ° outside fix set)

- SC1: 641e2e362 (counter reset on widget replacement; UUIDs added), 5ca2007ab
  (usage result of a previous widget), c8949e55c (abort cannot retract a
  queued reply), 1f17a0a04 (voice answers filtered by input generation),
  bf7c87a7b (close notification from an old runtime after resume), ffdf2dadd,
  f046cf35d°.
- SC2a: 1096d6453 (rebuilt widget replays buffered events and then queued live
  ones, so TurnComplete is handled twice), b83105710 (history queued by the
  previous thread repopulates the transcript), 0679e70bf, 2bb2e20d6,
  adca20357° (the bounded replay buffer evicted TurnStarted); issues #11273,
  #12930, #13119, #13252, #20191, #18854, #32536, #37942, #16904, #59875.
- SC2b: e4a202ea5 (draft and queue lived in the single ChatWidget),
  eabb7c91d° (kill buffer lost when the composer was replaced), 5183e1dcb°.
- SC3: 53a1f4c29° / #21894 (`self.thread_id.unwrap_or_default()` used for the
  elicitation reply; still visible at codex-rs/tui/src/chatwidget/tool_requests.rs:298,322),
  8843960ba° (approval ids collide across threads), 6f47fcb0b° (request ids
  reused across threads and connections), 5a440c03f (child MCP startup shown
  in the parent), 27294165d°; issues #18068, #58781, #30714 (unconfirmed).
- SC4a: 6107b9886, c313eb75f, 054128c9d, 7c797c654, 3f62b5cc6, e29eceb75°;
  #37831 (SubAgentActivity cell pushed on both ItemStarted and ItemCompleted;
  the /agent feed, which dedupes by item, was unaffected), #2886.
- SC5a: 44403dd3d (hook row cleared on interrupt/fail paths, not on normal
  completion), 5c18cc0ac, 850da19dc, 4ffeddcbc, 2808a9c34°, 408a77dc1°;
  #16779, #54136.
- SC6a: 333663921 (`set_connectors_enabled` called in some constructors but
  not on resume), bede1d9e2 (footer refresh moved into setters), b08bdd91e
  (collaboration mask copy of model/effort), bc969b651 (overlays keep their
  own request queue), 706f830dc (open popup not resynced), 692989c27,
  5c0fbf349, 05cf8eeb1°; #11025, #15961, #36487, #38304.
- SC6b: ad09c138b (auth.json vs AuthManager), ae057e0bb, 47d9cde20°; #33272,
  #14235, #18742, #35979, #36116.
- SC6c: cc8fd0ff6, 76a9b1167 (the fix itself: "derive ... on demand instead of
  caching state"); #30767.
- SC7: c0a84473a, 5d2702f6b, 4ed8d74aa, d876f3b94; #9811 (a); e1895710a,
  57ba758df (b); baaf42b2e (c).
- SC8: 4d7a5c7c7 (a); 764f3c7d0, 57b98bc4c, ffae97921; #18860 (b).
- SC9: be4203023, f8b6b1db8, 7389eebf4, 964aeda62, 9ed1b7469°; #15842.
- SC10: cb8dc18a6, 4294031a9; #25295 (a); 46b653e73, 47379efd5°,
  576110286°, 1badea29a° (b).
- SC11: 6e8c055fd, c978b6e22; #37735.
- SC13: ddfa032eb, c4b771a16, a3be74143, e8e5108c9°.

## 4. Rules to add to ADR-003 (each named after what makes it hold)

| rule | what | enforced by | sub-classes |
|---|---|---|---|
| R1 | Requests issued by the app return `Reply<T>` with a private payload. The only way to read it is `pending.resolve(reply) -> Option<(Ctx, T)>`, which checks the ticket. `AppCx::spawn` is called only in the shell module: its `then` closure receives `&mut App` (crates/rtwk/src/app.rs:1001-1004) and must only forward to `apply`. Transport events carry the thread's attach epoch, and one ingress router drops events from earlier epochs. | module privacy + `clippy::disallowed_methods` on `AppCx::spawn` outside the shell | SC1 |
| R2 | Per-thread data lives only in `ThreadModel` (draft, queue, status, permissions, pending requests). Per-thread views are built through one `thread_scope(thread_id, ..)` helper that keys the rtwk subtree by ThreadId. | mandatory switch round-trip property test (random state in A, then A→B→A; A unchanged, B shows only B) | SC2a, SC2b |
| R3 | Thread events are applied by `apply(&mut ThreadModel, ThreadEv, &mut ThreadFx)`. Thread-scoped RPCs can only be built through `ThreadFx`, which stamps the thread's own id. Pending requests are keyed by `(ThreadId, RequestId)`. User intents carry their target ThreadId, captured at dispatch. | types (no active-thread access in thread apply) | SC3 |
| R4 | The transcript is an ordered map keyed by `(TurnId, ItemId)` with upsert (started, delta and completed update one entry). One projection from protocol events, and legacy duplicates are dropped there. | type API (no push) | SC4a |
| R5 | `enum TurnState { Idle, Running(RunningTurn{hooks, reviews, prompts, mcp_startup, streams..}) }`. Transient state lives inside the variant, and the end transition takes `RunningTurn` by value. | types + end-path matrix test (complete/interrupt/fail × feature) | SC5a, SC5b |
| R6 | `Remote<T> { Loading, Loaded{value, fetched_at, generation}, Failed }` | types (makes age visible; refresh policy remains product logic) | SC6b2, SC10a |
| R7 | Derived values are functions of the model, never stored. Any cache is a memo on `Versioned<T>`, or is checked by `Model::assert_consistent()`, which the test harness runs after every event. | test harness | SC6c, SC7a |
| R8 | Lifecycles as enums, predicates (`can_submit`, `queue_blocked`) as functions of them; outcomes as enums, not bools | lifecycle × intent table test | SC7a–c |
| R9 | An entity is created or upserted from whichever of response/notification arrives first. Local mutations wait for confirmation (pending op as a model state). | event-permutation property test per feature | SC8a, SC8b |
| R10 | Intents that touch backend-shared or cross-thread state are values (`Intent` enum) in the same ordered stream as backend events. | permutation test interleaving intents and events | SC9 |
| R11 | Lists use rtwk's keyed `SelectionList<K>` (rtwk-data/src/selection_controller.rs:1, "Keyed selection"). The clock enters as model input. | lint `disallowed_methods` for `Instant::now`/`SystemTime::now` in view modules | SC10a |
| R12 | No IO in `apply` or views; IO only in effects run by the shell | lint `disallowed_methods` (std::fs, blocking calls) in apply/view modules; rtwk responsiveness gate | SC6b1, SC11 |
| R13 | One newtype per id kind (`ThreadId`, `TurnId`, `ItemId`, `CallId`, `ApprovalId`, `ReviewId`, `RequestId`); composite keys where an id is unique only in a scope | types | SC13 |
| R14 | App data never enters rtwk local state: `rtwk_framework::Obs` (obs.rs:23) and `rtwk_hooks::use_state` (primitives.rs:44) are allowed only in an allowlist of UI-only modules | `clippy::disallowed_types` | SC6a |

**Corrected by the architecture review** (../reviews/arch-review.md M2, M4;
2026-11-17): SC6a is CAUGHT, not IMPOSSIBLE — R14 is a lint over known escape
routes (hand-written `State`, controllers, `Rc` clones, `TokioApp::follow`),
not a type guarantee; SC4a's live-vs-replayed case (6107b9886, issue #2886)
is CAUGHT — the backend synthesises ids on history rebuild, so a resume
snapshot replaces its turns and a test checks it. Corrected totals: fix set
20 / 33 / 2; issues 8 / 21 / 5 (reports/data updated, report rebuilt). The
paragraph below is the original count.

Totals after the rules (fix set, 55 state-mechanism commits): IMPOSSIBLE 28,
CAUGHT 25, STILL POSSIBLE 2 (SC6b2 1, SC7c 1). Outside the fix set (23):
10 / 12 / 1. Issues (34): 13 / 16 / 5 (SC6b2 4, SC4b 1). As a share of the
508: removed as a class 28 (5.5%), caught by a mandatory test 25 (4.9%), still
possible 2 + feature logic 16. The rest of the 508 is outside this report's
scope (bug-classes.md classes 1–11 and 19).

## 5. Other families on the same sub-classes (quick verdict, before rules)

| sub-class | F2 store/reducer lib (≈R) | F3 signals | F4 entities+notify | F5 actors |
|---|---|---|---|---|
| SC1 stale reply | STILL POSSIBLE (no library handles ids; state-survey.md) | STILL POSSIBLE | STILL POSSIBLE | STILL POSSIBLE |
| SC2a switch | STILL POSSIBLE* | STILL POSSIBLE* | STILL POSSIBLE* | STILL POSSIBLE* (actor per thread) |
| SC3 wrong thread | STILL POSSIBLE | STILL POSSIBLE | STILL POSSIBLE | STILL POSSIBLE (address picked from active) |
| SC4a duplicates | STILL POSSIBLE (data-structure rule, family-independent) | same | same | same |
| SC5a lifecycle | STILL POSSIBLE (family-independent) | same | same | same |
| SC6a copies | STILL POSSIBLE* | STILL POSSIBLE: an untracked read leaves the view stale (state-approaches.md §3) | STILL POSSIBLE: forgotten notify (44 Zed PRs, ADR-003) | STILL POSSIBLE: UI holds snapshots |
| SC6c / SC7a derived | STILL POSSIBLE | STILL POSSIBLE* (memo derives; an untracked read goes stale) | STILL POSSIBLE (observers sync derived) | STILL POSSIBLE |
| SC8b / SC9 ordering | CAUGHT (actions are values) | STILL POSSIBLE | STILL POSSIBLE | STILL POSSIBLE, worse: no order across mailboxes |
| SC10a / SC13 | family-independent (rtwk keyed selection; newtypes) | same | same | same |
| SC11 | STILL POSSIBLE | STILL POSSIBLE, worse: cross-thread read panic, lost `batch` (ADR-003) | STILL POSSIBLE | two-writer races gone inside an actor; UI snapshot lag |

Of the remedies, only the ordering tests depend on the family, and they favour
values (F2/R). Every IMPOSSIBLE in §4 comes from types and lints that any
family could adopt. The families differ mainly in how they fail (F3 untracked
reads, F4 forgotten notify), and those failures fall in SC6a/SC6c/SC7a. For
those sub-classes the plain model with views as functions plus R14 is the only
option that reaches IMPOSSIBLE.

## 6. Variant R (simple hand-written reducer) vs M (handler methods)

**Why R exists.** Every change (user action, backend event, effect result) is
a value of one `Action` enum, and `reduce(&mut Model, Action) -> Effects` is
the only code that mutates. UI handlers only dispatch.

**What it solves.** One entry point (a place for a gate, a log, a trace).
Deterministic tests that interleave user input with backend events and need no
UI. Record and replay of a session as a `Vec<Action>`. No IO inside `reduce`
if effects are values.

**What it brings.** Indirection: each key handler becomes a variant plus a
dispatch plus a match arm (3 places for 1). Enum size: the oracle already has
335 `AppEvent` variants (codex-rs/tui/src/app_event.rs, 2,103 lines), 88
distinct `ServerNotification` variants used and 40 `Op`/`AppCommand`. A flat R
enum covering the user intents of 54 `handle_key_event` components would land
near the ~600 the coordinator estimated. Nested per feature
(`Action::Thread(ThreadId, ThreadAction)`, `Action::Composer(..)`) keeps each
match small. Compile time: not measured. The oracle already compiles a
335-variant enum and a 4,635-line dispatcher (app/event_dispatch.rs).
Splitting the match per feature limits single-function size. Allocations: an
`AppCx::channel` send of a non-zero-sized payload allocates a box per item
(rtwk docs/guide/state.md:43). That is fine for intents and the same cost the
ingress already pays.

**Evidence from Rust apps.**
- Oracle (codex-rs/tui): an R-shaped event bus is already in place.
  `App::handle_event` (app/event_dispatch.rs:35) matches 335 `AppEvent`
  variants sent from 464 `app_event_tx.send` sites. ChatWidget and BottomPane
  still mutate directly in key handlers and through 120 `set_*` setters, and
  every sub-class above occurred. So making changes into values did not remove
  the classes. Where the state lived decided that.
- ratatui component template (templates@5903368):
  `component/template/src/action.rs:5` (15 lines). `App::handle_actions`
  (app.rs:132) passes every action to every component's `update`
  (app.rs:152-153), and each component owns its state. This is R with
  distributed state, the SC6a shape.
- television (d91ab5b): `Action` enum, 480 lines (television/action.rs:8);
  `Television::handle_action` (television.rs:965); loop in app.rs:516.
- spotify-player (8d17ede): `Command` (command.rs:9), `Action`
  (command.rs:96) and `ClientRequest` (client/request.rs:24) are values. The
  state is `Mutex<UIState>` / `RwLock<PlayerState>` / `RwLock<AppData>`
  (state/mod.rs:26-29), written directly by the client task (client/mod.rs:304,
  481, 510; 15 `data.write()` sites). Actions as values coexist with several
  writers.
- atuin-ai (atuin 9a61c7a), the closest app (an agent chat TUI): `Msg` enum
  (crates/atuin-ai/src/tui/app.rs:63, about 20 variants); `update`
  (app.rs:987); the keymap is rebuilt from the model on every update
  (app.rs:59-61, 1206); FSM `#[must_use] handle(&mut self, Event) ->
  Vec<Effect>` (fsm/mod.rs:250-251); 52 test functions in fsm/tests.rs. Stale
  results are dropped per handler by tool id (fsm/mod.rs:899, 1013), not in one
  place. A reducer gives a single place for the check, but the check still has
  to be written there.
- gitui (d7214ec): `InternalEvent` queue, 41 variants (src/queue.rs:86),
  processed in `App::process_internal_events` (app.rs:722); components mutate
  their own state; async results via `update_async` (app.rs:420). That is M
  with a command queue.

**Which removes more as a class.** On this bug history M and R tie for
IMPOSSIBLE: no sub-class becomes impossible because of R alone. R moves SC9 (4
fix-set commits, 1 issue) from STILL POSSIBLE to CAUGHT, and makes SC8b's
permutation test cheaper. M with R1 is weaker on SC1 if spawn results write to
the model in the `then` closure. With R1 (spawn only in the shell) it is
equal. The mix plus R10 matches R on SC9 with a smaller enum
(only intents that touch shared state; not counted).

**"Every change is a value": useful for us?**
- Event log: yes, for intents plus backend events plus effect results. A debug
  recorder on the single channel gives a repro file for race reports. The mix
  with R10 records the same.
- Deterministic replay for parity: partly. The parity harness compares the
  oracle and the port from terminal input and app-server traffic (keys and
  JSON), not from Actions, because the oracle has no Action log of the port's
  shape. Action replay serves the port's own regression and race tests, not
  oracle parity.
- One place for the request-id check: R1 gives that in the mix and in M
  (every async result comes through `apply` via the shell). R adds nothing
  further. atuin-ai shows that a reducer alone does not centralise the check.

## 7. Decisions for the person

1. Accept that ADR-003 as written does not meet "remove as a class" for any
   sub-class, and whether to add R1–R14, which bring 28 of 55 fix-set state
   commits to IMPOSSIBLE and 25 to CAUGHT.
2. R, M or the mix with R10: the evidence favours the mix plus R10. Full R
   buys testability for local UI intents that rtwk already owns, at a larger enum surface (every UI intent a variant).
3. Whether to correct the ADR's "~22 points" figure: about 11% of the 508 are
   state-mechanism bugs, and the subject filter misses many more (106
   candidates outside it, 23 of 24 sampled were state bugs).

## 8. Commands run

- `git log --no-merges --format='%h|%ad|%s' --date=short -- codex-rs/tui codex-rs/tui2` (3,612) + bug-classes.md regex (508) + classify.sh (cls.txt)
- `git log -1 --format=%b <h>`; `git show --format= -U1 <h> -- codex-rs/tui*` for message-less commits
- `git log --no-merges --name-only --format='@%h' -- codex-rs/tui codex-rs/tui2` + dirs.awk (files_top, areas)
- outside-fix-set search: subjects matching ignore stale|dismiss|avoid duplicate|keep/preserve/clear/scope/route/reset/restore/retain/refresh ... not in the 508 (106)
- `gh api search/issues q='repo:openai/codex is:issue label:TUI in:title "<kw>"'`, 15 keywords; `gh issue view <n> --json body,state,createdAt`
- `gh api repos/openai/codex/commits/<full sha>` for all 532 hashes (upcheck.sh)
- shallow clones of ratatui/templates, aome510/spotify-player, atuinsh/atuin, alexpasmantier/television into src/ (read only); gitui from the state-survey clones
- scripts/mkcsv.py builds commits.csv, issues.csv, fix-set.csv, subclass-counts.csv from cls_manual.txt, supp_manual.txt, iss_manual.txt, subclasses.tsv
