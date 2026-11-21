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
- Link destinations wait for rtwk baccefc2 (MarkdownView::link_destinations); the pin is 492ab738.
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
Rows for the lander (item 11, src/app/theme.rs is startup-finish's area): `transcript_markdown` needs the ordered-list marker
colour LightBlue (10 markdown_streaming tests name it) and the blockquote bar Green (5 tests).
Gap rows: G71 (MarkdownView paragraphs take no inherited text style). Renumbered from G65 (composer took G65-G70 on the tip);
the commit message of fb9220a1 still says G65.
Pin: link destinations need rtwk baccefc2 (`MarkdownView::link_destinations`); the pin (492ab738) is older.
Left (in order): 7 `:codex-file-citation` rewrite (oracle markdown_render/inline_directives.rs: an offset-preserving scan of
the source outside code, HTML, links and reference definitions; rtwk way: parse with rtwk-rich `parse_markdown` and turn the
directive inside `Inline::Text` into a link node, or a source rewrite with a literal-range scan; then close or reword G60);
8 view-image "+ Show details" (stateful ViewedImageCell, Pressable, the disclosure indent of the patch cell; the image bytes need a
services effect: debt task); 9 web search: the oracle commits `• Browsing the web` when another item flushes the active cell before
`on_web_search_end` (trace the mock scenario's event order); the exec cell off `ToolRow` (~14 cells_1 tests: reasons say core ignores
commandExecution items; check that they still do), hook-prompt, collab-agent, sub-agent, sleep, image-generation items, warning/error,
deprecation, plan-update and session-info cells (core `Item` variants plus one widget per file); the 330 pending tests carry
reasons that are the probe's first difference: trace them cell kind by cell kind; mock-model check.py run and the pack's rows.
