# core-review-b: core features, keymap and kit, read for refactoring

Reviewer: codex-reviewer (Opus), 2026-11-24, read-only on `claude/tui-rtwk` at 8cf058010.
Scope: `codex-rs/tui-rtwk/core/src/{features/, keymap.rs, keymap/, kit/}` (~28.4k lines), their
callers in `src/keymap`, `src/pages/session/composer/vim.rs`, `src/features/mod.rs`,
`core/src/commands/dispatch.rs`, `host/src/feeds.rs`, and the rules in `tests/rules/*.rs`.
Paths below are relative to `codex-rs/tui-rtwk/` unless they start with `codex-rs/tui/` (the oracle).
Method: read the ADRs (003, 006, 012, 013), the budgets and the rules, then each slice's
`impl Feature`, the keymap lookup path per keystroke, the vim engine and the history walk. Static
evidence only. No benchmarks were run; the allocation counts come from reading the code.

## (a) Verdict

1. The slice framework holds: no slice names `CodexApp`/`ThreadModel`, no slice reaches another
   slice's path, no adr013 ratchet is open, and features never edit `fake.rs` (`Command::Feature`).
2. The rules have measurable holes. The prose rule misses multi-line `format!`/`write!` (37 sites,
   23 of them user-visible in `keymap/resolve.rs`). `ThreadView::parts()` lets a slice read every
   other slice's part. Thread RPCs written inside `rpc_list!` are not seen. The `feature!` shape is not enforced.
3. Seams: a slice that owns a command still edits `dispatch.rs` because `OPENS` is one command with
   no gate and no args (the `/usage` gate of w2-status, `/recap`, `/mcp`, `/side`, the nine
   settings openers). `host_feed_list!` is still a hand-written list.
4. Hot path: every keystroke checks an action by a linear scan of `fixed::TABLE` plus a SipHash on
   an `&str` id plus a `Vec` allocation, about 70 times for an unbound key in Vim Normal mode. The
   oracle walks a borrowed slice. History search decodes every entry again on every visit.
5. The best-paying refactors are small: a dense `ActionId` index with borrowed bindings, a
   token-based prose rule, an `OPENS`/gate door, decoded history entries, and `feature!` everywhere.

## (b) Findings

Size is the number of lines touched. Risk is the chance of breaking behaviour. Clash: **C** =
w2-commands (`features/{daemon,feedback,recap,security_setup,side,small_cmds,windows_sandbox}`),
**R** = rtwk-repin-5 (Cargo + selection), `-` = none.

