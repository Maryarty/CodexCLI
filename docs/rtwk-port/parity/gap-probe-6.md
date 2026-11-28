# rtwk-gap-probe: the gap rows on rtwk 5001d2cd9

Branch `codex/f-rtwk`, worktree `.claude/worktrees/f-rtwk`, pin `~/.cache/codexcli/rtwk-pin-5001`
(rtwk main `5001d2cd9`) with the `kit::delta` pipeline of `kit-delta`.

## Method

1. Probe: the oracle test binary with `KIT_PROBE_PENDING=1`, profile `oracle`, `--no-fail-fast`, over
   every shard (the marker lets the body run). The non-probe baseline of the same tree gives the counts
   (443 pending tests).
2. `summary.py --list stale-real` (a pending test that passes with a body that asserts something
   present), `--list stale`, `--list delta-stale`, and `delta.py candidates` (a pending test whose
   probe failure is only a comparison of rows or cell styles).
3. For each door the pin brings (`repin-6.md`, "What the pin brings"): the port was wired in a scratch
   checkout, the probe run again, the result read per test. The scratch patch is kept in
   `gap-probe-6/math-mermaid.patch` (not applied to the branch: see Needs).

## Result in one table

| | before | after |
|---|---|---|
| stale-real, free shards | 1 (`chat_status_1::ambient_pet_screen_bottom_anchor_uses_terminal_bottom`, by the pin) | 0 |
| stale-real, claimed shards | 3 (`cells_4` x1, `cells_5` x2: vacuous chat-live markers) | 3 (Need for `f-cells-exec-mcp`) |
| pending, whole suite | 443 | 441 |
| pending turned green here | | 2 (one by the pin: `chat_status_1`; one by a new door: `transcript_view_4`) |
| new delta candidates (free shards) | | 0 on the pin; 4 with math wired (not on the branch) |

The pin alone closes one test: the ambient pet at the terminal bottom (80x24) now draws at the
oracle's (71, 18, 9, 5); the marker was stale. Its row G4 keeps the short inline case.

## Per gap row

Counts are pending tests whose reason names the row (non-probe baseline).

