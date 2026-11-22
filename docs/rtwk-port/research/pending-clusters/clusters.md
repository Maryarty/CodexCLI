# Pending oracle tests by root cause (2026-11-22)

Source: the land junit (`.claude/worktrees/land/codex-rs/tui-rtwk/target/nextest/oracle/junit.xml`),
1645 pending (10 of them `kit::missing`) + 30 todo = **1675 tests, 108 clusters, 0 unclassified**.
Each test sits in exactly one cluster, the first missing thing its reason names (a feature
before a kit capability: a kit capability unlocks nothing while the feature is absent). Read
from reasons and test bodies only; no suite run, no probe. Machine form: `clusters.json`
(`[{cluster, title, kind, size, owner, uniform, tests:[{shard,name}]}]`).

Rank = tests / size weight (S=1, M=3, L=8). Owner = the running chunk whose brief holds the
work, `none` where no chunk runs (resume, approvals, cross-widget keymap, kit, rtwk gaps,
which the person files and rtwk fixes). Owner totals: none 377, chunk-cells 248,
chunk-popups 227, chunk-status 202, chunk-live 175, chunk-transcript-2 168,
chunk-composer 155, chunk-commands 93, chunk-usage 30.

By kind: feature 1179, seed 107, debug entry 69, kit capability 57, person decision 36,
protocol arm 26, stale pending 12, rtwk gaps 189 (G73 77, G61 27, G65 26, text break 16,
annotations 13, layout 7, G68 7, G63 6, G62 4, G-new 4, G67 2).

## Largest single unlocks (by raw count)

vim 59 (chunk-composer, L) · resume-modes 48 (none, L) · g73-blocks 46 (rtwk G73, M) ·
status-line 44 (chunk-status, L) · plugins 38 (chunk-popups, L) · tx-find 37
(chunk-transcript-2, L) · keymap-read 34 (none, M) · exec-cells 33 (chunk-cells, L) ·
rate-limit-prompts 31 (chunk-live, L) · g73-table 31 (rtwk G73, L) · tool-cells 30 ·
permissions 30 · tx-mouse-select 30 · history-replay-todo 30.

## Table (ranked by tests per size)

