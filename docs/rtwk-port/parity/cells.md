# chat-cells — parity
Shard counts (summary.py over the oracle profile junit; on arrival 57 pass / 12 n/a / 420 pending / 1 red of 490):
cells_1 9/0/51 · cells_2 8/1/24 · cells_3 1/0/38 · cells_4 5/0/34 · cells_5 7/1/33 ·
markdown_streaming_1 19/4/35 · _2 18/1/41 · _3 7/0/33 · _4 6/0/37 · _5 22/3/33 ·
renderer_internals 12/2/5 — total 114 green / 12 n/a / 364 pending / 0 red (pass/n-a/pending).
Core: core/tests/cells.rs 5 tests (FileChange, failed status, reasoning summary, web labels, plan/image/notices);
bite: `cells::entry` stubbed to None, 0/5. Pages: tests/pages/cells.rs patch disclosure (indent, tap) red on the old code.
Decisions:
- Item kinds Reasoning, Patch, Plan, Web, ImageView, Notice are projected once in core/src/threads/cells.rs and
  enter ThreadModel only through apply (ADR-003, R4). The cells are rtwk widgets in src/pages/session/cells/
  (patch: Column of Rows, fixed gutters as SizedBox, `Diff::parse` + `Highlighter`, disclosure a Pressable in a
  StatefulWidget; web/image: one Text.rich, max_lines 1, Ellipsis; plan: BoxWidget on the prompt surface around
  MarkdownView; reasoning: DefaultTextStyle dim italic, detailed transcript only).
- Table-holding md fences are lifted before MarkdownView (cells/markdown.rs, the oracle's unwrap_markdown_fences,
  a source rewrite before the parser): renderer_internals collector test and 6 fence tests green.
- The exec cell keeps the existing ToolRow; ~14 cells_1 exec tests stay pending (exec rework not in this pass).
- Kit limit (not an rtwk gap): markdown_render rows are the renderer's alone, the chat adds the agent gutter
  `• `/`  `, so most markdown_streaming tests stay pending with their old reasons; tests/kit is read-only here.
- Link destinations: the pin (rtwk main 5583c142) has `MarkdownView::link_destinations`; the chat draws `LinkDestinations::Auto` (a web label
  alone with its hyperlink, a local target with ` (target)`); the oracle's web suffix is for a terminal not known to support OSC 8 and stays pending (repin-4.md).
- cells_2 compact_patches…failure_details pending: the wire carries no failure stderr (patch half matches);
  finalized_plan_reuses_rendered_transcript_lines pending: spinner/cache frame equality.
Gap rows added (rtwk-gaps.md, provisional ids): G60 MarkdownView inline syntax extension (file citations),
G61 math renderer, G62 mermaid renderer, G63 soft line break as a line break (softLineBreak),
G64 nested list indent (listIndent).
Binary: mock-model check.py (pty 120x40) text-100, markdown, reasoning, exec-command, patch-approval, web-search,
view-image, proposed-plan: 16/16 PASS both binaries. The "sentinel on screen: no" notes for rtwk (reasoning,
web-search, proposed-plan) are the check's raw-stream heuristic (rtwk writes changed cells only); a VT replay of
the captures shows the sentinel. Captured rows, rtwk vs oracle (header/footer chrome left out):
- reasoning: equal. text-100, exec-command: not compared row by row (the scratch VT replay has no scroll
  regions; both reached the sentinel).
- view-image: the oracle adds `    + Show details` under `• Viewed image example.png`, revealing the full path and
  the image (transcript_view layout.rs image artifact); ours has no control. Left: the reveal needs a file-read
  effect in services (views hold no IO) and an rtwk-rich image widget in the cell.
- patch-approval: equal after the disclosure fix (`    + Show details`).
- markdown: nested bullet column 4 vs 6 (G64); rest equal.
- proposed-plan: soft breaks joined (G63).
- web-search: the oracle commits `• Browsing the web` before `• Searched the web for …`; ours shows the
  completed search only. Open: the event order that makes the oracle flush its active cell is not traced yet.
