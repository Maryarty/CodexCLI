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
