# code-layout: every .rs file of codex-rs/tui-rtwk as a Flutter `lib/` (read only, no code moved)

Task `code-layout`, 2026-11-19, base d9e2181c4. Source: `git ls-files 'codex-rs/tui-rtwk/**/*.rs'` (285 files,
~90.8k lines), each file's module doc and pub items read (not whole files). Target names follow architecture.md §1
Amendment 2026-11-19b: 5 pages, modal sheets, widgets. Lines in brackets are file sizes. "stub" = a 36-98 line
placeholder written by the skeleton, no behaviour.

## 1. What is muddled now (why the tree is wrong)
1. Everything UI lives under `src/screens/<dir>`, one directory per oracle *slot* (S10..S26), not per Flutter
   thing: pages (agents, resume, usage), sheets (approvals, select, settings, pets, dialogs, status, voice),
   chat parts (cells, composer, popovers, transcript, backtrack, sidebar, ui_tree) and the 11 stub dirs sit side by side.
2. `core/src/screens/<dir>.rs` mirrors that 1:1 through `screen_list!`/`core_dirs!` (`core/src/screens/mod.rs:56`),
   and `tests/layers.rs:215` enforces "every screen dir has a core half and a view half". The coupling is the
   oracle's slot list, which Amendment 2026-11-19b replaced.
3. Protocol mapping is mixed with feature logic: `core/src/wire.rs` (our commands <-> ClientRequest) is apart, but
   `screens/agents/{wire,sources,refresh,notifications}.rs` import the protocol 28/8/8/5 times and also merge pages,
   order rows, and apply replies. Every `screens/<dir>.rs` stub also imports `codex_app_server_protocol`.
4. `src/shell/*` (services) sits in the UI crate under `src/` beside views; `src/ui/` is a grab bag (shared widgets +
   the app view + debug + controllers + page keys).
5. Tests of both crates live in 3 layouts (`tests/oracle/*` 60 files, `tests/*.rs`, `core/tests/*.rs`).

## 2. Target tree (names are final; "(stub)" dirs are not created until a slice builds them)
```
codex-rs/tui-rtwk/                crate `codex-tui-rtwk` (UI + services), lib + tests
  src/
    app/        entry and root: app.rs (UiApp) run.rs view.rs (Navigator+providers) theme.rs start_page.rs debug/
    l10n/       shared string table + Msg plumbing (per-feature tables stay next to their widgets: strings.rs)
    keymap/     (reuse-map keymap slice, still empty)
    pages/      ONLY the five Navigator routes
      session/    session_page.rs (ex chat/home.rs) + its parts: transcript/ composer/ popups/ cells/ (status card
                  is a cell) status_line.rs, footer, backtrack/ sidebar/ voice/ ui_tree/ (stubs)
      agents/     agents_page.rs list.rs details.rs header.rs footer.rs help.rs tabs.rs path.rs dialogs.rs (page-private sheets) usage.rs strings.rs
      resume/     resume_page.rs chrome.rs layout.rs strings.rs
      usage/      usage_page.rs strings.rs
      startup/    startup_page.rs (onboarding steps: signin, trust, migration, import as steps) strings.rs
    sheets/     showDialog / bottom sheet content: approvals/ select/ settings/ pets/ shortcuts.rs warnings.rs feedback.rs dialog_host.rs
    widgets/    shared: session_list options page_keys(footer) age dots measure status_line thread_ui(controllers)
    services/   no widget, owns IO/tasks: shell.rs (pump, R1) backend/{fake, reconnect, rpc} agents/{search,version,workspace} cli_help.rs
  core/         crate `codex-tui-core` (no rtwk, no tokio, no IO): models + apply
    src/ lib.rs app.rs ids.rs rpc.rs
         session/    threads.rs thread_color.rs commands/{mod,dispatch,fuzzy}.rs
         agents/     model.rs center.rs details.rs discovery.rs refresh.rs reads.rs update.rs usage.rs notifications.rs carried.rs
         approvals/ resume/ usage/ settings/ select/ pets/ dialogs/ status/   one module per feature, ex screens/<dir>.rs
         protocol/   mod.rs (ex wire.rs) agents.rs (ex agents/wire.rs + decode half of sources/refresh/notifications) threads.rs
         fixtures/   builders.rs script.rs sandbox.rs (keep: they build protocol payloads, may use protocol/)
  host/         crate `codex-rtwk` (bin): main.rs build.rs + services that link codex-core:
    src/ in_process.rs cloud.rs target.rs facts.rs workspace.rs   (renamed under `connectors/`; see §3)
  tests/        flat per concern (§7)
```
Why these five folders and not more: Flutter's `lib/` is `app/ pages/ widgets/ services/ models/`; sheets are a
Flutter `showModalBottomSheet` bodies and deserve their own folder only because there are 7 of them here.

