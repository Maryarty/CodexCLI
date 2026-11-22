# Recon brief: cells-markdown (S11 history cells, Markdown, streaming renderers)

Recon 2026-11-22 at tip b7e2a0aef. No cargo was run. State comes from the shard markers (`kit::pending` / `kit::na`), read
against the code. `target/nextest/oracle/junit.xml` (2026-11-21 22:22) predates repin-4 (it shows 178 pass, the markers
show 138). Use it only as history.

## 0. First steps for the porter (they change the plan)
1. Probe: `KIT_PROBE_PENDING=1 python3 -I tests/oracle/tools/summary.py --run cells_1 cells_2 cells_3 cells_4 cells_5
   markdown_streaming_1 markdown_streaming_2 markdown_streaming_3 markdown_streaming_4 markdown_streaming_5 renderer_internals
   --list stale`. Then run a normal `--run` of the same shards to get the true baseline.
2. Stale-reason candidates. The text of these 74 reasons no longer matches the code. Re-read each first difference and
   either un-pend the test or rewrite its reason:
   - 31: "core threads::project maps only user and agent messages / ignores mcpToolCall / no exec cell / no dynamic
     tool-call cell": cells_1 12, cells_4 15, cells_5 4. Core now projects CommandExecution, McpToolCall and DynamicToolCall
     into `Item::Tool` (`core/src/threads/activity.rs:88-224`), which `ToolRow` draws (`transcript_activity.rs:54-275`). The
     differences that remain are look differences against the oracle's `ExecCell` / `McpToolCallCell`.
   - 20: "the kit cannot set the session cwd" (markdown_streaming_3 19, _4 1). `Open::cwd(..)` exists now
     (`tests/kit/markdown_gutter.rs:65`). Add `.cwd("/tmp")`, or the oracle's cwd, to each test.
   - 12: "no completion separator" (cells_2 9, cells_3 3). The `Worked for … • HH:MM` row exists
     (`transcript_activity.rs:477-507`).
   - 8 user-cell tests (cells_2 3, cells_5 5) and 3 agent-cell tests (cells_2). Their reasons cite
     `src/screens/chat/home.rs:328/333`, which is gone. Prompts are now `transcript_prompt.rs`, the agent cell is a
     `MarkdownView` (`transcript_pane.rs:167-184`).
   - 23 reasons say "probe: … vacuous / the body passes without it". They pass under the probe but check absence only.
     Keep them pending until the body checks what the oracle claims.
3. The guard `guards::shards_are_black_box_and_never_skipped` is red because of bare asserts at `tests/oracle/cells_3.rs:599-601`
   and `cells_4.rs:477`. Replace them with kit assertions. It is in this chunk.
4. Land step 1 of §6 (the dispatcher and the move) first, in one commit, together with the transcript porter (§5). All
   later steps touch only this chunk's files.

## 1. Scope
- Oracle cells: `tui/src/history_cell/` (mod.rs 395; base 264, messages 936, mcp 865 + mcp_preview 134 + mcp_result 188,
  hook_cell 783, notices 494, session 431, plans 399, approvals 351, patches 333, computer_activity 292, dynamic 281,
  exec 274, separators 265, request_user_input 217, search 190, spoken_artifacts 155, startup_warnings 111, warnings 105,
  delegated_message 75, activity_* 104, markdown_render_cache 53) and `exec_cell/` (render 1411, model 274,
  live_output 259, compact 132, transcript 75).
- Oracle content: `markdown.rs` (804), `markdown_render.rs` (3334) + `markdown_render/` (math.rs 321 + math/render.rs 515,
  mermaid 91, local_links 210, web_links 108, inline_directives 267, table_key_value 305, source_tables 97, streaming 152,
  list_spacing 44, task_lists 39, preferences 43), `markdown_stream.rs` (905), `markdown_text_merge.rs` (50),
  `streaming/` (controller 2208, chunking 457, prose_preview 290, render 291, table_holdback 271, commit_tick 223,
  code_fence 140), `diff_render.rs` (2745, tests from :1365), `wrapping.rs` (2096), `render/highlight.rs` (1700) +
  `highlight_streaming.rs` (119).