| # | cluster (the one missing thing) | kind | tests (shards) | size | owner | uniform? |
|---|---|---|---|---|---|---|
| 1 | `g73-blocks`: G73 items 1/2/4/5 + G64: quote `> ` marker, unpadded code rows, `•` bullets 4-cell nesting, em-dash rule, task boxes | rtwk gap G73 | 46 (markdown_streaming_2 25, markdown_streaming_4 15, markdown_streaming_1 4, markdown_streaming_3 1, renderer_internals 1) | M | none | no |
| 2 | `composer-alone`: oracle snapshot is the composer/popup widget alone at N rows; the chat always draws the page (n/a or entry decision) | person decision | 13 (composer_popups 6, composer_2 2, approvals_2 1, composer_3 1, transcript_view_2 1, transcript_view_3 1, transcript_view_5 1) | S | chunk-composer | no |
| 3 | `shortcut-overlay`: `?` shortcut overlay above the composer | feature | 13 (pane_footer_1 7, composer_1 3, small_dialogs_2 2, pane_footer_2 1) | S | chunk-status | no |
| 4 | `patch-cells`: patch / apply cells and file-change approvals in the chat | feature | 12 (chat_status_2 5, chat_approvals_1 4, cells_2 1, composer_2 1, transcript_view_2 1) | S | chunk-cells | no |
| 5 | `keymap-read`: sheets/pickers/composer/transcript read the keymap (`.keymap(..)` seed exists; only the command center reads it) | feature | 34 (approvals_1 6, approvals_2 6, small_dialogs_2 5, transcript_view_1 4, chat_approvals_2 2, chat_commands_1 2, chat_popups_4 2, composer_4 2, chat_commands_3 1, chat_live_1 1, chat_popups_1 1, composer_2 1, resume_2 1) | M | none | no |
| 6 | `resume-entries`: resume picker debug entries for row data the picker already draws | debug entry | 11 (resume_2 6, resume_1 5) | S | none | yes |
| 7 | `tool-cells`: MCP / CUA / dynamic tool-call cells (typed result blocks, computer-activity groups, image results) | feature | 30 (cells_1 11, cells_4 11, chat_status_1 6, cells_3 1, chat_status_3 1) | M | chunk-cells | no |
| 8 | `inline-vs-owned`: oracle asserts inline display_lines previews (`… +N lines`), the port draws the owned transcript's compact rows (S27) | person decision | 10 (cells_1 9, cells_5 1) | S | chunk-cells | no |
| 9 | `ps-cell`: /ps background-terminal list cell | feature | 10 (cells_3 7, chat_approvals_1 3) | S | chunk-cells | no |
| 10 | `windows-only`: Windows-only sandbox prompts and labels (oracle runs them on Windows) | person decision | 10 (chat_popups_2 10) | S | chunk-popups | no |
| 11 | `reasoning`: reasoning summary cells, reasoning status rows and streaming headers | feature | 28 (chat_popups_4 12, chat_status_2 6, cells_5 4, chat_status_1 3, chat_popups_1 2, chat_live_4 1) | M | chunk-cells | no |
| 12 | `feedback`: feedback flow: note step entries, success cells, consent diagnostics | feature + debug entry | 27 (small_dialogs_1 16, chat_popups_4 4, transcript_view_1 3, chat_status_1 2, chat_commands_3 1, transcript_view_2 1) | M | chunk-popups | no |
| 13 | `mcp-inventory`: /mcp inventory/tools cell + mcpServerStatus/list seed | seed + feature | 9 (cells_4 4, cells_3 3, cells_1 2) | S | chunk-cells | no |
| 14 | `local-flow-cells`: cells from local flows (approval answers, stdin interaction, recap, request-user-input results) | feature | 9 (cells_4 6, cells_2 1, cells_5 1, transcript_view_1 1) | S | chunk-cells | no |
| 15 | `reconnect`: disconnect/reconnect seeds and paused input | seed + feature | 9 (chat_commands_1 3, composer_4 3, transcript_view_1 2, chat_commands_3 1) | S | chunk-commands | no |
| 16 | `rename`: Name thread prompt (/rename, suggestion) | feature | 9 (chat_commands_2 6, chat_live_1 1, chat_popups_4 1, pane_footer_2 1) | S | chunk-commands | no |
| 17 | `mentions-plugins`: `@` plugin mentions (set_plugin_mentions v2), plugin accent, bound mentions | seed + feature | 9 (composer_1 6, composer_2 2, composer_4 1) | S | chunk-composer | no |
| 18 | `keymap-picker`: Picker reads the list keymap (landed 4a812ce3d in chunk-popups): picker-shard keymap tests whose reason still says no keymap seed (stale-reason candidates; re-probe) | stale pending | 9 (picker 9) | S | chunk-popups | no |
| 19 | `decision-cells`: approval decision cells (`✔ You approved ...`) in the chat history + network approvals + guardian wiring | feature | 26 (chat_approvals_2 11, chat_live_1 6, chat_approvals_1 5, approvals_1 2, approvals_2 2) | M | none | no |
| 20 | `g65-textarea`: G65: rtwk TextArea wrap/caret at full rows, wide graphemes, tabs, rect clear/scroll | rtwk gap G65 | 26 (textarea 26) | M | none | no |
| 21 | `apps`: /apps connector directory popup + app-link dialogs (manage/install/enable/auth/url entries) | feature | 25 (small_dialogs_1 14, chat_popups_3 6, chat_popups_4 5) | M | chunk-popups | no |
| 22 | `copy-export`: /copy picker, copy action and its feedback, /export | feature | 24 (chat_commands_2 13, chat_commands_3 7, transcript_view_4 2, cells_3 1, chat_popups_1 1) | M | chunk-commands | no |
| 23 | `warnings`: warning cells on the main screen (Item::Warning detailed-only), warning history, startup warnings panel, invalid status-line warning | feature | 8 (cells_3 2, chat_status_1 2, cells_5 1, chat_live_1 1, chat_popups_4 1, small_dialogs_2 1) | S | chunk-live | no |
| 24 | `unified-exec-footer`: background terminal (unified exec) footer + waiting status | feature | 8 (chat_approvals_1 6, pane_footer_2 2) | S | chunk-status | no |
| 25 | `vim`: composer Vim mode (normal/insert/replace, yank/put/dot, search prompt, footer label, vim_mode_default) | feature | 59 (composer_4 33, composer_1 10, composer_popups 5, composer_3 4, approvals_1 1, chat_approvals_2 1, chat_commands_1 1, chat_live_4 1, textarea 1, transcript_view_1 1, transcript_view_2 1) | L | chunk-composer | no |
| 26 | `sparkle`: composer sparkle / blossom starfield of a fresh task (15 s idle, 1 s fade) | feature | 21 (pane_footer_1 18, chat_commands_3 3) | M | chunk-status | no |
| 27 | `status-indicator`: status indicator row (Working header, timer, details, interrupt hint, animations/shimmer off, small heights) | feature | 21 (chat_status_1 7, chat_status_3 6, pane_footer_2 5, chat_status_2 3) | M | chunk-status | no |
| 28 | `picker-variants`: Select/Picker variants: multi-select + reorder, browser list, description columns, side content, row styles | feature | 21 (picker 21) | M | chunk-popups | no |
| 29 | `approval-entries`: approval sheet entries with missing states (writeStdin, multiline prefix, README patch, pager over verification) | debug entry | 7 (approvals_2 4, chat_approvals_1 3) | S | none | no |
| 30 | `startup-card`: startup card help/Tip rows (show_tooltips, tips catalog) and version/update notices | feature | 7 (cells_3 4, cells_4 3) | S | chunk-cells | no |
| 31 | `error-notice-cells`: cyber-policy / safety-access / daybreak / model-verification notice cells | protocol arm | 7 (cells_3 5, chat_status_1 2) | S | chunk-cells | no |
| 32 | `composer-history`: composer prompt history (Up recall, slash-command history, Ctrl+R preview over drafts) | feature | 7 (chat_commands_3 3, composer_3 2, chat_approvals_2 1, pane_footer_2 1) | S | chunk-composer | no |
| 33 | `collab-agents`: collab / sub-agent activity items and subagent streams | protocol arm | 7 (chat_live_1 4, chat_status_3 2, pane_footer_2 1) | S | chunk-live | no |
| 34 | `render-prefs`: rendering preferences seed (tui.rendering: mermaid, math, tables, lists) | seed | 7 (markdown_streaming_2 3, markdown_streaming_5 2, chat_popups_4 1, transcript_view_2 1) | S | chunk-cells | no |
| 35 | `composer-images`: composer image rows: remote image URLs seed, attach/delete/reorder, blocked/failed image restore | seed + feature | 20 (chat_commands_1 9, composer_3 4, chat_commands_3 1, chat_live_3 1, chat_live_4 1, composer_2 1, composer_4 1, pane_footer_2 1, transcript_view_1 1) | M | chunk-composer | no |
| 36 | `footer-entries`: pane_footer_2 debug entries chat/* for footer link lines, banners, voice strip, centered dialog, stacked views | debug entry | 20 (pane_footer_2 20) | M | chunk-status | no |
| 37 | `composer-status-rows`: composer status rows (extension status line, Plan mode row, agent label, context line) | feature | 19 (pane_footer_1 19) | M | chunk-status | no |
| 38 | `resume-modes`: resume picker modes: dense density, search query, ctrl+e expansion, ctrl+t transcript loading, archive, filters, zebra, fork | feature | 48 (resume_2 26, resume_1 22) | L | none | no |
| 39 | `goal`: goals: /goal command, goal menu, edit/resume prompts, goal status indicator, budget-limited turn | feature | 18 (chat_popups_1 8, chat_commands_2 3, chat_live_2 2, chat_live_5 2, chat_status_1 1, chat_status_2 1, chat_status_3 1) | M | chunk-popups | no |
| 40 | `mentions-skills`: `$` skill catalog seed + skill mentions/bindings/popup | seed + feature | 18 (composer_1 14, chat_live_5 1, composer_3 1, composer_popups 1, pane_footer_2 1) | M | chunk-composer | no |
| 41 | `url-wrap`: URL-aware wrap (adaptive_wrap_line: a URL token kept whole) | feature | 6 (renderer_internals 4, cells_5 1, markdown_streaming_1 1) | S | chunk-cells | no |
| 42 | `kit-sent-ops`: kit probe of sent ops (UserTurn text, Op::Interrupt, FollowTranscript) | kit capability | 6 (chat_commands_1 6) | S | chunk-commands | no |
| 43 | `g63-newlines`: G63: MarkdownView joins single newlines of transcript fixtures into one paragraph | rtwk gap G63 | 6 (transcript_view_1 5, transcript_view_2 1) | S | none | no |
| 44 | `hooks`: hook activity rows, hook history cells, hook status, /hooks popup | feature | 17 (chat_status_2 14, approvals_1 1, chat_popups_3 1, chat_status_3 1) | M | chunk-cells | no |
| 45 | `md-options`: markdown options in the chat: list spacing, table tail controller, follow-up directives, link display, code/link annotations | feature | 17 (markdown_streaming_1 9, markdown_streaming_5 4, cells_3 1, chat_live_1 1, renderer_internals 1, transcript_view_1 1) | M | chunk-cells | no |
| 46 | `tx-entries`: transcript cell entries/script shapes (tabs, wide graphemes, links, recaps, tool groups) for selection/resize tests | debug entry + feature | 17 (transcript_view_5 17) | M | chunk-transcript-2 | no |
| 47 | `status-line`: configured status line items (tui.status_line: model, git/PR, branch, provider, context) + terminal title | feature + seed | 44 (chat_status_2 22, chat_live_2 4, chat_live_1 3, chat_status_1 3, chat_status_3 3, chat_live_3 2, chat_commands_3 1, chat_popups_1 1, composer_1 1, composer_3 1, composer_popups 1, transcript_view_4 1, transcript_view_5 1) | L | chunk-status | no |
| 48 | `rtwk-text-break`: rtwk text breaking: G44 halfwidth kana cell, G48 hyphen, G49 slash, G72 autolink in plain text, G74 ellipsis, G75/76 UAX14 | rtwk gap G44/G48/G49/G72/G74-76 | 16 (approvals_1 3, approvals_2 2, startup_prompts 2, status 2, cells_1 1, cells_4 1, cells_5 1, picker 1, signin 1, small_dialogs_1 1, small_dialogs_2 1) | M | none | no |
| 49 | `completion-sep`: final-message separator `Worked for` + runtime metrics + viewer clock/date | feature | 16 (cells_2 9, cells_3 3, chat_approvals_1 1, chat_live_1 1, chat_live_2 1, chat_status_2 1) | M | chunk-cells | no |
| 50 | `statusline-popups`: /statusline and /title setup popups with previews | feature | 16 (chat_status_3 16) | M | chunk-status | no |
| 51 | `composer-states`: composer states: embedded plain-text, draft handoff, disconnected/offline, unavailable thread, input disabled, shutting down, parent-owned | feature + seed | 15 (composer_4 8, composer_2 2, composer_3 2, chat_commands_1 1, composer_1 1, transcript_view_1 1) | M | chunk-composer | no |
| 52 | `live-misc`: live chat states: compaction header, Luna Reserve, provider error, active cell heights, session header seeds, rename-by-thread name | feature | 15 (chat_live_1 7, chat_live_2 3, chat_live_4 2, chat_live_5 2, chat_popups_1 1) | M | chunk-live | no |
| 53 | `external-writer`: external-writer thread view (notice in place of the composer, fork progress) | feature | 5 (chat_live_1 4, chat_approvals_1 1) | S | chunk-live | no |
| 54 | `plugins`: /plugins browser (marketplace tabs, rows, detail, install/remove) + Feature::Plugins | feature | 38 (chat_popups_3 34, chat_popups_2 4) | L | chunk-popups | no |
| 55 | `kit-env-seed`: kit env seeds: terminal default colours / palette, FORCE_COLOR, syntax theme, a file on disk (png), foreign cwd, real clock/today | kit capability | 14 (markdown_streaming_3 2, pane_footer_2 2, transcript_view_5 2, cells_1 1, cells_2 1, cells_5 1, composer_1 1, composer_4 1, composer_popups 1, markdown_streaming_2 1, picker 1) | M | none | no |
| 56 | `slash-misc`: slash commands not dispatched: /init, /stop, /clear, /archive, /delete, /memories, /keymap debug, /pwd, /rollout, /app, stubs, /usage signed-out, /cd | feature | 14 (chat_commands_3 9, chat_commands_2 3, chat_approvals_1 1, composer_2 1) | M | chunk-commands | no |
| 57 | `footer-modes`: composer footer modes: esc/quit hints, `100% context left`, queue hint, flash, hint override, width collapse, warning notice | feature | 14 (composer_1 5, pane_footer_1 4, composer_popups 3, composer_4 2) | M | chunk-status | no |
| 58 | `pending-preview`: pending input preview + status/queue stack entries (queued messages, pending steers) | debug entry | 14 (pane_footer_2 14) | M | chunk-status | yes |
| 59 | `tx-find`: transcript Find (F3 / `/`): query editor, matches, find footer, live match | feature | 37 (transcript_view_4 26, pane_footer_1 4, transcript_view_1 3, transcript_view_2 2, transcript_view_3 1, transcript_view_5 1) | L | chunk-transcript-2 | no |
| 60 | `pets-kitty`: ambient pets: kitty image placements, pet width reservation, /pets picker and its unsupported-terminal notices | feature + kit capability | 13 (chat_status_1 6, chat_commands_3 5, transcript_view_1 2) | M | chunk-commands | no |
| 61 | `rtwk-editing-annot`: rtwk-editing has no hyperlink/highlight annotation: composer OSC 8 URLs, highlight ranges, per-range style (G-new-1) | rtwk gap (new) | 13 (textarea 12, composer_popups 1) | M | none | no |
| 62 | `custom-prompt`: CustomPromptView (Edit goal / rename prompt, suggestion prefill) entries | feature + debug entry | 13 (small_dialogs_1 13) | M | chunk-popups | no |
| 63 | `exec-cells`: exec/exploring/user-shell history cells to the oracle's ExecCell look (groups, orphan ends, waiting rows, interrupted) | feature | 33 (chat_approvals_1 18, cells_4 4, cells_5 4, cells_3 2, chat_live_1 2, chat_live_5 1, transcript_view_1 1, transcript_view_5 1) | L | chunk-cells | no |
| 64 | `user-cell`: user-message cell (prompt gutter, wrap, markdown, image labels, spoken, shared background) | feature | 12 (cells_5 4, cells_2 3, chat_live_1 2, composer_4 1, transcript_view_2 1, transcript_view_5 1) | M | chunk-cells | no |
| 65 | `kit-snap-frames`: kit snapshot reader: bare-label frames, caret rows, joined multi-render snapshots, Debug dumps of styled lines, `!fg` check | kit capability | 4 (approvals_1 1, approvals_2 1, composer_popups 1, picker 1) | S | none | no |
| 66 | `turn-notices`: turn notices: interrupted turn messages, turn error wording (codex_error_info) | protocol arm | 4 (chat_live_1 3, chat_live_5 1) | S | chunk-live | no |
| 67 | `mention-fixtures`: composer popup fixtures: wide-glyph names, ten skills, category tags, i-dot expansions, empty-filter loading | seed | 4 (composer_popups 4) | S | chunk-composer | yes |
| 68 | `md-gnew`: markdown parser G-new: HTML block verbatim, bare-URL style, empty list item | rtwk gap G-new | 4 (markdown_streaming_2 2, markdown_streaming_4 2) | S | none | no |
| 69 | `rate-limit-prompts`: rate-limit/usage-limit prompts: switch prompt, workspace owner/member nudges, request-limit increase, banner fallbacks, usage notice | feature | 31 (chat_status_1 16, chat_live_2 5, cells_2 2, chat_popups_4 2, cells_5 1, chat_live_1 1, chat_live_4 1, chat_live_6 1, pane_footer_2 1, status 1) | L | chunk-live | no |
| 70 | `g73-table`: G73 item 3: MarkdownView table look (two-space columns, ━/─ rules, cell wrap, key/value record fallback) | rtwk gap G73 | 31 (markdown_streaming_4 12, markdown_streaming_5 8, markdown_streaming_1 4, markdown_streaming_2 3, markdown_streaming_3 2, cells_3 1, renderer_internals 1) | L | none | no |
| 71 | `permissions`: /permissions picker complete: discovery, full-access confirm, auto-review row, profiles, args, history cells, constrained presets | feature | 30 (chat_popups_2 25, chat_popups_1 4, chat_popups_4 1) | L | chunk-popups | no |
| 72 | `tx-mouse-select`: transcript mouse selection: drag, double/triple click, copy-on-select, PRIMARY, keyboard extension | feature | 30 (transcript_view_2 10, transcript_view_4 10, transcript_view_5 8, transcript_view_1 1, transcript_view_3 1) | L | chunk-transcript-2 | no |
| 73 | `history-replay-todo`: kit::todo (C4): thread history replay/projection in the live chat (forked-thread line, replayed reasoning/images/review prompts, snapshot formatter, composer-history seeding) | feature (todo) | 30 (chat_live_2 30) | L | chunk-live | no |
| 74 | `kit-input`: kit input: SUPER/kitty keys, shift+enter in pty, right button, shift/ctrl+click, triple click, key release, pointer move, u16 wheel | kit capability | 11 (transcript_view_4 5, composer_3 2, approvals_2 1, composer_4 1, transcript_view_3 1, transcript_view_5 1) | M | none | no |
| 75 | `popup-misc`: other pickers: worktree, quick dials, security-setup banner, skills menu, experimental/feature prompts, config error cell | feature | 11 (chat_popups_4 10, chat_live_2 1) | M | chunk-popups | no |
| 76 | `async-questions`: async questions pane in the composer (question editor, drafts, turn-end recovery, desktop answers) | feature | 28 (chat_approvals_2 15, approvals_1 12, approvals_2 1) | L | none | no |
| 77 | `tx-follow`: transcript follow control, browsing (Esc-Esc), footers, pinned prompt header, details overlay, activity disclosure | feature | 28 (transcript_view_2 11, transcript_view_5 7, transcript_view_1 5, transcript_view_3 5) | L | chunk-transcript-2 | no |
| 78 | `g61-math`: G61: unicode math renderer (inline, display, named delimiters, pending display) | rtwk gap G61 | 27 (markdown_streaming_1 13, markdown_streaming_5 8, markdown_streaming_2 6) | L | none | no |
| 79 | `raw-mode`: raw output mode (alt+r, HistoryRenderMode) and rich agent markdown cell cache | feature | 10 (cells_3 4, markdown_streaming_5 3, cells_2 2, chat_status_1 1) | M | chunk-cells | no |
| 80 | `side-conv`: side conversation (/side, /btw): gates, context labels, hidden-side footer | feature | 10 (chat_live_5 6, chat_commands_3 1, chat_live_1 1, chat_status_3 1, composer_popups 1) | M | chunk-live | no |
| 81 | `tx-copy-mode`: transcript copy mode (copy target, highlight, footer) + copy feedback | feature | 10 (transcript_view_2 9, transcript_view_4 1) | M | chunk-transcript-2 | no |
| 82 | `plan-mode`: Plan collaboration mode: mode switch, Implement-this-plan prompt, plan reasoning scope, /plan literal submit | feature | 25 (chat_live_4 16, chat_commands_1 2, transcript_view_1 2, chat_live_2 1, chat_live_5 1, chat_popups_1 1, composer_2 1, pane_footer_2 1) | L | chunk-live | no |
| 83 | `queue-steer`: queue and steer while a turn runs: Tab queues (slash/!/text), pending steers, Esc interrupt restore, before-session queue | feature | 24 (chat_live_5 7, chat_commands_2 5, chat_commands_1 4, chat_commands_3 3, chat_live_4 2, chat_approvals_1 1, chat_live_1 1, chat_status_1 1) | L | chunk-live | no |
| 84 | `mentions-files`: `@` file search index/results + task mentions in the composer | seed + feature | 9 (composer_3 4, composer_2 3, chat_commands_1 2) | M | chunk-composer | no |
| 85 | `review-flow`: /review presets popup, commit/branch pickers, review mode steer/queue | feature | 9 (chat_live_5 7, chat_live_1 1, chat_popups_4 1) | M | chunk-popups | no |
| 86 | `thread-billing`: thread billing: thread-usage refresh, estimated cost / credits in status line and title | feature | 9 (chat_status_2 6, chat_live_6 3) | M | chunk-usage | no |
| 87 | `effort-footer`: reasoning-effort footer: ULTRA/MAX ignition, prompts, status-line transition frames | feature | 9 (pane_footer_2 6, pane_footer_1 3) | M | chunk-status | no |
| 88 | `untraced`: untraced probe differences (cause not written) | stale pending | 3 (cells_5 2, cells_4 1) | S | chunk-cells | no |
| 89 | `images-cells`: image artifact cells (view image, image generation) | feature | 3 (chat_approvals_1 3) | S | chunk-cells | no |
| 90 | `voice`: realtime voice transcript rows / voice strip | feature | 3 (chat_commands_3 1, chat_status_1 1, pane_footer_2 1) | S | chunk-status | no |
| 91 | `mcp-startup`: MCP startup state on the chat (booting header, review/fork during startup) | feature | 3 (chat_live_3 3) | S | chunk-live | no |
| 92 | `selection-style`: selected row style: theme accent+bold vs the oracle's unprobed-palette reverse | person decision | 3 (small_dialogs_2 2, small_dialogs_1 1) | S | none | no |
| 93 | `kit-history-page`: history paging: a script step answering thread/turns/list + ThreadModel loading states (partial, loading older/beginning, failed, page join) | kit capability + feature | 22 (transcript_view_3 6, pager 4, transcript_view_5 4, transcript_view_1 3, cells_2 2, transcript_view_4 2, transcript_view_2 1) | L | chunk-transcript-2 | no |
| 94 | `safety-misalign`: safety buffering (model/safetyBuffering/updated) and misalignment precaution flows | protocol arm + feature | 8 (chat_live_1 6, chat_live_3 2) | M | chunk-live | no |
| 95 | `tx-live-tail`: live tail held under a reader, commit, reading anchor across resize/replace | feature | 8 (transcript_view_5 5, pager 2, transcript_view_1 1) | M | chunk-transcript-2 | no |
| 96 | `reset-flow`: rate-limit reset credits flow (/usage menu, reset picker, confirmation, redemption) | feature | 21 (chat_live_6 21) | L | chunk-usage | no |
| 97 | `rtwk-layout`: rtwk layout gaps: G39b AutoVisible column, G50/G51 flex leftover, G54 explicit Reset style, G55 obscure reveal | rtwk gap G39b/G50/G51/G54/G55 | 7 (approvals_2 2, picker 2, onboarding 1, signin 1, startup_prompts 1) | M | none | no |
| 98 | `g68-popup-rows`: G68: composer popups as pane rows (aligned name/description columns, `›` marker, hint line) not a floating OptionList | rtwk gap G68 | 7 (composer_popups 5, composer_2 2) | M | none | no |
| 99 | `model-picker`: model catalog seed + /model picker, reasoning popup, shortcuts, session-only pick, Bedrock/custom models, service tiers | seed + feature | 16 (chat_popups_4 12, chat_popups_1 2, chat_commands_2 1, picker 1) | L | chunk-popups | no |
| 100 | `tx-select`: transcript keyboard select mode (caret, v/V, word/line motion, y copy) + ItemTexts | feature | 16 (transcript_view_4 12, pager 4) | L | chunk-transcript-2 | no |
| 101 | `running-seed`: seeds of a running/finished turn and model state for submit paths (unavailable model, no image input, hidden shell paste) | seed | 6 (chat_commands_1 3, chat_commands_3 3) | M | chunk-commands | no |
| 102 | `plan-cells`: proposed plan / plan update cells | feature | 2 (cells_3 1, chat_status_1 1) | S | chunk-cells | no |
| 103 | `external-editor`: G67 external editor ($EDITOR suspend) and fullscreen composer | rtwk gap G67 | 2 (composer_4 1, transcript_view_1 1) | S | none | no |
| 104 | `warnings-viewer`: warnings viewer/settings sheets: OSC 8 on wrapped URLs, dismissed set, status_line_pr entries | feature | 2 (small_dialogs_2 2) | S | none | no |
| 105 | `delayed-approval`: approval request in the live chat over a typing composer: 1 s typing-idle delay, typed shortcuts kept, deny while running, covering question overlay | feature | 5 (pane_footer_2 5) | M | chunk-live | no |
| 106 | `startup-draft`: startup draft / pre-session layout (startup submission, initial prompt, welcome decoration, onboarding hold) | feature | 12 (chat_commands_3 10, chat_commands_1 2) | L | chunk-commands | no |
| 107 | `composer-mouse`: composer mouse selection / yank over a drag | feature | 1 (composer_4 1) | S | chunk-composer | no |
| 108 | `g62-mermaid`: G62: native mermaid renderer and its holdback | rtwk gap G62 | 4 (markdown_streaming_5 3, markdown_streaming_2 1) | L | none | no |
## Notes for planning

- **Stale-reason candidates** (worth one probe per shard by the owner, not a cluster of work):
  - `keymap-picker` (9, picker shard): the Picker reads `tui.keymap.list.*` since 4a812ce3d
    (chunk-popups), yet the reasons still say "no keymap seed". The `.keymap(..)` seed exists
    (`tests/kit/seed.rs:111`). The other 34 `keymap-read` reasons mean "this widget reads no
    keymap" (approvals sheets, dialogs, transcript, composer). That work is one cross-widget
    pattern: read `crate::keymap::keymap(cx)` the way `src/widgets/picker/mod.rs:364` does.
  - 15 cells/tool reasons say "core threads::project maps only user and agent messages".
    `core/src/threads.rs` and `core/src/threads/activity.rs` now decode command and MCP items,
    so the stated cause is out of date. The cells themselves are still chunk-cells steps.
  - 34 reasons start "OTHER CHUNK"; they are counted where their feature lives (exec-cells 18,
    unified-exec-footer 6, ...).
  - `untraced` (3, cells): the reason is "cause not traced".
- **Entries no screen registers**: 170 pending tests name an `.entry(dir, name)` that is not
  in any `debug_entries()`. The largest are resume-modes 32, footer-entries 19,
  pending-preview 14, apps 14, custom-prompt 13, async-questions 12 and resume-entries 11.
  Most of them need the widget state first. Only the resume-entries family is pure data.
- **todo** (30, all chat_live_2): history replay and projection (`history-replay-todo`). This
  is chunk-live's replay step, C4.
- **Person decisions** (36): `composer-alone` 13 (the oracle snapshots the bare composer or
  popup), `inline-vs-owned` 10 (S27 inline display_lines), `windows-only` 10,
  `selection-style` 3.

## Haiku pilot

No uniform cluster of 30-50 tests exists. The big clusters are features: each needs widget or
model work before any item is data. The uniform clusters are `resume-entries` 11,
`pending-preview` 14 and `mention-fixtures` 4. Only resume-entries is in an area no porter
edits.

### Pick 1: `resume-entries` (11 tests, owner none, the resume worktree has been idle 12 h)

Tests (all in `codex-rs/tui-rtwk/tests/oracle/`):
- resume_1: `local_db_first_falls_back_when_initial_db_page_is_empty` (entry `db_fallback`),
  `footer_marks_missing_branch` (`footer_no_branch`), `footer_branch_expands_when_line_has_room`
  (`footer_long_branch`), `footer_cwd_truncates_to_responsive_column` (`footer_long_cwd`) and
  `hint_line_compacts_on_narrow_width` (`footer_narrow`).
- resume_2: `narrow_session_snapshot`, `moving_to_last_card_scrolls_when_cards_exceed_viewport`,
  `up_from_bottom_keeps_viewport_stable_when_card_remains_visible`,
  `up_scrolls_only_after_crossing_top_edge`, `partial_failure_keeps_healthy_rows_visible`
  (`partial_failure`) and `later_page_error_preserves_loaded_sessions`.
- Two may need view work rather than data: partial_failure and later_page_error. Each wants
  rows kept beside a search-line error, but `Ev::Failed` today means a whole-listing failure
  (`core/src/screens/resume.rs:84`). The filler reports them as "needs view" if the screen
  does not draw both.

Shape of one item: the test opens `Screen::build(..).size(w,h).entry("resume", NAME).open()`
and checks text or a snapshot region. The item is a JSONL fixture that seeds the session and
then emits resume screen events:
`{"step":"seed","seed":"session","thread":"thread-1"}`,
`{"step":"screen","dir":"resume","ev":{"clock":EPOCH}}`, and
`{"step":"screen","dir":"resume","ev":{"listed":{"rows":[{id, preview, created, updated, cwd, branch, thread_name, cloud, unread}]}}}`,
or `{"failed":"msg"}` / `{"loading":null}`. Row fields are `RolloutSeed`
(`core/src/screens/resume.rs:225-247`). The oracle's row data is in the oracle test the
`// ORACLE resume_picker.rs:LINE` comment names (codex-rs/tui, read only).

Finished sample (passing, same shape):
- test `tests/oracle/resume_1.rs:166-177` `mixed_backend_resume_table_snapshot`
- entry `src/pages/resume/mod.rs:54-59` (`table`)
- fixture `src/pages/resume/fixtures/table.jsonl`

Also passing: `resume_2.rs:135` (`more_indicators`) and `resume_1.rs:209` (`footer_wide`).

Per item the filler writes only `codex-rs/tui-rtwk/src/pages/resume/fixtures/<entry>.jsonl`.
It writes no other file.

It must NOT write these shared registry lines. It returns them instead as REGISTER lines,
which one sequential step (the orchestrator, or a last filler) applies:
- `src/pages/resume/mod.rs` `debug_entries()`: one block per entry, copied from lines 54-59:
  `DebugEntry { name: "<entry>", description: "<one line>", script: include_str!("fixtures/<entry>.jsonl"), open: EntryOpen::Route(|_: &Seeded| Screen { live: false }.into()) },`
- `tests/oracle/resume_{1,2}.rs`: delete the `kit::pending(..)` call of the test.
  For the five resume_2 tests whose body names no entry yet, the REGISTER line also gives the
  `.entry("resume", "<entry>")` line.

Area touched: `src/pages/resume/` and `tests/oracle/resume_*.rs`. No running chunk holds them
(`active_claims`: none). Verification is the resume shard only.

### Pick 2 (only with chunk-status's consent): `pending-preview` (14, pane_footer_2:810-1018)

These are uniform entries `chat/render_*` for the queued/steer preview (`render_one_message`,
`render_two_messages`, `render_wrapped_message`, `render_one_pending_steer`, ...). Each item is
one chat fixture with queued messages or steers, plus a REGISTER line. The registry is
`src/pages/session/mod.rs:45` `debug_entries()` (chunk-live's area). The widget is
`src/pages/session/pending_input.rs` (chunk-status's area). The pilot can start only once
chunk-status declares PendingInput at the oracle layout. Otherwise the fixtures pass nothing.
Hand it to chunk-status, or run it right after chunk-status lands.

### Not a pilot

`mention-fixtures` (4, chunk-composer), approvals user_input entries (they are
async-questions, a feature), and feedback, app-link and custom-prompt entries (each needs its
widget first, and they are chunk-popups' area).