## 3. Crate boundaries (decisions)
- **core stays a crate.** The Cargo manifest is the layer wall: no rtwk, no tokio, lints ban IO
  (`tests/layers.rs:47 core_manifest_has_no_ui_or_runtime_dependency`). `core/protocol/` is a *module* of core, not a
  crate: protocol types are already core's dependency (`codex_app_server_protocol`), and a crate buys a wall only
  against feature code importing the protocol, which a layers test (`feature modules name no codex_app_server_protocol`)
  does for free after the split. Revisit a `codex-tui-protocol` crate only if a second consumer appears.
- **host is a crate, keep it, and keep it thin.** It exists because linking codex-core costs >4 GB of ld memory
  (`host/build.rs`) and the library must not link the server (`host/src/in_process.rs`). Hence it is the *connector
  layer*: only what names codex-core (in-process server, cloud, target, facts, workspace). Rename `host/src/` files into a
  `connectors/` folder is optional; the crate and bin name stay.
- **services/ is a folder of the UI crate, not a crate.** `src/shell*` names tokio and the backend trait but no widget
  and no codex-core. Making it a crate (`codex-tui-services`) would let the compiler enforce "no widget" like core does for
  "no UI"; cost: the shell uses `UiApp` (UI state) — see §6, so a crate needs `UiApp` to move first. Not now.