| row | pending | lifted | remain | on the pin, why |
|---|---|---|---|---|
| G-new-sel-caret-range | 1 | 1 (`transcript_view_4::select_mode_renders_cursor_and_scannable_footer_hints`) | 0 | closed: `SelectionAreaController::set_decorations` + `SelectionDecoration::new(target, range, pen).item(row)` (a pen merged under the selection). Select mode draws the caret grapheme as a decoration (bold, underlined, reversed over a mark), through the area for every cell kind, Markdown included. Found on the way: in the transcript view alone (no composer) the area's own Esc clear key took the selection and left the mark; the key is the mode's while it is on. |
| G61 math | 27 | 0 on the branch | 27 | door exists: `rtwk-rich` feature `math`, `MdExtensions::MATH` / `MATH_BRACKETS`, `MarkdownStream::with_extensions`, `MarkdownView::extensions`. Wired in scratch: 1 new body pass (`markdown_streaming_5::unicode_math_rejected_display_keeps_its_closer_and_following_text`), 4 new delta candidates (`markdown_streaming_2::unicode_math_{accents_and_symbols,inline_narrow,inline,named_delimiters}_snapshot`), the 5 `{weak-body}` markers keep passing on the source text (they stay), the other 17 stay red: rtwk's renderer is a narrower subset with other typography than the oracle's, and it holds nothing back while streaming (the port's holdback is not the oracle's for `aligned`). Orch27: the oracle shows math (measured), so math is to be turned on: Need (Cargo.toml). |
| G62 mermaid | 5 | 0 on the branch | 5 | door exists: feature `mermaid`, `MarkdownView::mermaid(MermaidOptions)`. Wired in scratch: 1 new pass (`transcript_view_2::rendered_mermaid_copies_source`), `markdown_streaming_2::mermaid_fences_use_native_renderer_for_every_family` passes (it passed before, `{absence-only}`), 3 stay red (the drawing differs from the oracle's per family). Turned on unconditionally it breaks 3 passing tests (`markdown_streaming_2::disabled_mermaid_keeps_unsupported_source_without_notice`, `disabled_renderers_preserve_source_independently`, `markdown_streaming_5::mermaid_holdback_tracks_nested_blocks_after_normalized_markdown`): the port must honour `Rendering.mermaid` / `Rendering.math` (core/src/app/facts.rs), and `MdInline::Math` needs a text model for copy (`markdown_text.rs`, `widgets/markdown/inlines.rs` take it by a wildcard arm now). |
| footnotes, alerts | 0 | | | the oracle shows neither (measured): `MdExtensions::FOOTNOTES` and `ALERTS` stay off. |
| G73 | 84 | 0 | 84 | `heading_marks(true)` exists; the port already composes the heading `#`. No option for the quote prefix, code padding, list spacing, bullet per depth or table rules. |
| G63, G64 | 2, 5 | 0 | 7 | no soft-break option, no list-indent option on `MarkdownView` (unchanged since `c953f062`). |
| G65, G66, G67, G78 | 27, 0, 0, 1 | 0 | 28 | `rtwk-editing` has no commit since `c953f062`. |
| G48, G49, G72, G75, G76 | 5, 8, 8, 1, 0 | 0 | 22 | `rtwk-render/src/objects/text.rs`: `TextWrap::{Word, Grapheme, None}`, unchanged. |
| G60, G71 | | 0 | | no inline syntax extension, no inherited text style on `MarkdownView` (option list read, unchanged). |
| G-new-1 | 4 | 0 | 4 | `SelectionDecoration` styles ranges of selectable text; `TextArea` paints its text with one style: the editing widgets did not change. |
| G-new-sel-reveal | 8 | 0 | 8 | still no per-line reveal (`Scrollable.ensureVisible` of one line of a tall item). |
| G-new-sel-point | 3 | 0 | 3 | unchanged. |
| G-new-usage-2 | 6 | 0 | 6 | a `ListView` builder is not told the items it shows (`lazy_window` takes a window in, it does not report one). |
| G-new-sel-edge-zone, G-new-sel-wheel-pause | 2, 1 | 0 | 3 | `gestures/selection.rs` autoscroll and wheel timers unchanged. |
| G-new-sel-lines, G-new-markdown-text, G-new-reader-reflow-anchor | 1, 0, 1 | 0 | | the probe still fails them; `lazy_from` / `lazy_window` were not wired (the text of a block that is not built is still read from the document). |
| G4 | | 0 | 1 inline case | the pet test at the terminal bottom is lifted by the pin; the short inline viewport case stays. |

## Delta (item 3)

`delta.py candidates` on the probe of this tree: 128, the same 128 as before the select change; the
10 that are small are the ones `kit-delta` converted (`docs/rtwk-port/visual-deltas.md`); the rest are
rows that differ by words or layout, not a small delta. With the math wiring of the scratch patch 4 more
appear (the `unicode_math_*_snapshot` tests above). Nothing else on the pin.

## Needs

- `codex-rs/tui-rtwk/Cargo.toml` (+ `Cargo.lock`): `rtwk-rich = { workspace = true, features = ["math", "mermaid"] }`.
  Held by `s10-runtime-metrics-source`; `task_update` refused the widening. Then
  `gap-probe-6/math-mermaid.patch` is the start (`citation.rs` stream with `MdExtensions`,
  `widgets/markdown.rs` `.extensions(..)` / `.mermaid(..)`); it must read `Rendering.math` and
  `Rendering.mermaid` first (3 passing tests break otherwise) and add the `MdInline::Math` text model.
- `docs/rtwk-port/rtwk-gaps.md`: held by `f-footer-status`; `task_update` refused. Marks proposed for the
  landing (or for the next free claim):
  - G-new-sel-caret-range: `closed (rtwk-gap-probe, pin 5001d2cd9): SelectionDecoration; select mode draws its caret as a decoration, Markdown cells included; transcript_view_4::select_mode_renders_cursor_and_scannable_footer_hints green`.
  - G4: append `the terminal-bottom case closed on pin 5001d2cd9 (chat_status_1::ambient_pet_screen_bottom_anchor_uses_terminal_bottom green); the short inline case stays`.
  - G61: replace the tail with `pin 5001d2cd9: door exists (feature math, MdExtensions::MATH/MATH_BRACKETS); not wired (Cargo.toml claim); scratch: 1 pass + 4 delta candidates of 27, the renderer differs (subset, typography, no streaming holdback)`.
  - G62: `pin 5001d2cd9: door exists (feature mermaid, MarkdownView::mermaid); not wired; scratch: 1 new pass of 5, the drawing differs per family; the port must honour Rendering.mermaid`.
  - G73: `pin 5001d2cd9: heading_marks only (already composed); the rest has no option`.
  - G63, G64, G65, G66, G67, G78, G48, G49, G72, G75, G76, G60, G71, G-new-1, G-new-sel-reveal, G-new-sel-point, G-new-usage-2, G-new-sel-edge-zone, G-new-sel-wheel-pause: `unchanged on pin 5001d2cd9 (rtwk-gap-probe)`.
- `cells_4::code_mode_tool_call_preserves_failure_details`, `cells_5::agent_markdown_cell_narrow_width_shows_prefix_only`,
  `cells_5::render_clears_area_when_cell_content_shrinks`: stale-real in shards of `f-cells-exec-mcp`; their
  markers describe a state that passes vacuously (the owned transcript draws the port's cell); strengthen or
  un-pend with the claim's owner.
