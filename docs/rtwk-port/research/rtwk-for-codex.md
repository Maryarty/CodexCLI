# rtwk for the Codex TUI: API map, capability matrix, references

Research for tm `port-design` (2026-11-17), rtwk at `675a98d7`. Everything comes
from files opened in rtwk; nothing was built or run. Not covered: rtwk-ssh,
rtwk-web, rtwk-l10n, rtwk-cli beyond a grep, xtask internals.

## 1. rtwk map

Layers, bottom to top: rtwk-unicode, rtwk-input, rtwk-core, rtwk-render,
rtwk-framework, rtwk-widgets, domain crates, facade `rtwk` (README crate table,
docs/ARCHITECTURE.md).

| crate | role | items a Codex-like app uses |
|---|---|---|
| rtwk-core | surface, presenter in 3 modes, terminal backends, frame loop, headless | `presenter::ScreenMode::{FullScreen, Inline{at_line_start}, Viewport}`; `screen::Screen::{commit, resize_region, leave}` (docs/design/inline.md); `terminal::TerminalOptions` (kitty_keyboard, bracketed_paste, mouse, normal_screen_mouse, focus_events; terminal/info.rs:316); `Terminal::{copy_to_clipboard, notify, set_title, suspend}` (terminal/mod.rs:1298-1375); `terminal::TerminalInfo` (caps, graphics, color_mode, kitty_keyboard); `app::App::{suspend, suspend_to_shell}` (app.rs:3002, 3036); `app::AppOptions` (app.rs:800) |
| rtwk (facade) | `App`, `run`, app API | `run_app` / `run_app_with(state, view, RunOptions{mode, max_fps, theme, locale, terminal, clear_inline_on_exit})` (crates/rtwk/src/platform/app_start.rs:23-87); `app::{AppView, ViewCx, AppCx}` (app.rs:938; `spawn`, `channel`, `emit`, `write`, `quit`, `unchanged`); `focus::{Focus, KeyPress, KeyBindings}`; `prelude`; `column!` / `row!` |
| rtwk-widgets | basic widgets, binding | `binding::Static` (inline history sink; widgets/src/binding/statics.rs), `Commit`, `Text`, `Builder`, ANSI spans, `style::Span` (OSC 8 link, style.rs:411) |
| rtwk-framework | elements, state, build owner | `StatefulWidget`, `State`, `Cx`, `AnyWidget`, `Obs<T>` |
| rtwk-editing | text editing | `TextArea` (text_area.rs:109), `TextField`, `TextEditingController` (value, selection, composing, undo/redo; text_editing.rs), `editing::paste(tree, text)` (lib.rs:54), `Autocomplete`, `editing_key` |
| rtwk-scroll | scrolling | `ListView::builder` (stick_to_bottom), `ScrollController`, `SingleChildScrollView`, `LogView` |
| rtwk-overlay (+ -base) | popups | `show_dialog`, `show_toast`, `Popover`, `OptionList` (overlay-base/options.rs:96), `CommandPalette` (command_palette.rs:451), `Overlay`, menus |
| rtwk-controls | controls | `SelectionList` (selection_list.rs:114), `Select`, `Button`, `KeyHintsBar`, `Link` (OSC 8, link.rs), `Form` |
| rtwk-animation | animation | `Spinner`, skeleton shimmer (skeleton.rs), controllers, tweens |
| rtwk-rich | rich text | `markdown::{MarkdownStream, MarkdownView, ComposedMarkdownView, parse_markdown, Block}` (stream.rs:78: `add`, `live`, `take_settled`, `snapshot`); `highlight::{Highlighter, Lexer, Language, SyntaxTheme, CodeView}`; `diff::{Diff::parse, Diff::between, DiffView, DiffTheme}`; `json`; `bigtext` |
| rtwk-tokio | tokio bridge | `TokioApp::{spawn_tokio, subscribe, try_subscribe, follow}`, `Queue::{Bounded, Latest}`, `enter` |
| rtwk-state | external stores | `Source<R>`, `Select`, `store_provider`, `reactive::Derived` (feature `reactive-graph`) |
| rtwk-provider / rtwk-hooks | DI, function components | `Provider`, `watch` / `read` / `select`; `use_state`, `use_effect`, `use_stream`, `use_future` |
| rtwk-test | headless harness | `Tester::{pump_app, pump, press, type_text, paste, click, resize, lines, screen, styled_screen, assert_*}`, `find::*`, insta snapshots |
| rtwk-term | VT emulator and pty | embedding a terminal (exec output) |
| rtwk-dev | `rtwk new --template {app,dashboard,form,inline}`, `record` / `play`, `inspect`, `logs` | `templates/inline/src/main.rs` (65 lines) |
| rtwk-gallery | 148 catalog routes with Rust source | `cargo run -p rtwk-gallery`; src/pages/*.rs |
| rtwk-e2e | scenarios in a pty | `agent.rs` (648 lines) |

- **Start.** `run_app_with(State, view, RunOptions{ mode: ScreenMode::Inline{at_line_start: true}, clear_inline_on_exit: .. })`. `run_app` is full screen; a fixed viewport is `ScreenMode::Viewport`, with `TerminalOptions::alternate_screen` choosing the physical screen. `rtwk::app::run` takes your own frame loop.
- **Async.** `cx.spawn(fut, then)` runs `!Send` futures on the UI thread; `cx.channel` gives a Sender into the app; the terminal waker interrupts the read. For tokio, `rtwk-tokio`: `subscribe(stream, queue, on_end)` batches every item between two frames into one flush; `Queue::Bounded` gives backpressure, `Queue::Latest` coalesces; unmounting aborts the producer (contract in rtwk-tokio/src/lib.rs).
- **Redraw** is implicit: a handler that changes state rebuilds, `cx.unchanged()` opts out. Input frames are not held to `max_fps`, animation is (docs/design/responsiveness.md). An idle app costs 0 frames.
- **State**, three levels (docs/guide/state.md, docs/design/framework-api.md): D-over-A (one state value + `view(&App, &mut ViewCx<App>)`, `cx.on(|app, cx| ..)` handlers — the default); A+Msg (`StatefulWidget` + `State` for components); Obs / providers / hooks, then `rtwk-state` (`Source`, `Select`, `store_provider`, `Derived`) for an existing store such as a core-event stream.

Closest skeletons to Codex: `crates/rtwk-rich/examples/chat.rs` (283 lines, inline chat: `Static` history, `MarkdownStream::take_settled` committing complete blocks, a live tail, TextEditingController, Spinner, KeyHintsBar), `crates/rtwk/examples/agent_chat.rs` + `examples/monitor/` (tokio), `agent_chat_latency.rs`, the inline template.

## 2. Capability matrix (22 rows: 14 ready, 7 partial, 1 missing)

| need | rtwk answer (file) | status | note |
|---|---|---|---|
| inline region + finished history into scrollback | `Static` + `Screen::commit` (widgets/binding/statics.rs; core/presenter/modes.rs; design/inline.md §1.12.3); `chat.rs` | ready | rows go in with IL above the region and are never rewritten; terminal scroll and selection work on them |
| resize reflow | `Screen::resize` repaints the region (inline.md §1.12.2) | partial | committed history belongs to the terminal: rtwk cannot re-wrap it. Codex's transcript re-render on resize needs a design |
| streaming markdown, commit ticks | `MarkdownStream::{add, live, take_settled}` + `MarkdownView::blocks` (markdown/stream.rs; rich.md §R.3) | ready | incremental parse, `Arc` sharing of closed blocks, settled blocks go to history; 24 µs per token at 1000 turns (guide/markdown.md) |
| syntax highlighting | `highlight::{Highlighter, Language, SyntaxTheme}` (highlight/lang/*.rs) | partial | 10 languages (bash, dart, diff, go, js/ts, json, python, rust, yaml); `Lexer` is an open trait; a syntect adapter producing `Span`s is possible |
| unified diff | `Diff::parse` + `DiffView` unified / side-by-side, word diff, `DiffTheme` (diff/view.rs, diff/mod.rs; rich.md §R.4) | partial | model ready; `DiffView` is a scroll widget — as a non-scrolling `Static` history item unverified |
| multiline editor, history, paste, attachments, IME | `TextArea` + `TextEditingController` (undo, selection, composing), `editing::paste`, `Autocomplete` (rtwk-editing; widgets-editing.md) | partial | missing: Up/Down input-history recall, atomic placeholder elements (`[Image #1]`), paste-burst detection, `/command` and `@file` popup glue; `editing::paste` is called by the host |
| popups / list pickers with filtering | `OptionList`, `Autocomplete`, `CommandPalette` (`fuzzy_match`), `SelectionList`, `Select` | partial | widgets ready; a popup growing the inline region above/below the composer has no example; `Overlay` in an inline app mentioned (widgets-overlay.md:206), untested |
| overlays / alt-screen pager (Ctrl+T) | — | missing | no runtime switch between Inline and alt screen (no `set_mode`). Workarounds: full-height in-region pager, or `App::suspend(run)` handing the terminal to a closure |
| approvals modal | `show_dialog` (overlay/dialog.rs:301) or `SelectionList` in the region, `KeyBindings` | ready | dialog + barrier + focus return |
| spinner / shimmer | `Spinner`, skeleton shimmer (rtwk-animation) | ready | a spinner does not delay input (responsiveness.md) |
| OSC 8 | `Link` (controls/link.rs), `Span` link (style.rs:411), ANSI parse | ready | |
| OSC 52 clipboard | `Terminal::copy_to_clipboard`, tmux-wrapped (terminal/mod.rs:1323; sequences.rs:35) | ready | |
| notifications | `Terminal::notify`, `AppBinding::notify` (terminal/mod.rs:1345; sequences.rs:264) | ready | |
| keyboard enhancement | `TerminalOptions.kitty_keyboard{,_flags}`, parser (info.rs:335) | ready | |
| bracketed paste | `TerminalOptions.bracketed_paste` | ready | paste timeout option exists |
| mouse / selection | `SelectionArea`, `normal_screen_mouse`, `scroll_input` | ready | history selection stays the terminal's |
| terminal probing | `TerminalInfo`, probe.rs (DECRQM 2026/2027/2048/69/2031, CPR, bg colour) | ready | |
| suspend / resume Ctrl+Z | `App::suspend_to_shell` (app.rs:3036); design/jobcontrol.md | ready | POSIX only |
| external editor | `App::suspend(|| run editor)` (app.rs:3002) | partial | primitive exists; no $EDITOR helper or file round-trip |
| Windows | `platform/windows.rs`, console_input.rs | partial | FEATURE_PARITY X0003 / repository.X0001 "partial": 11 ConPTY scenarios in CI |
| headless testing + snapshots | `rtwk-test` `Tester`, fake clock, strict VT model, insta, `rtwk-e2e` pty | ready | |
| performance under token streaming | design/rust-native.md §2 (2000 tokens: frame core p50 38 µs, 0.52 elements rebuilt per token with Obs, 0 idle frames); docs/PERFORMANCE.md vs ratatui | ready | caveat: one huge live table costs 4.7 ms per token at 2,000 rows (transcript.md); no Codex-shaped benchmark |

## 3. How an agent finds rtwk API today

| source | gives | cost | gap |
|---|---|---|---|
| `crates/rtwk/SKILL.md` | 415 lines: entry points, state, forms, tables, overlays, tokio, CLI, testing, doctests | ~6k tokens | no inline mode, `Static`, markdown streaming, editor internals, ratatui terms |
| `docs/parity/api_map.csv` | 776 rows twk (Dart) → rtwk | cheap per grep | maps from Dart: useless to a ratatui porter |
| `docs/parity/registry.csv` / `.md` | 6,469 rows, every Rust path from rustdoc JSON, with status | huge | no signatures, no docs |
| `tools/parity/rust_api.py` | walks rustdoc JSON (nightly) across crates, follows re-exports; path, kind, file:line, members | a generator to reuse | emits no signature or doc line |
| `docs/design/*.md` | 75 pages, 29k lines, "what exists" tables, checked rust blocks (`check_snippets.sh`) | 3–10k tokens a page | no index by task |
| gallery | 148 routes with live source | read source | no machine-readable list |
| examples | `chat.rs`, `agent_chat`, `monitor`, `acceptance_form`, `file_manager` | ~300 lines each | no index |
| `rtwk new --template inline` | 65-line starter | tiny | echo only |
| rustdoc | doctests on public items | a build + reading source | no compact index |

Missing: a signature-level symbol index, a ratatui→rtwk concept map, a
Codex-feature→widget map, a widget catalog, an example index.

## 4. Proposed generated references

1. **API index per crate** (`<crate>.txt`, 100–400 lines each, 5–15 crates, under 4k lines, plus a 150-line `INDEX.md`): one line per public item, `path | signature | first doc sentence | file:line`. Source rustdoc JSON; extend `tools/parity/rust_api.py` (it already walks paths and re-exports); skip derived and blanket impls. Freshness: a `check` mode in `cargo xtask check` / CI.
2. **ratatui → rtwk concept map** (~150 lines): Frame/Buffer/Widget/Layout/Paragraph/List/Block/Style/Text/Line/Span/crossterm::Event/`Terminal::insert_before` → rtwk items. Hand-written TOML + a checker that validates every rtwk path against the index.
3. **Codex feature → rtwk map** (~120 lines): section 2 plus rows for diff cell, exec cell, status line, slash popup, history cell, shimmer, approvals — each with rtwk item, example file, gap. Lives with the port (CodexCLI), checked against the index.
4. **Widget catalog** (~250 lines): one line per widget with a minimal constructor, from the gallery catalog (`reference_catalog.rs`, 153 routes) and the first doctest.
5. **Example index** (~60 lines): examples and templates with the task each covers and its size, from `//!` headers.
6. **Inline recipes** (~200 lines): Static, commit ticks, live tail, popup in region, resize, suspend, exit cleanup, tokio feed — doctests from the design pages plus `chat.rs` excerpts.

## 5. Gaps and risks, ranked

1. No alt-screen pager / runtime mode switch (Codex: Ctrl+T transcript, diff views) — core change or a design decision.
2. Resize reflow of committed history: the terminal owns the text; re-wrap needs re-printing.
3. Composer features (input history, atomic attachments, paste burst, slash and file popups) are app code over `TextArea`.
4. Syntax coverage: 10 languages vs syntect — adapter or more lexers.
5. Popups inside the inline region: no example; region growth and clipping unverified.
6. API instability: early status, facade cut (ADR-020), registry churn — pin a commit, regenerate the index per bump.
7. Windows: partial CI evidence.
8. Performance unknown for Codex's shape (fast streaming + big live table + 10k-line exec output); the 4.7 ms/token table case is the known trap.
9. Discoverability: SKILL.md is the only agent-facing text and it does not cover inline.