- What the user sees: every item of a turn in the transcript (prompt, answer, commands with output, tool calls, patches
  with diffs, plans, searches, images, notices, separators), with the Markdown answer streaming in. The answer shows
  headings, lists, quotes, code, tables, links, file links, math and mermaid as the oracle draws them.
- Which cells live where:
  - /status card (`cells/status/`): usage-status.
  - Transcript list, follow, selection and copy, search, Ctrl+T, disclosure state kept across the list, activity groups
    (`ActivityGroup`, turn details), the session header (`transcript_header.rs`) and the history-paging header rule: chat-transcript.
  - Turn lifecycle, the `Working` row, retry banners, rate-limit warnings: chat-live.
  - Footer `⚠ N warnings · f2 to view`: chat-status.
  - Everything else drawn as a transcript item: this chunk, including each cell's collapsed and expanded rendering given
    an `expanded` flag.

## 2. Tests: 490 in this chunk (138 pass / 12 n/a / 340 pending / 0 todo, by markers)
| shard | total | pass | n/a | pending | oracle files |
|---|---|---|---|---|---|
| cells_1 | 60 | 16 | 0 | 44 | diff_render 23 (5 pass), exec_cell/render 13, live_output 1, transcript 1, computer_activity 4, dynamic 2, hook_cell 6 (pass), mcp 10 |
| cells_2 | 33 | 10 | 1 | 22 | messages 12, patches 3, plans 2, request_user_input 1, search 4 (pass), separators 9, session_transcript 2 |
| cells_3 | 39 | 4 | 0 | 35 | history_cell/tests.rs |
| cells_4 | 39 | 7 | 0 | 32 | history_cell/tests.rs |
| cells_5 | 41 | 18 | 1 | 22 | history_cell/tests.rs 39, warnings 2 |
| markdown_streaming_1 | 58 | 21 | 4 | 33 | markdown 11, markdown_copy 4 (n/a), markdown_render 18, file_citations 9, followups 2, list_spacing 2, math_display 12 |
| markdown_streaming_2 | 60 | 12 | 1 | 47 | math 6 (pass), mermaid 1, preferences 3, task_list 1, web_links 9, markdown_render_tests 40 |
| markdown_streaming_3 | 40 | 14 | 0 | 26 | markdown_render_tests |
| markdown_streaming_4 | 43 | 8 | 0 | 35 | markdown_render_tests 38, streaming/code_fence 5 |
| markdown_streaming_5 | 58 | 18 | 3 | 37 | streaming/controller 23, preview 6, math 6, mermaid 3, render 17, rendering_preferences 3 |
| renderer_internals | 19 | 10 | 2 | 7 | markdown_stream 7, highlight 3, renderable 1 (n/a), wrapping 8 |

The 340 pending, grouped by what blocks them:
- **Blocked by rtwk gaps now: 95.**
  - G73: 44.
  - G61 math: 27. Decision D1 makes this port work.
  - G62 mermaid: 9. D3.
  - G64 nested-list indent: 5.
  - G72 URL-aware wrap: 5 (`renderer_internals` adaptive_wrap ×4, `markdown_streaming_1` does_not_split_long_url_like_token_without_scheme).
  - G63 soft break: 3. D2.
  - G44: 1. G49: 1 (also a kit limit: bare-label frames).
  - About 20 of the 59 "first difference (cause not traced)" tests look like G73 or G64 by name: blockquote ×3, table_* ×8,
    nested and continuation lists ×5, code_block_indented, multiline_finding_items, horizontal_rule (G-new below). Trace them
    in step 2.
- **Other chunks: 4.**
  - `cells_5` visible usage warning and warning count: chat-live and chat-status (2).
  - `cells_2` session-information paging header ×2: chat-transcript.
- **n/a candidates: 7 (D5).**
  - URL "logical line" tests ×5 (`cells_1` command/exploring/output/transcript_view url, `cells_5` plan_update url).
  - Render-cache frame equality ×2 (`cells_1` truncated ANSI tail, `cells_2` finalized_plan_reuses_rendered_transcript_lines).