Mouse: patch cell disclosure on tap (tests/pages/cells.rs tapping_the_patch_disclosure_reveals_every_line).
Rows for the lander: the colour of ordered-list markers (oracle LightBlue, MarkdownTheme.marker) lives in
src/app/theme.rs `transcript_markdown`, outside this area (markdown_streaming_3 deeply_nested_mixed_three_levels).
transcript_view_2 patch_disclosure_preserves_manual_scrolling (another area) can use the patch cell now.

## chat-cells-2 (fix round, 2026-11-21; unfinished, see "Left")
Shard counts (summary.py, oracle profile; chat-cells landed 114 pass / 12 n/a / 364 pending / 0 red of 490):
cells_1 9/0/51 · cells_2 8/1/24 · cells_3 1/0/38 · cells_4 5/0/34 · cells_5 16/1/24 ·
markdown_streaming_1 24/4/30 · _2 21/1/38 · _3 7/0/33 · _4 19/0/24 · _5 22/3/33 ·
renderer_internals 12/2/5 — total 144 pass / 12 n/a / 334 pending / 0 red / 0 stale (the probe, KIT_PROBE_PENDING=1,
lists 23 stale: pending tests whose body is weaker than the oracle's claim, each reason says so).
Findings of the FULL review of chat-cells:
1. Reasoning header split in core (`summary_text`, oracle split_reasoning_summary_parts); core/tests/cells.rs rewritten
   (bite: old code returns the header inside the body); 9 cells_5 reasoning tests un-pended (the feature-flag and config ones keep their seed reason).
2+4. Patch preview: the gutter is measured over every hunk line, the context is then left out (tests/pages/cells.rs
   patch_preview: `@@ -8,3 +8,3 @@` gives the 2-column gutter) and a file stops at three SCREEN rows (a column under
   `ConstrainedBox` max height 3, `overflow_intended`); both tests red on the old code.
3. Reasoning dim italic: `DefaultTextStyle` above `MarkdownView` has no effect (style assertion: plain, bold and code cells
   carried no DIM|ITALIC). The cell now dims through the `MarkdownTheme` (strong, emphasis, code, link, headings, quote, marker:
   tests/pages/cells.rs reasoning_style) and the plain words stay undimmed: gap G71, pinned by `probe_g71_...`.
5. `PatchCellState.expanded` was lost when a timing row arrived (the row changed shape). Every transcript row is now one `Column`
   keyed by the item (cell first, optional timing row second); tests/pages/cells.rs patch_in_transcript (red with the old shape).
6. The pending reasons were stale for a kit reason, not a missing renderer: the oracle's Markdown tests are bare rows, the chat
   draws the agent cell (two gutter columns and a free column). `Open::markdown(width, height)` (tests/kit/markdown_gutter.rs)
   opens the terminal at the oracle's width and reads the screen without the gutter (`resize` keeps it). 34 stale markers removed,
   23 kept with the probe note, 99 false reasons replaced by the probe's first difference (or the theme colours, below), four
   nested-list tests pend on G64. One test whose oracle rows hold the bullet (controller_set_width_preserves_in_flight_tail) is read whole.
10. One widget per file: cells/{reasoning,web,notice,viewed_image,one_row}.rs; summary.rs is gone.
Item 11 (theme colours) is done in round 2 below.
Gap rows: G71 (MarkdownView paragraphs take no inherited text style). Renumbered from G65 (composer took G65-G70 on the tip);
the commit message of fb9220a1 still says G65.
Pin: rtwk main 5583c142 has link destinations (`MarkdownView::link_destinations`, baccefc2); see repin-4.md for how the chat uses it.

### Round 2 (same day, after startup-finish landed; items 11, 7, 9, 8 done)
Shard counts (oracle profile, rebased on 1171555ad = tip, nothing to rebase at the end): cells_1 9/0/51 · cells_2 10/1/22 · cells_3 1/0/38 ·
cells_4 5/0/34 · cells_5 16/1/24 · markdown_streaming_1 27/4/27 · _2 29/1/30 · _3 12/0/28 · _4 20/0/23 · _5 22/3/33 · renderer_internals 12/2/5 —
total 163 pass / 12 n/a / 315 pending / 0 red / 0 stale of 490 (on arrival 57 / 12 / 420 / 1; after round 1 144 / 12 / 334 / 0).
Checks: fmt clean, clippy `--workspace --all-targets -D warnings` clean, whole nextest (not the oracle binary) 1124 pass / 15 skipped,
`shards.py --check` 55 shards, 0 stale. Core adds core/tests/{directives,citations}.rs (area widened for them) and two web tests in core/tests/cells.rs.
- 11 `src/app/theme.rs` `transcript_markdown`: ordered marker ANSI light blue (`marker`), quote text and bar green (`quote`, `quote_bar`);
  14 tests un-pended. `blockquote_with_list_items` stays pending: the oracle test runs with an empty syntax theme (the bullet takes the quote's
  green); the app's theme gives bullets the Mocha/Latte teal like the oracle's default and the kit cannot choose the syntax theme.
- 7 file citations: core/src/threads/{directives,citations}.rs (the oracle's directive scan, carried; destination encoding, file:// and drive paths,
  cwd-relative label) and cells/citation.rs (`view(source, cwd) -> MarkdownView`: the source is parsed with the directives masked, the way the
  oracle masks them, and the node that stands for one becomes a code node; code, HTML, link text and destinations, reference definitions stay
  literal). Agent and plan cells read citations, the reasoning summary does not (the oracle's generic renderer). G60 is solved for these two:
  the lander closes the row (rtwk-gaps.md is composer-2's file this run). The link-suffix tests waited for rtwk baccefc2 (the pin 5583c142 has `MarkdownView::link_destinations`; repin-4.md), the soft-break test is G63,
  the adjacent-entities test pends on G61.
- 9 web search: oracle `handle_thread_item` runs `on_web_search_begin` then `on_web_search_end` for an item/completed; begin commits the cell
  item/started left (`• Browsing the web`), so the pty shows both rows. `Transcript::upsert` keeps the started web entry and appends the
  completed one under the id `<id>#2` (a stored thread without a start stays one row). Core and page tests (red with the rule off).
- 8 viewed image: cells/disclosure.rs (the control, shared with the patch cell), `ViewedImageCell` stateful: compact row, `+ Show details`,
  expanded the full path under a two-column hanging indent and `− Hide details`, detailed transcript the path and no control; tap, Enter, Space.
  The oracle's path-in-details test now clicks the control. NOT done (named for the lander): the image link (`TerminalHyperlink::viewed_image`,
  needs canonicalize and is_file) and the image preview (`TranscriptImage::load`) read the file system; a views-without-IO design needs a
  services effect that answers "is this a local image, its bytes" into the model. Debt (chat-cells-2 phase 1): the session had no
  `task_create`, so the orchestrator creates it under the parent of chat-cells-2 (text in the handover): it owns the services effect
  (canonicalize / is_file / the oracle's workspace-locality gate, image load) answering through `apply`, then the link span and image widget.
Phase 1 rebase (onto 629c4cd84, composer-2's paste burst landed): `core/tests/all.rs` got `mod citations;` and `mod directives;`;
tests/pages/cells.rs types through `h.press` (which waits one key pace) so the raw `type_text("go")` + Enter is no burst; whole nextest
1198 pass / 15 skipped; oracle shards cells_1-5, markdown_streaming_1-5, renderer_internals 163 pass / 12 n/a / 315 pending / 0 red / 0 stale;
web-search and view-image PASS on both binaries again.
Binary (mock-model check.py, pty): web-search and view-image on both binaries PASS; a VT replay with scroll regions of the captures
(scratch /tmp/chat-cells-vt.py) gives, rtwk vs oracle, equal transcript rows: web-search `• Browsing the web` / `• Searched the web for rtwk
terminal ui framework` / `• Search done: …`, view-image `• Viewed image example.png` / `    + Show details`. Footer chrome differs (model name
line, the `scenario` title row, `⚠ 1 warning`): other areas.
Mouse: tests/pages/cells.rs viewed_image_disclosure (tap reveals, tap again hides).
Left (in order): the exec cell off `ToolRow` (~14 cells_1 tests, reasons say core ignores commandExecution items; check), hook-prompt, collab-agent,
sub-agent, sleep, image-generation (cells_3: "draws no imageGeneration item yet"), warning/error (cells_5), deprecation, plan-update and
session-info cells (core `Item` variants projected in core/src/threads/cells.rs, one widget per file, tests first); the 315 pending tests carry
the probe's first difference or a named kit reason: trace them cell kind by cell kind; the image link and preview (services effect, above).

## chat-cells-2 phase 2 (2026-11-21, rebased onto e8524be63; items 1-10 of the LEFT list)
Shard counts (summary.py, oracle profile, stdout in a file): cells_1 16/0/44 · cells_2 10/1/22 · cells_3 5/0/34 · cells_4 7/0/32 · cells_5 16/1/24 ·
markdown_streaming_1 27/4/27 · _2 29/1/30 · _3 12/0/28 · _4 20/0/23 · _5 22/3/33 · renderer_internals 12/2/5 —
total 176 pass / 12 n/a / 302 pending / 0 red of 490 (phase 1: 163 / 12 / 315). Probe (KIT_PROBE_PENDING=1): 23 stale, the same 23 as before,
each reason says "probe: stale, vacuous" (an absence check); `--check` of shards.py 55 shards, 0 files stale.
Checks: fmt clean, clippy `--workspace --all-targets -D warnings` clean, whole nextest 1347 pass / 15 skipped.
Items (commits after the rebase; tests first, one widget per file in `src/pages/session/cells/`):
- 1 exec (5f3ed35a7 off ToolRow in phase 1 of this list, da105ca13 after the rebase): command output rows read their SGR codes as styles
  (`transcript_activity.rs`), the command fixtures quote the script as the server does, `powershell_skill_read` is un-pended, the exec tests that read the
  inline `display_lines` get their true reason (the chat shows no inline cell). Left: the `cells/exec.rs` extraction (the rows are still drawn by
  `transcript_activity.rs`) and an un-clipped user-shell display.
- 2 hook (55dca9ead): `Item::Hook(HookRun)` (entries by kind), a minted key (the server has no item for a finished hook run), cell `hook.rs`; a
  quiet hook draws nothing; hook-prompt and sleep are rowless in the oracle. S11 windows of the hook and image tests are 24 rows.
- 3 collab, 4 sub-agent (aacf0df39, 2e03990ab): `Item::Collab(CollabEvent)` (spawn, sent input, wait, resume, close, sub-agent activity) with the oracle's
  labels, states and previews, `Item::Delegated(DelegatedMessage)` (`Sent by Codex from task …`), cells `collab.rs`, `delegated.rs`; check.py multi-agent (MOCK_AGENT_API=v2)
  rows equal on both binaries (below). The agents' nickname and role feed for labels is not built (rows show the thread id); the spawn-request cache is
  not implemented; previews are cut by chars, not graphemes.
- 5 sleep: rowless (oracle: `on_sleep` adds nothing). 6 image generation: `Item::Generated(GeneratedImage)` and `generated_image.rs`; the terminal preview of the file is the
  debt task `chat-cells-debt-viewed-image` (services effect), as for the viewed image.
- 7 warning and error (9e89120e0): `Item::Error` / `Item::Warning`; an error notification is a red `■ message` row, once per turn (a failed turn that carries
  the same message does not draw it twice, a failed turn without a message draws nothing: oracle `turn_runtime`); `Item::is_detailed_only` (Reasoning, Warning,
  Deprecation) drives the pane's row filter; the protocol `warning` shows in the detailed transcript only, the visible warnings of the oracle are the
  rate-limit snapshots (not built). `Transcript::replace_turn` now keeps minted rows (hook, plan update, error survive a Full snapshot); core/tests/model.rs
  (area widened) has the two failed-turn tests rewritten. `Failure::TurnFailed`, `strings.chat.turn_failed` and the `failure_text` arm are unused now:
  cleanup for the orchestrator (outside the area).
- 8 deprecation (f1abd9093): `Item::Deprecation{summary, details}`, detailed-only `⚠` cell; `core::threads::project_unrouted` turns what the server sends
  without a thread into the displayed thread's event. The routing arm is in core/src/app.rs, claimed by startup-device-code, so it is NOT applied: both cells_5
  notice tests stay pending with that reason. The patch below was proven (both tests pass, then reverted), `git apply --check` passes on this tip:
```diff
diff --git a/codex-rs/tui-rtwk/core/src/app.rs b/codex-rs/tui-rtwk/core/src/app.rs
index fa3d5ff56..37c68b8fe 100644
--- a/codex-rs/tui-rtwk/core/src/app.rs
+++ b/codex-rs/tui-rtwk/core/src/app.rs
@@ -745,21 +745,11 @@ fn apply_event(app: &mut CodexApp, origin: BackendId, ev: AppEv, fx: &mut AppFx)
                     }
                     return;
                 }
-                ServerNotification::Warning(note) if note.thread_id.is_none() => {
-                    Rc::make_mut(&mut app.session)
-                        .backend_notices
-                        .insert(origin, note.message.clone());
-                }
                 ServerNotification::ConfigWarning(note) => {
                     Rc::make_mut(&mut app.session)
                         .backend_notices
                         .insert(origin, note.summary.clone());
                 }
-                ServerNotification::DeprecationNotice(note) => {
-                    Rc::make_mut(&mut app.session)
-                        .backend_notices
-                        .insert(origin, note.summary.clone());
-                }
                 _ => {}
             }
             // The startup page's sign-in: a login completed, the account.
@@ -839,6 +829,14 @@ fn apply_event(app: &mut CodexApp, origin: BackendId, ev: AppEv, fx: &mut AppFx)
                     agents::observe(&mut Rc::make_mut(&mut app.screens).agents, &notification);
                 send_agents(app, outs, fx);
             }
+            // What the server sends without a thread (a deprecation notice) is
+            // the displayed thread's: the chat shows it in the detailed transcript.
+            if active_backend(app) == origin
+                && let Some(ev) = threads::project_unrouted(&notification)
+                && let Some(thread) = app.session.active.clone()
+            {
+                apply_thread(app, &thread, ev, fx);
+            }
             if let Some((thread, ev)) = threads::project(&notification) {
                 if app.threads.contains(&thread) {
                     apply_thread(app, &thread, ev, fx);
```
- 9 plan update (177af1d09): `Item::PlanUpdate` with a minted key, the status line follows the same report (core/src/screens/chat.rs: the
  `TurnPlanUpdated` projection arm is gone, `plan_progress` reads the transcript); cell `plan_update.rs` (compact rows with the steps done, the disclosure control the
  oracle's `activity_disclosure` asks for, note and steps revealed); the dim of the step rows is `Attrs::DIM`. Frames `Compact` and `Full` of the oracle are
  bare labels the kit cannot address; the rows are compared in tests/pages/cells.rs plan_update (`the_oracle_snapshot_full_rows` pending G49: no break after
  a slash).
- 10 session info (73798f4f4): the three `session_header_*` tests run on a `{"step":"screen","dir":"chat","ev":{"status":{...}}}` seed (cwd, profile, approval
  policy, version): YOLO and narrow pass on the existing `transcript_header.rs` (no code change), the halfwidth directory stays pending on G44 (the sound mark
  clusters into one cell). Not built (the pending reasons say so): the startup card of the oracle (`new_session_info`: help lines of the first session and one
  `Tip:` row, `show_tooltips`, tips catalog, model-availability NUX) is a feature of its own (core needs a first-session fact, the setting, the catalog;
  the host reads the config), and the history paging states of the session-information tests (the header is hidden while `can_load_history`; the kit has no
  `thread/turns/list` script step). The state tests `yolo_mode_includes_managed_full_access_profiles` / `..._excludes_external_sandbox_profiles` belong to
  the host (`Profile` is produced there; the port has no External variant).
Binary (mock-model check.py, pty, rtwk and oracle, rows replayed through /tmp/chat-cells-vt.py): PASS on both for multi-agent (v2), plan-tool,
image-generation, error-fatal, error-stream-closed, exec-command, web-search, view-image; error-context-window is the known red on both. Transcript rows
equal for: multi-agent (`Started /root/helper`, `Waiting for agents`, `Completed /root/helper`, `Finished waiting`, `└ No agents completed yet`), plan-tool
(three `Updated Plan` cells and the `+ Show details` control), exec-command, error-fatal (`■ Quota exceeded. Check your plan and billing details.`).
Differences outside this area: the footer (model name casing `GPT-6-Sol`, `fast`, the `scenario` title row, `⚠ N warning · f2 to view`).
FINDING for chat-live (streaming, not cells): error-retry. The oracle's final screen drops the cut-off first attempt; rtwk keeps its streamed text as a row
(`• This answer is cut off by`) above `• Second try worked.` (the item the server started never completed; the oracle shows only the completed one).
Mouse: tests/pages/cells.rs (disclosure taps of the viewed image and the plan cell). The collab and error cells are not selectable: no new mouse path.
Handover list: G44 (halfwidth), G48 (hyphen break), G49 (no break after a slash), G71 (inherited text style); the kit lacks bare-label frames; oracle magenta is the
theme accent (`tokens.accent()`); middle truncation by screen rows and unbroken wide URL tokens may be gaps (not pinned); chat_live_1 WARN_PENDING /
ERR_PENDING / COLLAB_PENDING tests are chat-live's (some may pass now: not probed); an incident of disk full (ENOSPC) in phase 1 was cleared by pruning ~18 GB
of this worktree's own target artifacts.

## chat-cells-2 lander fixes (2026-11-21, FULL review LAND WITH FIXES)
- 1 turn error ends the turn (oracle `on_error`): `ThreadEv::TurnError` now runs `end_turn` (finalize, next queued input) then `report_error`; the failed turn's
  `TurnCompleted` afterwards is a no-op (no second error row, `send_next` once: `finished_turns` / idle turn). Core test
  `a_turn_error_ends_the_turn_and_sends_the_next_queued_message_once` (core/tests/model.rs); oracle `live_app_server_failed_turn_does_not_duplicate_error_history` green.
- 2 deprecation routing applied in core/src/app.rs (the patch above, both cells_5 notice tests un-pended and green). Correction of the reviewer: the thread-less
  warning and the deprecation summary stay as the `backend_notices` banner line while NO thread is open (`session.active` is none); with a thread open they are
  that thread's detailed-transcript rows. (The oracle always has a chat; the startup page has none.)
- 3 `Failure::TurnFailed`, `strings.chat.turn_failed` and the `failure_text` arm removed.
- 4 collab `preview` stays by chars: `codex-tui-core` has no `unicode-segmentation` dependency (the layers rule lists its dependencies; the crate is only in the
  workspace manifest), so it is port debt: grapheme cut in collab previews (needs the dependency allowed in core, or a segmentation fn from the host).
  PLAN_URL_TOKENS stays pending, not an rtwk gap: the oracle keeps a URL whole on one row wider than the screen (the terminal wraps it); a cell grid cannot
  hold a 70-char token on a 30-column row, so this is a by-design difference of an alt-screen grid, not something rtwk could add. Reason text unchanged.
