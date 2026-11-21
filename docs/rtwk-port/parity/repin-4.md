# rtwk-repin-4: tui-rtwk on rtwk main 5583c142

Branch `claude/rtwk-repin-4` (rebased on `claude/tui-rtwk` ad43b307c), worktree
`.claude/worktrees/agent-a8fc638d581e1cfd7`. The person's decision (2026-11-21): the
`codex/rtwk-chat-integration` pin (492ab738) is dropped; tui-rtwk builds against rtwk
main and records what main lacks.

## What changed

1. Pin `~/.cache/codexcli/rtwk-pin-5583` (Cargo.toml paths, `RTWK_REF`, the rtwk skill
   regenerated: `generate.py --check` green, "29 files match the pin 5583c142"). The
   `[profile.dev]` lines of build-profiles are kept. README.md and `parity/cells.md` no
   longer name 492ab738 / rtwk-chat-pin.
2. The four chat-integration fields are gone: `block_style`, `list_spacing`,
   `table_style`, `unordered_marker` (`app/theme.rs`, `transcript_pane.rs`). The colours
   stay in main's `MarkdownTheme` (heading, strong, link, marker = ordered numbers light
   blue, quote and quote bar green). The teal unordered bullet colour is lost with the
   field (gap G73 item 4).
3. `src/widgets/markdown.rs` (new): `view(source)` is `MarkdownView::blocks` over the
   parsed blocks with each heading's `#` marker as the first text of its content (also in
   lists and quotes); `themed(view, theme)` applies the theme and
   `LinkDestinations::Auto`. Every Markdown cell goes through them: Agent
   (`transcript_pane.rs`, `cells/citation.rs`), plan, reasoning, delegated,
   transcript prompt, startup prompts panel. Heading markers come from a six-entry table
   (rule R15 forbids `.repeat(` in views). Three unit tests (rendered rows) bite: with the
   marker composition removed all three fail.
4. Startup migration notice (`pages/startup/prompts/panel.rs`): main's inline
   `MarkdownView` is a Column of blocks, so under a bounded Column it flags an overflow
   when clipped (RenderFlex lays inflexible children out with `(0, max_main)`);
   wrapped in `SingleChildScrollView` + `NeverScrollableScrollPhysics` +
   `can_request_focus(false)`, which lays the child out whole and clips it
   (startup_prompts 26 pass / 0 red).
5. G53 closed: the import review checklist is a `ListView` with `shrink_wrap(true)`; the
   `SizedBox(row count)` is gone (`pages/startup/import/mod.rs`, `tests/pages/import.rs`).
6. G55 narrowed: Bedrock secret fields use `obscure_reveal_last(true)` with
   `obscure_reveal_timeout(None)` (tests/pages/signin_wizards.rs). What stays:
   `credential_entries_share_region_and_keep_aws_secrets_hidden` is `pending(G55)` with
   the narrowed reason.
7. G71 closed by the pin: main's `MarkdownView` paragraphs follow `DefaultTextStyle`.
   `ReasoningCell` is a `DefaultTextStyle(dim italic)` over the themed view (the
   `dim_italic(MarkdownTheme)` copy of every slot is deleted); the `probe_g71_...` test
   became `plain_words_are_dim_and_italic_too`.
8. Link destinations: `LinkDestinations::Auto` (web label alone with its OSC 8
   hyperlink, a local target with ` (target)`). The oracle's web suffix is its display for
   a terminal not known to support OSC 8 (`web_links.rs` `WebLinkDisplay::for_terminal`);
   the port has no terminal detection. Under `Suffix` three passing oracle tests
   (`supporting_terminals_render_only_the_styled_label_and_keep_its_target`,
   `links_keep_their_columns_after_styled_fragments`,
   `web_link_labels_keep_link_style_in_wrapped_prose_and_tables`) turn red, under
   `Hidden`/`Auto` the suffix tests stay pending with this reason
   (`markdown_streaming_1` annotates_explicit_web_link_label_and_visible_destination,
   `markdown_streaming_3::url_link_shows_destination`); the false "pin lacks baccefc2"
   reasons are rewritten and `file_citations_preserve_escaped_nested_and_reference_directives`
   passes (its pending line removed, the soft-break line is G63).

## Gap rows for `docs/rtwk-port/rtwk-gaps.md` (not edited: the file is claimed by approvals)

Next free number is G73 (G72 is on `claude/approvals`).