- **Port work: 234.** About 20 of these may turn out to be G73 or G64 on tracing.
  - Exec, MCP, dynamic and computer-activity cells: 31.
  - Untraced: 59, of which about 39 are port work. Diff render 18, web search 3, agent cell 3, links 5, html 2, code
    fences 3, incremental 1, task list 1.
  - Local file links (cwd): 20.
  - Separators: 12.
  - Startup card, tips and version notices: 10.
  - /mcp inventory + loading: 9.
  - /ps processes: 9.
  - Raw mode (alt+r): 9.
  - Streaming previews and plan stream: 9.
  - Inline exec `display_lines`: 8. First check what the oracle's owned full-screen transcript draws
    (`app/owned_transcript.rs`).
  - User cell: 8.
  - Table, code and link render: 8.
  - Web-link display by terminal kind: 8.
  - Follow-up and inline-visualization directives: 8.
  - Local flows (approval decision, unified-exec interaction, recap loading): 6.
  - Cyber and safety notices: 5.
  - Reasoning seeds: 4.
  - Syntax-theme seed: 3.
  - Spoken / voice: 3.
  - Rendering-preferences seed: 1.
  - Request-user-input result: 1.
  - Patch failure stderr: 1. The wire has no stderr, so this one stays pending.
  - Initial indent: 1.

## 3. What exists in ours
- `src/pages/session/cells/`: one widget per file, landed by chat-cells and chat-cells-2.
  - citation.rs 284: file-citation masking, `view(source, cwd)`.
  - markdown.rs 183: lifts table fences before the view.
  - patch.rs 403: `Diff::parse` + `Highlighter`, disclosure.
  - plan.rs 47, plan_update.rs 238, reasoning.rs 52 (`DefaultTextStyle` dim italic), web.rs 68, viewed_image.rs 94,
    generated_image.rs 60, collab.rs 258, delegated.rs 64, hook.rs 128, error.rs 26, warning.rs 48, deprecation.rs 46,
    notice.rs 42, one_row.rs 14, disclosure.rs 47 (the `+ Show details` control), strings.rs 234.
  - mod.rs 78 is still the skeleton's empty `Screen`, and `debug_entries()` is empty.
- `src/pages/session/transcript_pane.rs:104-340` (transcript area): `ListView::builder`. The `match item` at :148-303
  chooses the cell; Agent is :167-184 (gutter `• ` + `Expanded(MarkdownView)`, right reserve 1).
- `transcript_activity.rs` (transcript area): `ToolRow` (:35-475) draws exec, MCP and dynamic calls, exploration
  coalescing and the detailed shells; `completion()` (:477-507) draws the separator row.
- `transcript_prompt.rs` (transcript area): `panel()` draws the user prompt.
- `src/widgets/markdown.rs` 114: `view()` = `MarkdownView::blocks` with heading markers (`with_heading_markers`) and
  `themed()` (`LinkDestinations::Auto`). `src/app/theme.rs` `transcript_markdown` holds the colours.
- Core:
  - `core/src/threads.rs:78-126`: `Item` with 18 variants. `item_entry` is at :1340, `project` at :1158-1270
    (`AgentDelta` :1255).
  - `core/src/threads/cells.rs` 465: projects Patch, Web, PlanUpdate, Hook, Notice, Delegated, Generated and the
    reasoning summary.
  - `threads/activity.rs` 233 (tool), `collab.rs` 325, `directives.rs` 216, `citations.rs` 280, `user_prompt.rs` 72,
    `message.rs` 89.
  - `core/src/screens/cells.rs` 98 is the EMPTY slot: `Model`, `Ev`, `ThreadPart`, `Command` / `Reply` / `Ctx` through
    `ThreadFx`, and `seed()` for `{"step":"screen","dir":"cells",..}`. This is where the cell facts and RPCs go.
- Tests: `tests/pages/cells.rs` 1768; `core/tests/{cells,citations,directives}.rs`; `tests/kit/markdown_gutter.rs`
  (`Open::markdown`, `agent_says`, `reasoning_says`, `cwd`).