- Dependency direction after the move: `host -> ui crate (services, pages) -> core`; `pages/sheets/widgets` import
  `core` models and `services` only through `app/` intents (today's rule: views never name shell, tokio, protocol).

## 4. UI crate `src/` (current -> target)
| current | target | role | notes |
|---|---|---|---|
| src/lib.rs | src/lib.rs | crate root, `Cli` | mod list changes: app, l10n, keymap, pages, sheets, widgets, services |
| src/app.rs [140] | app/app.rs | `UiApp {core, ui}` | rules.rs:105,127 read it by path |
| src/run.rs [74] | app/run.rs | composition root | |
| src/ui/view.rs [84] | app/view.rs | MultiProvider + keyed Navigator | rules.rs:122 reads by path |
| src/theme.rs [138] | app/theme.rs | ThemeData styles | rules.rs:479 allows literal colours only here |
| src/ui/start_page.rs [61] | app/start_page.rs | page the CLI starts on (`agents`, `resume`) | tiny; may fold into view.rs |
| src/ui/debug_menu.rs [202] | app/debug/menu.rs | `--debug-screens` menu | each `debug_entries` in a page/sheet stays there |
| src/ui/seeded.rs [124] | app/debug/seeded.rs | seeded fixture screen | debug only |
| src/ui/presenter.rs [159] | sheets/presenter.rs | opens a dialog for the request-queue head (ADR-002) | needs the approvals sheet, owned by it |
| src/ui/mod.rs | (deleted) | | |
| src/ui/age.rs [50] | widgets/age.rs | relative age value (`Age`) | duplicate: see §8 |
| src/ui/dots.rs [29] | widgets/dots.rs | ellipsis-after-blank (N1) | |
| src/ui/measure.rs [46] | widgets/measure.rs | wrapped row count (G38) | |
| src/ui/options.rs [214] | widgets/options.rs | numbered option rows of a dialog | used by sheets (select/approvals) |
| src/ui/page_keys.rs [216] | widgets/page_keys.rs | `KeyHintsBar` footer + page key map | |
| src/ui/session_list.rs [210] | widgets/session_list.rs | list shared by agents and resume | |
| src/ui/status_line.rs [199] | widgets/status_line.rs | configurable status line | chat part; moves to pages/session/ when built |
| src/ui/thread_ui.rs [47] | widgets/thread_ui.rs | per-thread composer controllers (R2/R14) | rules.rs:426 allowlist |
| src/l10n/mod.rs, strings.rs [109,194] | l10n/ | shared strings | |
| src/keymap/mod.rs [1] | keymap/mod.rs | empty | |
| src/screens/mod.rs [93] | (deleted) | `view_dirs!` of the 1:1 list | §6 |
| src/screens/chat/home.rs [636] | pages/session/session_page.rs | the skeleton chat page (`Chat`, `ChatState`) | split: transcript list, composer, search key, warnings opener |
| src/screens/chat/{keys,open,popup}.rs [61,82,79] | pages/session/{keys,open,composer/autocomplete}.rs | chat actions/chords, route opener, popup lists | `popup` is the composer autocomplete |
| src/screens/chat/{mod,strings}.rs | pages/session/{mod,strings}.rs | | |
| src/screens/agents/mod.rs [1101] | pages/agents/agents_page.rs | `Screen`/`ScreenState`, route | split: page + `ScreenState` (modes, keys) |
| src/screens/agents/view.rs [1224] | pages/agents/{list,details,header,footer,row}.rs | RowsList, details, header, footer, controls | one widget per file by `fn header/list/details/footer/entry` |
| src/screens/agents/{tabs,path,help,dialogs,usage,strings}.rs | pages/agents/ same names | filter strip, path truncation, help, 2 sheets, usage lines, strings | `dialogs.rs` stays page-private |
| src/screens/resume/mod.rs [541] | pages/resume/resume_page.rs | resume/fork page | |
| src/screens/resume/{chrome,layout,strings}.rs, list.rs | pages/resume/ same; `list.rs` deleted | chrome by width, geometry, strings | list.rs is the skeleton's minimal list (dead after the slice, verify) |
| src/screens/usage/{mod,strings}.rs [470,149] | pages/usage/usage_page.rs, strings.rs | /usage dashboard | |
| src/screens/{onboarding,startup,import}/{mod,strings}.rs (stubs) | pages/startup/startup_page.rs | one page with steps | three slot stubs collapse to one page |
| src/screens/approvals/{dialog,present,dismiss,mod,strings}.rs | sheets/approvals/ same | request dialog, option/header builder | `dismiss.rs` is the skeleton's dialog: dead once `dialog.rs` shown, verify |
| src/screens/select/{dialog,mod,strings}.rs | sheets/select/ | model/permissions picker | |
| src/screens/settings/{mod,strings}.rs [70,191] | sheets/settings/ | settings dialogs (view is stub, strings real) | |
| src/screens/pets/{mod,strings}.rs [390,68] | sheets/pets/ | /pets picker | |
| src/screens/dialogs/{dialog,shortcuts,warnings,feedback,mod,strings}.rs | sheets/{dialog_host,shortcuts,warnings,feedback,strings}.rs | small dialogs, one host | |
| src/screens/status/{card,format,mod,strings}.rs | pages/session/cells/status_card.rs, format -> core | /status is a history cell (2026-11-19b) | `format.rs` duplicates core, §8 |
| src/screens/{backtrack,cells,composer,popovers,sidebar,transcript,ui_tree,voice}/{mod,strings}.rs (stubs, 59/36) | pages/session/{...}/ created when built; voice -> sheets/voice | chat parts | delete placeholders now |
| src/shell.rs [217] | services/shell.rs | task pump, command queue, effects (R1) | rules.rs:75-100 by path; the only `disallowed_methods` allow |
| src/shell/fake.rs [508] | services/backend/fake.rs | scripted test backend | test support in `src`; `#[cfg(any(test, feature="fake"))]` or a dev module, verify users |
| src/shell/reconnect.rs, rpc.rs [62,124] | services/backend/{reconnect,rpc}.rs | reconnect backoff, RPC send | `rpc.rs` names the protocol (2) |
| src/shell/{search,version,workspace}.rs | services/agents/{search,version,workspace}.rs | history-search debounce, version words, machine facts | `version.rs` holds user words (strings), check adr010 |
| src/shell/help.rs [10] | services/cli_help.rs | `--help` text | tiny |

## 5. core `core/src/` and host
| current | target | role | notes |
|---|---|---|---|
| core/src/lib.rs, app.rs, ids.rs, rpc.rs | same | model root, ids (R13), RPC table | `rpc.rs` is request book-keeping, not protocol: stays |
| core/src/threads.rs [474] | session/threads.rs | ThreadModel + apply | imports protocol 5x: its decode half to protocol/threads.rs; rules.rs:178 path |
| core/src/thread_color.rs [17] | session/thread_color.rs | colour slot of a thread | |
| core/src/commands.rs, commands/{dispatch,fuzzy}.rs | session/commands/{mod,dispatch,fuzzy}.rs | slash commands | |
| core/src/wire.rs [123] | protocol/mod.rs | Command -> ClientRequest, JSON -> reply | name `wire` -> `protocol` |
| core/src/screens/agents/wire.rs [410] | protocol/agents.rs | wire forms of list/action RPCs, script seed | 28 protocol imports |
| core/src/screens/agents/sources.rs [399] | split: protocol/agents.rs (requests, decode) + agents/discovery.rs (merge by recency) | two sources merge | 8 imports |
| core/src/screens/agents/refresh.rs [451] | split: protocol/agents.rs (reply decode) + agents/refresh.rs (merge pages, `Out`) | | 8 imports |
| core/src/screens/agents/notifications.rs [307] | split: protocol/agents.rs (notification -> event) + agents/notifications.rs (tracking) | | 5 imports |
| core/src/screens/agents/reads.rs [149] | agents/reads.rs, request builders to protocol/agents.rs | Read chain | 2 imports |
| core/src/screens/agents/{mod,center,details,discovery,update,usage,carried}.rs | agents/{model,center,details,discovery,update,usage,carried}.rs | model, derived rows, details, paging, intents, usage fetch, carried oracle items | `mod.rs` [884] -> `model.rs`; `carried` keeps markers (tests/carried.rs) |
| core/src/screens/{cells,transcript,backtrack,sidebar,voice,ui_tree,onboarding,startup,import}.rs (98 each, stubs) | deleted until built | slot stubs | each imports the protocol for nothing |
| core/src/screens/{chat,composer,popovers}.rs [363,148,420] | session/{chat,composer,popovers}.rs | models with content (not stubs) | |
| core/src/screens/{approvals,select,settings,pets,dialogs,resume,status,usage}.rs | {approvals,select,settings,pets,dialogs,resume,status,usage}/mod.rs (or .rs) | feature models | their protocol parts to protocol/ |
| core/src/screens/mod.rs [284] | app root `screens.rs` -> `features.rs` | `Kind`, `DirInfo`, `ScreenStates`, root `ScreenEv/Intent/Command/Reply`, `screen_list!` | keep the dispatch enums (one writer), drop only the 1:1 view tie, §6 |
| core/src/fixtures/{mod,builders,sandbox,script}.rs | same | JSONL scripts, protocol payload builders, sandbox model | builders are protocol-coupled by design (17 imports): allowed; no feature code may import it |
| host/src/main.rs, build.rs | same | `codex-rtwk` bin, ld workaround | |
| host/src/in_process.rs [578] | connectors/in_process.rs | in-process app-server Connector | |
| host/src/{cloud,target,facts,workspace}.rs | connectors/ same | cloud backend, launch target, config facts, directory keys/worktrees | `host/workspace.rs` and `src/shell/workspace.rs` pair, §8 |

## 6. Wrong layer / coupling to fix (ordered by harm)
1. `screen_list!` ties core dirs to UI dirs (rules `every_screen_dir_has_a_core_half_and_a_view_half`). In the target a feature
   has one model and may feed a page, a sheet, or a cell. Keep the core macro for the root enums (one `apply`, R-rules),
   delete the view macro `view_dirs!` and the 1:1 test; the UI registry is hand-written `pages/`, `sheets/`.
2. Protocol inside feature code: `agents/{sources,refresh,notifications,reads}` and the stub `screens/*.rs`. Rule: only `protocol/*`,
   `app.rs`, `threads.rs` (the one writer) and `fixtures/` name `codex_app_server_protocol`; the feature modules use core types.
3. `src/shell/rpc.rs` and `shell/workspace.rs` import the protocol 2x each in the UI crate: the shell is allowed (services), views are not.
4. `shell/fake.rs` (508) is test support in `src`; `shell/help.rs`, `shell/version.rs` carry user-visible words outside
   `strings.rs`: move into the string tables or confirm adr010 passes by allow.
5. `src/ui/presenter.rs` (request -> dialog) and `ui/thread_ui.rs` (R14 controllers) are app wiring, shown as "widgets":
   presenter goes to `sheets/`, thread_ui stays shared; both stay on the rules.rs:426 allowlist.
6. `UiApp.ui` state used by services: the shell touches `UiApp`; a services crate would need that cut first.

## 7. tests/
| current | target | notes |
|---|---|---|
| tests/oracle/*.rs (60 files, ~48k lines), oracle.rs, oracle_snap/ | tests/oracle/ unchanged | replay of codex-tui snaps; named by oracle file, not by our tree; do not move |
| tests/{agents,agents_mouse,carried,pin,pets,popovers,resume,select,status,usage,...}.rs | tests/{pages,sheets,widgets}/ by target folder (pages/agents.rs ...) | one cargo test target per folder (`tests/pages/main.rs`) cuts link time |
| tests/shell_*.rs (4), skeleton.rs, navigation.rs, transitions.rs | tests/services/, tests/app/ | |
| tests/{common,kit,support,srcscan}/ | tests/support/ | three helper kits (`common`, `kit`, `support`) overlap (compare, seed, script, grid): merge |
| tests/layers.rs, rules.rs, l10n.rs, theme.rs, fixtures.rs, cli.rs | tests/rules/ | path tables in §9 |
| 9-line stubs: backtrack cells composer import onboarding sidebar startup transcript ui_tree voice | deleted | |
| core/tests/*.rs (17) | core/tests/<feature>.rs unchanged | agents_* (7) -> core/tests/agents/ |
| host/tests/helper_dispatch.rs | same | |

## 8. Duplicates to merge
- `core/screens/agents/carried.rs::format_tokens_compact` vs `src/screens/status/format.rs::tokens_compact` (both carry
  `status/helpers.rs format_tokens_compact`): keep core's (`core/agents/` -> `core/format.rs`), view calls it.
- Age: `ui/age.rs::age`, re-exported by `resume/layout.rs:40`, plus `agents/view.rs fn age` and the two strings tables'
  `age()` wording: one `widgets/age.rs`; agents/view uses it (strings stay per page).
- Shell/host pairs: `src/shell/workspace.rs` (effects) + `host/src/workspace.rs` (machine); `src/shell/rpc.rs` + `core/wire.rs`:
  documented split, but the names collide; rename the host one `connectors/directory_keys.rs` or keep with comment.
- Test kits: `tests/common`, `tests/kit`, `tests/support` (compare/seed/script/normalize each twice).
- `ui/dots.rs`, `ui/measure.rs`, `agents/path.rs` are three width helpers; fine, but one `widgets/text_fit.rs` is cheaper to find.
- Dead after slices (verify by grep before deleting): `resume/list.rs`, `approvals/dismiss.rs`, `chat/home.rs` skeleton parts.

## 9. Effect on tests/layers.rs and tests/rules.rs (paths are string literals; none uses globs of the new dirs)
| test | now | after |
|---|---|---|
| layers.rs:22 `is_view` | `src/` except app/run/lib | `src/{pages,sheets,widgets}/`; `src/app/`, `src/l10n` apart (app wires intents) |
| layers.rs:28 `is_shell` | `src/shell.rs`, `src/shell/` | `src/services/` |
| layers.rs:74,106 core scans | `core/src` | same; add: only `core/src/{protocol,fixtures}`, `app.rs`, `session/threads.rs` name the protocol |
| layers.rs:189-247 | `src/screens/` dirs, 1:1 halves | drop 1:1 test; "view files only" over `pages sheets widgets` |
| layers.rs:283-320 | `src/app.rs`, `core/src/screens/` | `src/app/app.rs`, `core/src/{agents,...}` |
| rules.rs:75,81,86 R1 | `src/shell.rs` | `src/services/shell.rs` |
| rules.rs:105-127 | `src/app.rs`, `src/ui/view.rs` | `src/app/app.rs`, `src/app/view.rs` |
| rules.rs:134 host | `host/src` | `host/src` (also `connectors/`) |
| rules.rs:178 | `core/src/threads.rs` | `core/src/session/threads.rs` |
| rules.rs:293 `core/src/screens/` | model scan | `core/src/` minus protocol/fixtures |
| rules.rs:331,373,540 `src/ui/` + `src/screens/` | R14, R15 scopes | `src/{pages,sheets,widgets}/` |
| rules.rs:426-432 controller allowlist | 7 file paths | new paths of the same files |
| rules.rs:479,493-506, 633 | theme, gap files, not-yet list (R15), `ends_with("/strings.rs")`, `src/l10n/` | renamed paths; strings rule unchanged |
| tests/srcscan/mod.rs | `sources_of("src")`, `core/src` | same helpers; add `sources_of("src/pages")` etc. |
Run-order effect: moving is `git mv` + `mod` edits + one path table; no behaviour changes. The gap files and R15 "not yet" list
(rules.rs:493-566) name files, so they must move in the same commit as the files, or the rules test fails.

## 10. Move order (each step builds and passes alone)
1. Delete the 9 core stubs + 11 view stubs + 10 9-line test stubs (needs lib.rs/mod.rs edit; orchestrator).
2. `core/protocol/` split of agents wire/sources/refresh/notifications; add the "protocol is named only here" layers test.
3. Merge duplicates (§8).
4. `src/ui/*` -> `app/`, `widgets/`, `sheets/presenter`; `src/shell*` -> `services/`; update layers/rules path tables.
5. `screens/<dir>` -> `pages/` (agents, resume, usage), `sheets/` (approvals, select, settings, pets, dialogs); drop `view_dirs!`.
6. Split `agents/view.rs`, `agents/mod.rs`, `chat/home.rs` by widget (the largest files: 1224, 1101, 636).
7. Test dirs by target folder, kits merged. Do after code moves so `git mv` history stays readable.
Open for the person: (a) a services crate (§3), (b) whether `status` card belongs in `sheets/` (as today, Kind::Sheet) or `pages/session/cells/` (amendment), (c) collapse of onboarding/startup/import into one StartupPage.
Not inspected line by line: the bodies of `agents/{view,mod}.rs`, `chat/home.rs` and `agents/{sources,refresh}.rs` — the split points are from their pub items and module docs; verify at the move.