```
| G73 | rtwk theme has no ThemeExtension / Markdown layout options (Flutter `ThemeExtension`, `MarkdownStyleSheet` `blockSpacing` / `tableBorder.horizontalInside` / `listBullet` / element builders): 1 heading+quote source markers and unpadded code, 2 list spacing compact/uniform, 3 table row rules, 4 separate unordered-bullet style. On main 5583c142 `MarkdownTheme` holds colours only; `Glyphs` fix the quote bar `│`, the bullets `•`/`◦` and the table grid (`│`, `─┼─`, one line per row, cells cut with `…`; `ComposedMarkdownView::wrap_table_cells` wraps but is a document scroll view); a code block pads one cell | the chat's Markdown cells (Agent, plan, reasoning, delegated, prompts, startup notice) | headings only: `widgets/markdown.rs` composes the `#` marker per block (`MarkdownView::blocks`); a quote's `> ` repeats on every wrapped row, so it cannot be composed; the rest is pending | 44 `pending(G73)` oracle tests (markdown_streaming_1..5, cells_3, renderer_internals): quote tui `markdown_render.rs:884`, code `:1162`, list spacing `:1084`, bullet `:1131`, table `:112` `:1851`, key/value `markdown_render/table_key_value.rs:98`; page test `tests/pages/cells.rs` a_delegated_message_..._uniform_list_spacing | wanted: Markdown options for the quote prefix, code padding, list spacing, the bullet per depth, table rules / wrapped cells / key-value fallback, or a `ThemeExtension`-style slot in the theme | open (the person, 2026-11-21: repin to main, record the rest) |
```

Rows to change:

- G53: status `CLOSED by rtwk-repin-4`: `ListView::shrink_wrap(true)` (abbafd32) in the import review, `SizedBox(row count)` dropped (`pages/startup/import/mod.rs`).
- G55: keep open, narrowed: "rtwk `TextField::obscure_reveal_last(true)` + `obscure_reveal_timeout(None)` shows the grapheme TYPED last while focused (Bedrock secret fields use it, `tests/pages/signin_wizards.rs` credential_entries_share_region_and_keep_aws_secrets_hidden); the oracle's `FieldDisplay::FocusedSecret` always shows the last grapheme of a focused secret, also on arrival at text already in the field. Wanted: reveal the last grapheme of a focused obscured field without a typing event."
- G71: `CLOSED by the repin (main 5583c142)`: paragraphs follow `DefaultTextStyle`; `ReasoningCell` uses it; probe turned into `plain_words_are_dim_and_italic_too`.

## Oracle counts (`--profile oracle`, `summary.py`)

| | pass | n/a | pending | todo | red |
|---|---|---|---|---|---|
| before (tip 67cf2ad40, old pin) | 393 | 271 | 1864 | 30 | 3 |
| after | 350 | 271 | 1907 | 30 | 3 |

44 tests pass -> `pending(G73)`; 1 pending -> pass
(`file_citations_preserve_escaped_nested_and_reference_directives`). The 3 reds are the
same before and after (`chat_commands_2` x2, `small_dialogs_2::warning_pages_...`);
`guards::shards_are_black_box_and_never_skipped` is red before and after (assert lines in
cells_3, cells_4, chat_live_5; not in the summary table). Shards moved: cells_3 5->4,
markdown_streaming_1 27->21, _2 29->12, _3 12->11, _4 20->8, _5 22->18,
renderer_internals 12->10.

## Checks

- `acargo fmt -- --check`: clean. `clippy --workspace --all-targets -- -D warnings`: exit 0.
- Whole nextest (`--workspace`): 1392 passed, 16 skipped. An earlier run failed
  `startup::gate::the_thread_starts_after_the_startup_screens` (1 of 4 runs of that test,
  `left: 0, right: 1` at `tests/pages/startup.rs:147`: the count of `StartThread` read
  right after the chat row; startup-rest's test, no Markdown involved, a race of the fake).
- `shards.py --check`: 55 shards, 2560 tests, 0 files stale. `generate.py --check`: green.
- Binary (`check.py --bin rtwk --bin oracle --only markdown,reasoning,markdown-list-layout`,
  pty 120x40): 6/6 PASS.

## Binary rows: `turn-1` of `markdown` (the oracle shows scrollback, rtwk its transcript, so rows are paired by content)

```
oracle                                    rtwk
## Code                                   ## Code
fn main() {                                fn main() {            (code padded one cell)
    • nested item                              ◦ nested item
 name     kind      count                  name  │ kind   │ count
━━━━━━━  ━━━━━━━━  ━━━━━━━                 ──────┼────────┼──────────────
 alpha    letter        1                  alpha │ letter │     1
───────  ────────  ───────                 (no rule between body rows)
 beta     letter       22                  beta  │ letter │    22
> a quote that runs long enough ...        │ a quote that runs long enough ...
```


`markdown-list-layout` adds: the oracle's pty terminal is unknown to it, so it writes
`Один сценарий (https://example.com/compare) для двух размеров терминала.`; rtwk draws
`Один сценарий для двух размеров терминала.` (item 8). `reasoning`: the text rows are
equal (the dim italic of the plain words now comes from `DefaultTextStyle`); only the dim
colour differs (the chat theme's `dim`, indexed 245, against the oracle's plain DIM attr:
older than this task).

## For the lander / orchestrator

- `docs/rtwk-port/rtwk-gaps.md`: G73 new, G53 closed, G55 narrowed, G71 closed (above).
- tm area: `rtwk-gaps.md`, `tests/pages/signin_wizards.rs` and `tests/oracle/startup_prompts.rs`
  were claimed by approvals / startup-rest during the run, so they were not edited here
  (the earlier commit changed `signin_wizards.rs` before the claim).
- Not built on: `ListView::selection_text`, `ItemTexts`, `ListView::item_text` (ADR-029).
- Visible changes against the old pin, all under G73: quotes read `│ …` not `> …`, code
  blocks are padded one cell, nested bullets are `◦`, a loose list has blank rows between
  every item, tables are `│` grids, the unordered bullet is no longer teal.