- Missing (protocol entry points, ADR-003):
  - `mcpServerStatus/list` (`ClientRequest::McpServerStatusList`, `app-server-protocol/src/protocol/common.rs:1325`).
    App-scoped, so it is a `cells::Command` from `apply_intent` through `AppFx`.
  - `thread/backgroundTerminals/list` (common.rs:798). A thread RPC, so it goes through `ThreadFx::request` with a
    `cells::Command` (`/ps`).
  - `item/commandExecution/terminalInteraction` (common.rs:2111, `TerminalInteractionNotification`). Add it to
    `threads::project` as an interaction item (oracle `history_cell/exec.rs` `UnifiedExecInteractionCell`).
  - Error `codexErrorInfo: CyberPolicy` (`v2/shared.rs:88`) together with thread daybreak (`ThreadPart` of chat): the
    cyber notice item (oracle `chatwidget/turn_runtime.rs:446-450`). The same goes for `safety_recourse.rs:105` (bio
    block).
  - Host facts, not in the protocol, for `cells::Model`:
    - terminal kind for link display (oracle `markdown_render/web_links.rs` `WebLinkDisplay::for_terminal`);
    - `tui.rendering` preferences (`markdown_render/preferences.rs`: mermaid, math, tables, list spacing);
    - syntax theme name;
    - `tui.show_tooltips` and first-session flag (`history_cell/session.rs` `new_session_info`);
    - the version check result (update available);
    - the remote server version;
    - per-turn `codex_otel::RuntimeMetricsSummary` (`chatwidget/turn_runtime.rs:60-68`, session telemetry);
    - the viewer's clock and 12/24-hour setting for separators.

## 4. Flutter -> rtwk
- **Choosing the cell for an item.** Flutter: `ListView.builder(itemBuilder: (_, i) => switch (item) {...})`. rtwk: the
  list stays `ListView::builder` in `transcript_pane.rs`; the switch moves to `cells::cell(item, &CellEnv) -> AnyWidget`
  in `cells/mod.rs`.
- **Detailed (Ctrl+T) and raw (alt+r) modes.** Flutter: `InheritedWidget` / `Provider<RenderMode>`. rtwk: a
  `CellMode { detailed, raw }` read with `rtwk::provider::{read, watch}`, as `transcript_pane.rs:104-112` reads `ThreadModel`.
- **Expand and collapse.** Flutter: `ExpansionTile` / `ExpansionController`. rtwk: `ExpansionController`
  (`rtwk-widgets/src/layout/expansion.rs:33`), or keep `cells/disclosure.rs` + a `StatefulWidget`. The state across
  scroll-out is the transcript chunk's.
- **Markdown answer.** Flutter: `MarkdownBody(data, styleSheet)`. rtwk: `MarkdownView::blocks` (`rtwk-rich/src/markdown/view.rs`)
  through `widgets/markdown.rs`.
  - Document-level semantics (heading markers, soft break D2, local-link display, citations, math D1, follow-up
    directives) are transforms of the parsed blocks, done before the view. That is the precedent of
    `with_heading_markers` and G60. The text a selection copies is unchanged (`MdInline::{SoftBreak, HardBreak}` both
    copy `\n`, `nodes.rs:306`).