| id | file:line | problem | why it matters | proposed refactor | size | risk | rules test | clash |
|---|---|---|---|---|---|---|---|---|
| K1 | `core/src/keymap/resolve.rs:331-342,377-405`; `core/src/keymap/fixed.rs:800-805`; `src/keymap/mod.rs:153-161` | `pressed()` scans the 110 rows of `fixed::TABLE` (`ActionId` eq = `&str` compare), then `singles(id)` SipHashes the `&str` and returns a new `Vec` | perf: runs per key per action. Vim `read` (`src/pages/session/composer/vim.rs:245-257`) calls `fires` for up to 34 actions, then `starts_chord` calls `bindings()` 34 more times: about 70 allocs and 70 hashes for an unbound key. The oracle does `[KeyBinding]::is_pressed` on a borrowed slice (`codex-rs/tui/src/key_hint.rs:212-215`) | give `ActionId` a dense `u16` index from its const table row (catalogue and fixed in one space). `Keymap::singles`/`bindings` return `&[Bound]` or an iterator; `fixed::bindings_of` indexes. The name stays for the config path | 150 | low | yes: a test that `pressed` allocates nothing (counting allocator in `core/tests/keymap.rs`) | - |
| K2 | `core/src/features/settings/keymap.rs:135-140`; `core/src/keymap/resolve.rs:470-477` | `id_of(context, name)` scans `keymap.actions()`, which builds a `Vec<Binding>` for every row it passes | perf: O(n^2) allocs. `picker()` calls it twice per action (about 30k Vecs per build, run on the host at every config change). `matching()` calls it 173 times per key in the `/keymap debug` inspector (`src/features/settings/keymap_editor/debug.rs:110`) | `ActionId::from_config(context, name)` through the catalogue's `index_of`; `actions()` yields `&[Bound]` | 25 | low | yes: test that every `ACTIONS` row but `global.leader` resolves (today a mismatch silently shows "unbound") | - |
| K3 | `core/src/keymap/resolve.rs:586-587,616,695,725-727,878-879` | `ActionId::new(Context, "name")` written inline outside the tables | bug class: a typo compiles, and a reserved-key or overlap rule then matches nothing | use the `actions::` constants; make `ActionId::new` `pub(super)` to `catalogue`/`actions`/`fixed` | 15 | low | yes: `ActionId::new(` only in `keymap/{catalogue,actions,fixed}.rs` | - |
| K4 | `core/src/keymap/resolve.rs:129-290` | `KeymapError`'s `Display` is English prose in core (28 `write!`/`write_str`), carried as `Wish::KeymapConflict { error: String }` (`settings/mod.rs:94-104`) | ADR-010: core holds no words. The rule misses it (H1) | `KeymapError` stays typed and crosses as the value; the words go to `src/features/settings/keymap_editor/strings.rs` and the startup error table | 200 | med (golden text) | yes, once H1 is fixed | - |
| H1 | `tests/rules/layers.rs:135-163` | `core_sources_hold_no_user_visible_prose` reads only `format!("` on one line. It misses `format!(\n"..."`, `write!`/`writeln!`/`write_str`, and plain `const &str` literals | 37 multi-line sites in core today: `keymap/resolve.rs` (23), `feedback/mod.rs:484`, `screens/agents/refresh.rs:345,372`, `screens/onboarding/session.rs:30`, `screens/startup/launch.rs:206`, ... | scan the tokens with `adr013.rs::code_tokens`: every string literal argument of `format!`/`write!`/`writeln!`/`write_str`/`concat`, with a ratchet file. Model prompts (`side/policy.rs`, `recap/prompt.rs`, `rename/title.rs`) are exempt by name | 80 rules + ratchet | low | it is the rule | - |
| H2 | `core/src/features/cx.rs:298-300`; `core/src/features/small_cmds/mod.rs:38-43` | `ThreadView::parts()` hands any slice every slice's `ThreadPart`; `parts.goal.x` written in another slice is a field access, not a path, so `features_are_named_only_through_the_list` passes it | ADR-012 "a slice does not read another slice's state" is unenforced (0 violations today, so the rule is cheap now) | rule over `core/src/features/<f>/`: `parts()...<g>` / `.parts.<g>` / `states.<g>` with g != f is refused. `small_cmds::daybreak_of` reads its own part, so it stays | 40 rules | none | yes (new rule) | - |
| H3 | `tests/rules/layers.rs:418-464`; `core/src/features/side/mod.rs:495` | `thread_rpcs_only_in_threads` is a deny list of names. `ClientRequest::ThreadInjectItems` (side) is in neither list, and an `rpc_list!` line writes `=> ThreadX(...)` without the `ClientRequest::` prefix, so a lifecycle RPC whose params come from a helper is not seen | ADR-003: one writer and thread RPCs in `ThreadFx` is enforced only for listed names | classify every `ClientRequest::Thread*\|Turn*` variant (lifecycle / feature / read-only, e.g. `ThreadList`), refuse an unclassified one, and also scan `=> Name(` inside `rpc_list!` | 60 rules | none | yes | - |
| H4 | `tests/rules/wire.rs:92-108` | `HAND_WRITTEN` is per file, not per count: `feedback/mod.rs` makes 4 requests by hand and can add more. `thread_usage` has an `rpc_list!` but is missing from `GENERATED` | the ratchet does not ratchet | count the lines per file, like `hub_budget.txt`; add `thread_usage` | 20 | none | it is the rule | C (feedback, side, windows_sandbox own the lines) |
| H5 | `test-mods/registry.rs:38`; `core/src/features/{goal,rename,small_cmds,mcp_inventory,plugins,thread_usage}/mod.rs`, `settings/mod.rs:996-1021` | 7 of 18 slices write `impl Feature for` by hand (27 `Never<Self>` lines plus one-line `wire`/`decode`). The ADR says "a slice is written with `feature!`". `FEATURE_IMPL` accepts both | porting speed: each associated type the framework adds (`Target`, `Wish`, `Interest`, `Event`) was a hand edit in every one of them (goal 6, rename 6, small_cmds 5, settings 5 framework commits) | move them to `feature!`; add a `rpcs: Ask;` shorthand that writes `wire`/`decode` from the slice's `rpc_list!` | 7x30 | low | yes: registry refuses an `impl Feature for` outside `feature! {` | C (small_cmds) |
| S1 | `core/src/commands/dispatch.rs:399,422-424,435-437,444,447-453,457,528,536-552` | `Feature::OPENS` is one `(SlashCommand, Target)` with no gate and no args, so the hub keeps per-feature cases: `Side\|Btw` review gate, `Recap` needs a thread, `Mcp` parses its own args, `/usage` needs auth (w2-status adds `command != Usage \|\| cx.has_backend_auth` beside `target_of`), settings' nine openers and `/keymap debug` go through `Target::Settings` (also `src/features/mod.rs:183-186`, `src/pages/session/actions.rs:264`). The `Feedback` arm at :444 is dead (`target_of` answers first) | porting speed: each W2 command task edits this file and `core/tests/commands.rs` (`land w2-a fix` 2ed379c64) | door: `const OPENS: &[(SlashCommand, Self::Target)]`, `const NEEDS: &[(SlashCommand, Need)]` (`Need::{Thread, BackendAuth, NotInReview}`, pure data that `dispatch_command` checks generically), and `fn open_args(command, args) -> Result<Target, UsageKey>`. Settings then leaves `Target::Settings` | 250 (hub) + 10 per slice | med | yes: `dispatch.rs` names no `SlashCommand` that some `RUNS`/`OPENS` owns (count ratchet in `hub_budget.txt`) | C (recap, side touch `NEEDS`) |
| S2 | `core/tests/commands.rs:1126-1150` | `every_screen_opening_command_names_its_target` lists feature-owned commands by hand (`/feedback`, `/mcp` at :981) | each move of a command into a slice breaks a shared test table | derive the row of an owned command from `target_of`, or drop it there: `core/tests/slash_door.rs` already proves the owners' routes | 20 | none | yes: the table names no `features::target_of` | C |
| S3 | `host/src/feeds.rs:77-84` | `host_feed_list!` is a hand-written list of 6 slices; "Registration by file" (ADR-012) says no list is | two slices adding a feed edit adjacent lines | `host/build.rs` writes the list from `host/src/feeds/*.rs` with `test-mods/registry.rs` (the host already has a `build.rs`) | 40 | low | yes: the registry test (`the_lists_are_generated_from_the_files`) extends to it | - |
| S4 | `core/src/features/mod.rs:951-955`; `tests/rules/layers.rs:743` | `daybreak_of`, a slice selector written by hand in the list file, names `small_cmds`. The rule exempts the whole file, though its own comment (:160-163) says "no line of this file names it" | a hole the next selector widens | move it to `app/notices.rs` through a generated `features::selector`, or exempt only the `include!` | 10 | none | yes: the exemption covers the generated include only | C (small_cmds) |
| P1 | `core/src/kit/history.rs:658-668` vs `codex-rs/tui/src/bottom_pane/chat_composer_history.rs:775-783` | `entry_at_cached_offset` runs the mention codec (`HistoryEntry::persisted`) on every visit; the oracle caches the decoded entry | perf: Ctrl+R decodes and lowercases every cached entry it walks, again on every keystroke | `data::history::Store` keeps `Rc<HistoryEntry>` decoded once at `apply` (keyed by the at-mention flag), or `PromptWalk` memoises by offset | 40 | low | no (bench in `perf/`) | - |
| P2 | `core/src/kit/vim/mod.rs:236-273`; `engine.rs:381,913,948,1043`; `src/widgets/composer_draft/vim.rs:301-307` | `VimText::elements()` returns a new `Vec` and `is_command_target` calls it once per candidate: per search match, per grapheme of an `f`/`t` scan | perf: allocations grow with the matches in a long draft | `fn elements(&self) -> &[Range<usize>]` (the draft holds them), or fetch once per command | 40 | low | no | - |
| P3 | `core/src/features/plugins/state.rs:171-187`; `catalog.rs:1026-1203` | `State::tabs()` dedupes, sorts and builds every row and tab again on each call; the Home page calls it in `build` (`src/features/plugins/pages.rs:349`, `lists.rs:404`) and again per toggle (`pages.rs:253`) | perf: one rebuild per frame that changes the page; small catalogues hide it | keep the tabs in the listing, rebuilt on the reply/fact (`Remote` revision), or `ViewCx::memo` on `(rev, Rc<Catalog>)` | 40 | low | no | - |
| D1 | `core/src/features/settings/mod.rs:286-303,793,840,924` | `State::edits` is a log of every dialog request, appended and never read outside tests (`core/tests/settings.rs:724`, `tests/sheets/settings.rs:554,806`) | unbounded growth in production state, compared on every `FeatureStates` equality | tests read the `AppFx`/`Out` the step produced; drop the field | 30 + tests | low | yes: core state fields read only under `#[cfg(test)]` are flagged (adr013-style scan) | - |
| D2 | `core/src/features/settings/keymap.rs:183,214,230-297`; `src/features/settings/keymap_editor/lists.rs:70,110` | the `/keymap` picker is built in core with words (`"All"`, `"Common"`, `"Customized (N)"`, `"Unbound (N)"`, `"Debug"`, `"Custom"/"Default"` in the search text, `" (inactive)"`, `action_label`'s "Change Where to Run"). `binding == "unbound"` is a sentinel string both crates compare | ADR-010 and ADR-013 (what is drawn is the UI's): the host sends a presentation catalogue as a fact (`host/src/feeds/settings.rs:76`). Stringly typed | core keeps `Keymap` (a fact the app holds) and `ACTIONS` as data. The tabs, labels and search text are a UI derivation in `src/features/settings/keymap_editor/`; `binding: Option<String>` | 300 | med (goldens) | H1 bites part of it | - |
| D3 | `core/src/features/settings/mod.rs:78-89,94-104` (14 `context: String, action: String` fields) | keymap edits, notes and wishes carry an action as two `String`s and look it up again (`keymap::action(&context, &action).is_none()` -> `KeymapUnknown`, :823) | stringly typed. A wrong pair is a runtime note, not a compile error | carry `ActionId` (it is `Copy`); the config pair is `ActionId::config_path` | 120 | low | no | - |
| D4 | `core/src/features/{settings/mod.rs:47,plugins/state.rs:143,feedback/mod.rs:544,side/mod.rs:272,small_cmds/mod.rs:377,goal/note.rs:66,mcp_inventory/mod.rs:294}` (+3 in screens) | 10 copies of "the server's text of an `RpcError`", each with its own fallback for `Mismatch`/`Provider` (`"unexpected answer"`, `""`, `{:?}`) | duplication and inconsistent failure text | `RpcError::server_text(&self) -> Option<&str>` in `rpc.rs`; the slice picks its fallback word in the UI | 40 | low | no | C (feedback, side, small_cmds) |
| D5 | `core/src/features/feedback/mod.rs:573-596` | when `FsCreateDirectory`/`FeedbackSave` params fail to build, `wire` sends a real `config/requirements/read` as a sentinel so the reply fails to decode | defect class: an RPC the user never asked for goes to the server. The framework gives `wire` no error path although `RpcError::Prepare` exists (`rpc.rs:26,163`) | `Feature::wire -> Result<ClientRequest, RpcError>` (and a fallible `rpc_list!` line); the app answers `Prepare` locally | 60 | low | yes: `ConfigRequirementsRead` named only by its owner | C (feedback) |
| D6 | `core/src/features/feedback/mod.rs:328,373-374,484-486` | `Note::Failed(String)` with English text built in core | ADR-010 | typed `Failure` note; the words in `src/features/feedback/strings.rs` | 40 | low | H1 | C |
| D7 | `core/src/features/cx.rs:88-102,144-176`; `core/src/threads.rs:2068-2075` | `AppFacts::unknown()` gives default `TuiSettings`/`Keymap`/`Access` where a caller forgot `with_keys`/`with_access`/`with_facts` | latent stale-fact bug: a thread step built without facts names the default key in a hint, not the user's (both app paths stamp them today: `app/features.rs:23-52`, `app.rs:3232-3241`) | make them arguments of `AppView::new`/`ThreadFx::new`; tests use a `for_test` constructor | 40 | low | yes: `AppFacts::unknown` named only in `#[cfg(test)]` | - |
| N1 | `core/src/kit/vim/engine.rs:158,1083`; `kit/vim/history.rs:101`; `kit/mentions/popup.rs:54`; `features/settings/catalog.rs:405`; `features/settings/keymap.rs:118`; `features/windows_sandbox/mod.rs:411` | `pub fn`s no file calls (`uses_insert_cursor`, `search_highlights`, `can_undo`, `set_candidates`, `is_unbound`, `binding_spec`, `prompt_for_auto`) | dead code | delete, or wire them in where the oracle has the behaviour (`search_highlights`: the draft's search highlight) | 80 | none | no (`pub` in a lib hides it from `dead_code`) | C (windows_sandbox) |
| N2 | `docs/rtwk-port/architecture.md` ADR-012 recipe ("Files", "A slash command stays in ... `SessionIntent`") | the recipe still lists `wire.rs` (banned by `wire_is_generated`) and routes a command through `SessionIntent`/`app/session.rs` (the slash door replaced it) | porters copy the recipe | update the recipe with `rpc_list!`, `RUNS`, `feature!` | 15 doc | none | no | - |
| N3 | `core/src/keymap/fixed.rs:276-797` | one `TABLE` that 13 tasks edited in 4 days (adr013-keys 1-5, debt-b9, b11, w2-commands...) | merge friction (append conflicts) more than a bug | per-area files `keymap/fixed/<area>.rs` collected by the registry, or left as is if K1's index needs one table | 60 | low | no | C |

## Answers to the questions

**1. Architecture holes.** ADR-003 holds in the family: the rules `features_hold_no_app` and
`feature_slices_ask_only_through_the_outbox` bite, and `ThreadCx` keeps `fx` private. What no
rule catches: H2 (cross-slice part reads), H3 (unclassified thread RPCs, `rpc_list!` lines), H1
(prose), H5 (two slice shapes), S4 (the exempt list file), D7 (silent default facts). ADR-013:
no ratchet is open (`tests/rules/adr013_ratchets/` is empty). The semantic half (drawn vs domain)
still finds D1 (a test log in state) and D2 (a drawn catalogue sent as a host fact). Slices are
not uniform. Eleven use `feature!` and seven write the impl by hand. Three make requests by hand
(feedback, side, windows_sandbox). Settings opens through a hub `Target`, while every other sheet
opens through `OPENS`.

**2. Seams.** A slice still edits shared files in these cases:
- `dispatch.rs` for gates and args (S1);
- `core/tests/commands.rs` for routes (S2);
- `host/src/feeds.rs` for a feed (S3);
- `keymap/catalogue.rs`, `keymap/actions.rs` and `keymap/fixed.rs` for a key. This is oracle data
  and partly unavoidable; N3 covers it.

It no longer edits `features/mod.rs`, `src/features/mod.rs` (generated), `fake.rs`/`live`
(`Command::Feature` is generic) or `app/session.rs` (slash door). Doors: the S1 data `NEEDS` +
`OPENS` list, and a generated feed list.

**3. Structure.** Large files that hold one job, where size follows from the oracle:
`plugins/catalog.rs`, `kit/vim/engine.rs`, `keymap/resolve.rs` and `kit/history.rs`.
`settings/keymap_actions.rs` is generated. Misplaced code:
- `settings/keymap.rs` (D2) builds the drawn `/keymap` catalogue in core, and the host calls it;
- `KeymapError` words (K4) and feedback's words (D6) live in core.

`settings/mod.rs` mixes nine dialogs. Splitting it pays only after D1 and D3, so this review
proposes no split for its own sake.

**4. Duplication, dead code, strings, hot paths.**
- Duplication: D4 (10 copies of the `RpcError` text).
- Dead code: N1, plus the dead `Feedback` arm in S1.
- Strings: D2 (`"unbound"`), D3 (action pairs), K3 (inline `ActionId` names).
- Hot paths: K1 (per keystroke), P1 (Ctrl+R), P2 (Vim search and find), P3 (plugin tabs), K2
  (`/keymap`). K1 is the one in every key of every page. The oracle's cost there is zero allocs.

## (c) Top 5 by payoff and cost

1. **H1, the token-based prose rule, with a ratchet** (80 lines, no behaviour change). It catches
   37 sites now and every multi-line `format!` later, and it unblocks K4 and D6 as ratchet steps.
2. **K1 + K2 + K3, a dense `ActionId` index and borrowed bindings** (about 190 lines, core only, low
   risk). It removes about 70 allocs and hashes per Vim key, makes the `/keymap` build O(n), and
   lets a typo of an action fail to compile.
3. **S1, the `OPENS`/`NEEDS` door with the dispatch ratchet** (about 250 lines). It ends the W2 and
   W3 edits of `dispatch.rs` and `core/tests/commands.rs`. Land it after w2-commands merges
   (recap and side gain a `NEEDS` line).
4. **H2 + H3, the cross-slice part rule and the classified thread-RPC rule** (about 100 rules
   lines, 0 violations to fix today). They close ADR-012's "not done" and ADR-003's RPC list
   before W3 adds slices.
5. **H5 + D5, `feature!` for the seven slices, the `rpcs:` shorthand and a fallible `wire`**
   (about 270 lines). One shape for every slice, with no sentinel RPC. The small_cmds and feedback
   parts wait for w2-commands.

Not proposed: splitting `settings/mod.rs`, `plugins/catalog.rs`, `kit/vim/engine.rs` or
`keymap/resolve.rs` by size. No finding clashes with rtwk-repin-5 (no Cargo or selection code is touched).
