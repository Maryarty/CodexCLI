# rtwk-repin-6: tui-rtwk on rtwk main 5001d2cd9

Branch `codex/f-rtwk` (from `claude/tui-rtwk` 154ed15a6), worktree
`.claude/worktrees/f-rtwk`. The person's decision (2026-11-28): "much is done in rtwk
already, the repin is due". The pin is `~/.cache/codexcli/rtwk-pin-5001`, rtwk main
`5001d2cd9`, 905 commits after `c953f062`. The old `rtwk-pin-c953` stays: live worktrees
build on it. No new crate is needed by the workspace (`Cargo.lock` unchanged: rtwk's new
crates `rtwk-math` and the mermaid ones are optional features of `rtwk-rich`, off by default).

## What changed

1. Pin: `Cargo.toml` paths, `docs/rtwk-port/RTWK_REF` (`5001d2cd90ce0caa4ba5b9abf05689ffed3333bf`),
   `README.md` (line 22), `research/briefs/transcript.md`, the rtwk skill regenerated
   (`generate.py --check` green, 41 files). `tests/rules/pin.rs` passes.
2. Compile on the new API: `MdBlock` and `MdInline` are `#[non_exhaustive]` now (md-alerts:
   `MdBlock::Alert`; md-footnotes: `MdInline::FootnoteReference`; md-math: `MdInline::Math`
   behind feature `math`). Six matches had no wildcard arm:
   - `widgets/markdown/inlines.rs` `block` and `plain`, `pages/session/transcript/markdown_text.rs`
     `block` and `push`: a wildcard arm (kept as is / not modelled / no text); the app turns
     none of those extensions on, so no node reaches them. Committed.
   - `pages/session/cells/citation.rs` `block` (:521) and `inline` (:618): the same two arms
     (`other => other.clone()`). **NOT committed: the file is in the area of
     `f-cells-exec-mcp` (a live claim), so this is a Need.** The patch, to apply once the
     claim is free (or by that task, which owns the file):
     `            // a block of an extension the app does not turn on (alerts)` /
     `            other => other.clone(),` as the last arm of `fn block`, and
     `            // footnote references and math belong to extensions the app does not turn on` /
     `            other => other.clone(),` as the last arm of `fn inline`.
   Nothing else breaks: with those two arms in a scratch checkout (not the branch),
   `check --all-targets` and `clippy --workspace --all-targets -D warnings` are clean.
   No workaround around rtwk was written (R15).
3. Tests red from the new rtwk: none found in the scoped runs (scratch checkout with the
   citation arms): `--test rules` 184 of 184; `--test screens --test pages` filtered to
   markdown, transcript, cells, composer, select, wrap, rtwk_g: 693 of 693; the lib tests of
   the same words 59 of 59; oracle shards `markdown_streaming_1..5`, `composer_1..4`,
   `composer_popups`, `textarea`, `pager`, `transcript_view_1..5`, `cells_1..5`, `guards`:
   713 pass / 105 n/a / 315 pending / 0 red. The whole suite is the batch waiter's.
4. Docs: this file. `rtwk-gaps.md` is left to `rtwk-gap-probe` (its row marks follow the probe).

## What the pin brings that touches the port

Read from the pin's source (`crates/rtwk-rich/src/markdown/{nodes,options,view,math}.rs`,
`git log c953f062..HEAD` per crate); none of it is wired in the port yet.

- `rtwk-rich` feature `math` (off by default): `MdExtensions::MATH` (`$..$`, `$$..$$`) and
  `MdExtensions::MATH_BRACKETS` (`\(..\)`, `\[..\]`), `MdInline::Math`, `rtwk_math::render`.
- `rtwk-rich` feature `mermaid` (off by default): `MarkdownView::mermaid(MermaidOptions)` draws a
  mermaid fence (33 kinds), `Fallback::Diagnostic`, links and colours options.
- `MdExtensions::ALERTS`, `MdExtensions::FOOTNOTES` (+ `footnote_jump`, `Span::copy_as`).
- `MarkdownView`: `heading_marks(bool)`, `table_overflow`, `table_stretch`, `code_continuation`,
  `code_language_label`, `lazy_from` (lazy blocks; "a selection that leaves the blocks built
  copies the text of those only"), `MarkdownController` jumps.
- Selection: `SelectionDecoration` (styled ranges over a selectable's text, drawn under the
  selection), selection edges snap to grapheme clusters, ListView `lazy_window`.
- Unchanged since `c953f062`: `rtwk-editing` (TextArea, TextField, EditingKeys: no commit),
  `rtwk-render/src/objects/text.rs` (`TextWrap::{Word, Grapheme, None}`, one memory commit),
  the Markdown options for soft break, list indent, spacing, table rules (none exist).

## Candidates for `rtwk-gap-probe` (rows of `rtwk-gaps.md`)

| row | on the pin | action for the probe |
|---|---|---|
| G61 math | door exists: feature `math` + `MdExtensions::MATH`/`MATH_BRACKETS` + `rtwk_math::render` | enable the feature, wire the chat markdown; the `$..$`, `\[..\]`, `aligned` tests of `markdown_streaming_1/2/5` |
| G62 mermaid | feature `mermaid` + `MarkdownView::mermaid` draws the fence | enable, wire; `markdown_streaming_2/5` MERMAID tests (holdback while streaming is the port's) |
| G73 | `heading_marks(true)` (item 1, heading `#`), `table_stretch` exist; no list spacing, table row rules, bullet style | partly: probe item 1 only |
| G-new-sel-caret-range, G-new-1 (matched range style) | `SelectionDecoration` styles ranges of a selectable's text; the editing widgets still paint one style | probe the transcript half only |
| G-new-sel-lines, G-new-markdown-text, G-new-reader-reflow-anchor | `lazy_from` / `lazy_window` / jumps: text of blocks not built is still "until read from the document" | probe: unclear, read `lazy-l3-rich` design page first |
| G63 soft break, G64 list indent | no option on the pin | stays open |
| G65 TextArea, G66, G67, G78 | `rtwk-editing` unchanged | stays open |
| G48, G49, G72, G75, G76 wrap | `TextWrap` unchanged | stays open |
| G60, G71 | no inline syntax extension, no inherited text style | stays open (not looked at beyond the option list) |

The probe run (`KIT_PROBE_PENDING=1`, the shards above, scratch checkout) lists no stale
test that comes from the pin: `stale-real` has three (`cells_4::code_mode_tool_call_...`,
`cells_5::agent_markdown_cell_narrow_width...`, `cells_5::render_clears_area...`), all
chat-live reasons that pass vacuously, and the `{weak-body}` G61/G62 markers pass only
because the body asserts the source text. So nothing un-pends by the pin alone: G61 and
G62 need the port to turn the features on.

## Oracle counts (scoped, `--profile oracle`)

| | pass | n/a | pending | red |
|---|---|---|---|---|
| shards above, after | 713 | 105 | 315 | 0 |

## State of the branch

`codex/f-rtwk` tip builds only with the two `citation.rs` arms (Need above): the lib
fails with E0004 at `cells/citation.rs:521` and `:618` until then.