- **Streaming answer.** Flutter: a `StreamBuilder` over a `ValueNotifier<String>`. rtwk: ADR-004's live tail. The item's
  text grows in core through `apply`, and rtwk coalesces to one rebuild per frame. The holdback and previews (unfinished
  link, table tail, follow-up prompt, open math or mermaid) are a pure function of the source in core, carried from
  `streaming/prose_preview.rs` and `table_holdback.rs` with a comment naming the files. `MarkdownStream::live()` and
  `take_settled()` (`markdown/stream.rs`) are for inline mode only; do not swap them in (reuse-map row "streaming/
  controller": the oracle commits by line).
- **Code and diff colours.** Flutter: `flutter_highlight`. rtwk: `Highlighter` + `SyntaxTheme`
  (`rtwk-rich/src/highlight/`, 10 languages: bash, dart, diff, go, javascript, json, python, rust, yaml).
  `Language::new(name, aliases, &dyn Lexer)` is public (`lexer.rs:545`), so an app `Lexer` plugs in for the patch and exec
  cells (D4). `MarkdownView` fences use the fixed set (`markdown/lines.rs:455`): G-new below.
- **Labelled rows, gutters and hanging indents (exec, MCP, `└` branches).** Flutter: `Row[SizedBox(width), Expanded(Text)]`.
  rtwk: `Row` + `SizedBox` + `Expanded` + `Text::wrap` / `max_lines` + `TextOverflow::Ellipsis`, as `transcript_activity.rs`
  and `patch.rs` do now. No hand wrap (R15).
- **Mermaid diagram.** Flutter: `LayoutBuilder` + a `CustomPaint` / builder. rtwk: `LayoutBuilder`
  (`rtwk-framework/src/layout_builder.rs`) gives the width to the backend crate `codex-mermaid` (`render_spans(source,
  max_width)`, `codex-rs/mermaid/src/lib.rs:63`), drawn as `Text::rich` rows with no wrap (D3).
- **Math.** Flutter: `flutter_math_fork`. rtwk: none. The converter is pure (`markdown_render/math/render.rs:81`
  `render(source, mode) -> RenderedMath{text, required_width}`), carried into core (D1). The fallback when the line is
  narrower than `required_width` is a `LayoutBuilder` choice of which spelling to draw.
- **Tables.** Flutter: `Table` with `TableBorder.horizontalInside`. rtwk: `MarkdownView`'s grid (one-line cells, `…`).
  The oracle's box rules, wrapped cells and key/value fallback are G73 item 3; nothing to compose (D6).
- **/mcp and /ps output, startup card, notices.** Flutter: `Card` / `ListTile` columns. rtwk: `Column` of `Text::rich`,
  `Grid` with content tracks for label columns (`cells/status/card.rs` pattern), `Hyperlink`
  (`rtwk-controls/src/link.rs:45`) for docs links, `Spinner` (`rtwk-animation/src/spinner.rs`) for loading cells.
- **Viewed and generated image preview.** Flutter: `Image.file`. rtwk: `rtwk_rich::image` (`image/widget.rs`). File IO stays
  in services: debt task `chat-cells-debt-viewed-image` (named in parity/cells.md). Not in this chunk.
- **Mouse.** Disclosure taps on every expandable cell (exec, MCP, dynamic, plan, image), as `Pressable::on_tap`
  (`transcript_activity.rs:255-262`). Links use OSC 8 (`Span::link`). Test with rtwk_test taps (ADR-011).

## 5. Area and conflicts
Porter area (tm). Listed file by file because usage-status holds `src/pages/session/cells/status`:
- `src/pages/session/cells/{mod,citation,collab,delegated,deprecation,disclosure,error,generated_image,hook,markdown,
  notice,one_row,patch,plan,plan_update,reasoning,strings,viewed_image,warning,web}.rs`.
- New cell files: `exec.rs`, `tool_call.rs`, `computer_activity.rs`, `user.rs`, `agent.rs`, `separator.rs`,
  `session_info.rs`, `update_available.rs`, `safety.rs`, `decision.rs`, `interaction.rs`, `mcp_inventory.rs`,
  `processes.rs`, `request_input.rs`, `raw.rs`, `src/pages/session/cells/fixtures`.
- `src/widgets/markdown.rs` (may become `src/widgets/markdown/`), `src/app/theme.rs`.
- Core: `core/src/screens/cells.rs` + `core/src/screens/cells/`, `core/src/threads/{cells,activity,collab,directives,
  citations,user_prompt,message}.rs`, new `core/src/threads/{math,local_links,followups,stream_preview}.rs`.
- Tests: `core/tests/{cells,citations,directives}.rs` + new `core/tests/markdown_*.rs`;
  `tests/oracle/{cells_1..5,markdown_streaming_1..5,renderer_internals}.rs` and their `tests/oracle/fixtures/<shard>/`;
  `tests/pages/cells.rs` + new `tests/pages/markdown.rs`.
- `docs/rtwk-port/parity/cells.md`.

Shared files, each with the minimal change this chunk needs:
- **chat-transcript** holds `transcript_pane.rs`, `transcript_activity.rs`, `transcript_prompt.rs`,
  `pages/session/strings.rs`, `keys.rs`, `tests/screens.rs`, `rtwk-gaps.md`. Changes:
  - (a) Replace the `match` at :148-303 by one `cells::cell(..)` call, and provide `CellMode`.
  - (b) Move `ToolRow` to `cells/exec.rs` + `cells/tool_call.rs`, and `completion()` to `cells/separator.rs`.
    `transcript_activity.rs` keeps the grouping of runs (`rest`) and calls the cell.
  - (c) Move `panel()` to `cells/user.rs`.
  - (d) Move the tool, `worked_for` and `image_label` strings to `cells/strings.rs`.
  - (e) Bind alt+r in `keys.rs` to toggle `CellMode.raw` (UI state in `UiApp.ui`).
  - (f) Add `mod markdown;` in `tests/screens.rs`.
  - (g) Add the 2 gap rows to `rtwk-gaps.md`.
  - Recommendation: the cells porter makes (a)-(d) in one move commit that lands before either porter goes on.
- **chat-live** may need `core/src/threads.rs` (steer). This chunk adds `Item` variants, a live marker on the streaming
  agent item, and `project` arms for TerminalInteraction and CyberPolicy. Sequence it, or give the file to the porter
  who runs first.
- **keymap** holds `core/src/app.rs`, `core/tests/all.rs`, `host/src/facts.rs`, `tests/kit/seed.rs`. Changes: `mod` lines in
  `core/tests/all.rs`. Routing a cells `ScreenEv` should need no app.rs arm; check in step 1.
- **settings, then chat-commands**: `src/pages/session/actions.rs`. Add the `/mcp` and `/ps` arms that send the cells intent.

Needs (the porter must NOT build these itself):
- Host: the facts listed at the end of §3, sent as `cells::Ev` (host/src/facts.rs).
- Script: a `reply` step in `core/src/fixtures/script.rs` (unclaimed) that answers the next request of a method:
  `mcpServerStatus/list`, `thread/backgroundTerminals/list`, `thread/turns/list`.
- Cargo: `codex-mermaid` (workspace crate) for the UI crate after D3. `syntect` + `two-face` after D4. Each goes in its
  own commit.

## 6. The plan (one job, tests first in every step)
1. **Structure.**
   - `cells::cell(item, &CellEnv)` + `CellMode` in `cells/mod.rs` (drop the skeleton `Screen`).
   - Do the moves of §5 (a)-(d).
   - `tests/pages/cells.rs` stays green.
   - Debug entries: a "cells gallery" route over a JSONL fixture through `apply`.
   - Oracle: `history_cell/mod.rs`. Turns on: nothing new; the base for everything else.
2. **Markdown transforms and tracing** (`widgets/markdown.rs`, core `threads/local_links.rs`).
   - Soft break to hard break (D2): G63 ×3.
   - Local file links relative to cwd (oracle `markdown_render/local_links.rs`), plus `.cwd()` on the 20 tests.
   - Link display by terminal kind: a `cells::Model` fact gives `LinkDestinations::Suffix` or `Auto`. 8 tests, with a
     `screen` seed.
   - HTML verbatim, code fences kept unwrapped, task-list markers.
   - Trace the 59 untraced tests: un-pend, or re-pend with G73, G64 or G-new.
   - Oracle: `markdown_render.rs`, `markdown_render/{local_links,web_links,task_lists}.rs`.
3. **Math (D1).** Carry `markdown_render/math.rs` + `math/render.rs` into core `threads/math.rs`, masked as `citations.rs`
   masks. Inline math becomes a text node. Display math becomes a hard-broken paragraph under a `LayoutBuilder` that falls
   back to the source below `required_width`. Covers markdown_streaming_1/2/5 math, 27.
4. **Mermaid (D3).** Top-level ```` ```mermaid ```` fences through `codex-mermaid` under `LayoutBuilder`; the holdback is in
   step 5. Covers 9; the part that needs unpadded code stays G73.
5. **Streaming.** Core `threads/stream_preview.rs` (oracle `streaming/prose_preview.rs`, `table_holdback.rs`,
   `code_fence.rs`, `markdown_render/inline_directives.rs`):
   - Applied to the live agent item and the proposed-plan stream (`history_cell/plans.rs` `ProposedPlanStreamCell`).
   - Unfinished link, follow-up prompts hidden while the directive is open, table tail, open fence.
   - Inline visualization directives.
   - Covers markdown_streaming_5: 9 streaming previews and plan stream, 8 directives.
6. **Exec, MCP, dynamic and computer-activity cells** (`cells/exec.rs`, `tool_call.rs`, `computer_activity.rs`; oracle
   `exec_cell/render.rs`, `history_cell/{mcp,mcp_preview,mcp_result,dynamic,computer_activity}.rs`).
   - Header labels, `└` / 4- and 8-space continuations, highlight-limit fallback, stderr tail, image blocks, code mode.
   - Covers cells_1, 4 and 5: 31 + 8 `display_lines` + exec URL.
7. **Separators** (`cells/separator.rs`; oracle `history_cell/separators.rs`): duration, date on another day, 12/24 hour,
   runtime metrics (fact). Seed through `cells` screen steps. 12.
8. **User and agent cells** (`cells/user.rs`, `agent.rs`; oracle `history_cell/messages.rs`): gutter, wrapped URLs, image
   labels on the prompt surface, tiny widths, narrow agent prefix. 11. Spoken ×3 stay pending: the wire has no spoken
   flag (voice).
9. **Session info and notices** (oracle `history_cell/{session,notices,approvals,exec,request_user_input}.rs`):
   - Startup card (help lines, tips, NUX tooltip), update-available ×3, server and daemon version ×2.
   - Cyber and safety ×5, approval decision ×3, unified-exec interaction ×3, recap loading, request-user-input result.
   - Reasoning feature and config seeds ×4.
   - About 30.
10. **/mcp inventory and loading, /ps processes** (`cells/mcp_inventory.rs`, `processes.rs`; oracle `history_cell/mcp.rs:472-860`,
    `history_cell/exec.rs:142-274`). Core `cells::Command` + decode, the reply step (need), and the actions arms. 18.
11. **Raw mode** (`cells/raw.rs`; oracle `history_cell/mod.rs:159-200` `raw_lines_from_source`): each cell draws its source
    when `CellMode.raw` is set; alt+r is shared change (e). 9.
12. **Diff render** (patch cell; oracle `diff_render.rs` tests :1365-2745): gallery sizes, line-number gutter, wraps,
    rename, relativized paths, ANSI-16 without background, tabs and wide characters. Highlight after D4. 18.
13. **Close.**
    - `shards.py --check` and the parity pack (`parity/cells.md`, shard counts, gap rows, binary rows).
    - Scenarios on both binaries: markdown, markdown-list-layout, reasoning, exec-command, exploration, mcp-tool,
      code-mode, unified-exec, background-terminal, patch-approval, patch-heredoc, web-search, view-image,
      image-generation, proposed-plan, plan-tool, request-user-input, multi-agent (known red), error-*.

## 7. Gaps, risks, decisions
- G-new (code highlighting): Flutter `MarkdownBody(syntaxHighlighter:)`. Tried: `MarkdownView` with a ```` ```cpp ````
  fence. What differs: plain text, because the fence language comes from the fixed 10-language set
  (`markdown/lines.rs:455`) and there is no highlighter hook.
- G-new (G73 item 5): Flutter `MarkdownStyleSheet.horizontalRuleDecoration`. Tried: `MarkdownView` thematic break. What
  differs: it draws `─` across the width (`lines.rs:457`); the oracle draws `———` (`horizontal_rule_renders_em_dashes`).
- Risk: the structure move (§5 a-d) collides with the running chat-transcript porter (claim 01a1eaa9-6db3). Sequence it
  first.
- Risk: `Item` additions in `core/src/threads.rs` collide with chat-live's steer work. One owner at a time.
- Risk: the streaming preview must run only on the live item; a live marker in core gives the view that (ADR-003: the
  marker is set only through `apply`).
- D1 (G61 math): carry the pure LaTeX-to-Unicode converter into core (citations precedent, G60), or keep 27 tests pending
  on rtwk. **Recommend carry.**
- D2 (G63): close it as port work, the soft-break-to-hard-break block transform (heading-marker precedent). **Recommend yes.**
- D3 (G62): `codex-mermaid` used as is in the UI crate under a `LayoutBuilder` (top-level fences only), or wait for
  `MarkdownView` builders. **Recommend the crate.**
- D4: a syntect-backed `Lexer` carried from `render/highlight.rs` for the patch and exec cells, or rtwk's 10 languages.
  **Recommend the Lexer**; Markdown fences stay G-new.
- D5: n/a for the 5 URL logical-line tests (a grid row cannot hold a token wider than the screen) and the 2 render-cache
  frame tests. **Recommend n/a with that reason.**
- D6 (G73): splitting top-level blocks into separate widgets (code, tables) would leave nested cases different. **Recommend
  waiting for rtwk Markdown options / element builders** (person decision 2026-11-21: record the rest). One rtwk feature
  (block builders per kind) would close G73, G64, G62-nested and the fence-highlight G-new.
