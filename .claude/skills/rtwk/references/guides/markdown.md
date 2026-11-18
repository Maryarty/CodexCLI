<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Markdown documents

Use `MarkdownView::new(source).into_viewport()` for a document pane.
Give the pane bounded width and height, for example by placing it in
an `Expanded` child of a bounded column. The viewport owns scrolling
and selection. It mounts visible list items on demand and preserves
the visible source fragment when the width changes.

```rust
use rtwk::gestures::SelectionAreaController;
use rtwk::scroll::ScrollController;
use rtwk_rich::markdown::{MarkdownController, MarkdownView};

let headings = MarkdownController::new();
let scroll = ScrollController::with(0, true);
let selection = SelectionAreaController::new();

let view = MarkdownView::new("# Report\n\nReady to review.")
    .controller(headings.clone())
    .tab_width(8)
    .into_viewport()
    .scroll_controller(scroll.clone())
    .selection_controller(selection.clone());
```

Keep the controllers across builds. For a stream, keep a
`MarkdownStream` in application state and publish a snapshot after
appending each piece. The snapshot shares completed blocks with earlier
revisions.

```rust
use rtwk_rich::markdown::{MarkdownStream, MarkdownView};

let mut stream = MarkdownStream::new("# Report\n\n");
stream.add("Ready to review.");
let view = MarkdownView::from_snapshot(stream.snapshot())
    .controller(headings.clone())
    .scroll_controller(scroll.clone())
    .selection_controller(selection.clone());
```

Mount each revision at the same position. If the application already owns
the complete source, use `.appended(piece)` when the suffix is known.
Use `MarkdownView::blocks(blocks, definitions).into_viewport()` for an
existing parsed document. The viewport accepts theme, hyperlink and code
settings, whether it comes from a source, blocks or a snapshot.

## Migrating an existing pane

Replace the enclosing vertical scroll view with the composed viewport
and attach the former scroll controller directly. Use the viewport's
selection controller for copy and selection across unmounted blocks.
The default table view uses single-line cells with ellipsis.
Call `.wrap_table_cells(true)` on the viewport for multiline cells.
Scrollbar metrics may be estimates while offscreen items await layout.
Use `.surrounding(before, after)` to place neighboring widgets in the
same virtualized scroll view.

Keep the inline `MarkdownView` in an existing outer scroll container
when Markdown is one item in a larger scrolling transcript or exact
content-height metrics are part of the enclosing view's contract. The
inline view is selectable under a `SelectionArea` (see
[Selecting and copying](#selecting-and-copying)); use the viewport when
one document is too long to lay out whole, or its selection must reach
blocks that are not mounted.

## Selecting and copying

As in twk, a `SelectionArea` around an inline `MarkdownView` selects and
copies the text of its blocks: paragraphs, headings, lines of code and
the cells of tables. Bullets, numbers, the boxes of tasks, the bars of
quotes, rules and the padding of tables and of code are not text; they are
not selected and not copied. The area around the view, or around many
of them and other widgets, joins what it copies as it joins any texts: a
line break, and a blank line where a row lies between two.

```rust
use rtwk::gestures::{SelectionArea, SelectionAreaController};
use rtwk::scroll::SingleChildScrollView;
use rtwk_rich::markdown::MarkdownView;

let selection = SelectionAreaController::new();
let transcript = SelectionArea::new(SingleChildScrollView::new(
    MarkdownView::new("# Answer\n\n- one\n- two\n\nDone."),
))
.controller(selection.clone());
// a drag from "Answer" to "Done" copies
// "Answer\n\none\ntwo\n\nDone."
```

A wrapped paragraph is one line of text: no line break is copied where it
wraps. A selection of a streaming answer stays while text is added after
what it holds (a token, a block); an edit of text before its end clears it,
as a `Text` clears the selection of text that changed. The first press on a
document reads the text of every block once (about 11 ms for 10 000
rows in a release build on a loaded machine; the figures and their load
are in `research/markdown-native-comparison/inline-selection.md`);
dragging and appending do not read it again.

## Chat transcripts

A chat is a prompt, an answer, a tool block, again and again, and the answer
streams in. It is built from parts that stay in the framework, in one of two
shapes, each with an example that runs and a test.

**In the terminal's own history** (the inline mode, as twk's `chat.dart`).
What was sent and each block of an answer, once complete, go to the history
above the prompt through `Static`, where the terminal scrolls and selects
them like anything printed. The answer still coming in shows above the prompt
in an inline `MarkdownView` inside a `SingleChildScrollView` that sticks to
its end: on a short terminal the answer gives up its start, not the prompt,
and the region gives back the rows a block took as it settles into the
history. Esc stops an answer by dropping it (its stream and the timer that
feeds it), Ctrl+C quits and leaves the chat in the history.

```rust
use rtwk_rich::markdown::MarkdownStream;

let mut answer = MarkdownStream::default();
answer.add("# Plan\n\nRead the **config** first.\n\nThen");
let settled = answer.take_settled(); // the heading and the paragraph: to `Static`
let live = answer.live().to_vec();   // what may still change: the live view
let definitions = answer.definitions().clone();
// show `MarkdownView::blocks(live, definitions)` above the prompt, and
// `Static::new(history, |_, block| MarkdownView::blocks(vec![block.clone()], ..))`
assert_eq!(settled.len(), 2);
assert_eq!(live.len(), 1);
```

`cargo run -p rtwk-rich --example chat` runs it; its test
(`tests/example_chat.rs`) is twk's `chat_test.dart` ported. The blocks sent to
the history have their references resolved by the definitions of the blocks
settled so far (`MarkdownStream::take_settled`). Like twk's example it runs
inline with `RunOptions { clear_inline_on_exit: true, .. }`, so the empty
prompt is erased on exit and only the chat stays in the history.

**In the full screen.** A `ListView::builder` of the entries (a prompt `Text`,
an inline `MarkdownView::blocks(stream.blocks(), stream.definitions())` for
each answer, a `CodeView` for a tool block), keyed by a stable entry id (with `ListView::find_index`), with
`stick_to_bottom`, inside one `SelectionArea`. A token changes one entry. One drag selects
from a prompt through an answer into a tool block, and copies the three in
order.

`cargo run -p rtwk-rich --example chat_fullscreen` runs it, and
`tests/example_chat_fullscreen.rs` has the drag.

What the list costs, measured on 1,000 turns of this shape (the
transcript-composition benchmark, release build, a shared machine): appending
a token to the newest answer takes about 24 µs at 10, 100 and 1,000 turns,
and mounting 1,000 turns 0.4 ms. One shape is expensive: a single message
with a very large live table lays all its rows out again for every token
(4.7 ms per token at 2,000 rows). For such a message make that one entry a
`ComposedMarkdownView` (a bounded item of the list, 86 µs per token in the
same benchmark, scrolling inside its item, with no selection across it and its
neighbours) or give it a pane of its own with `MarkdownView::from_snapshot`.
There is no transcript widget: `TranscriptView`, shipped in v0.1.1 to v0.1.3,
was removed because in a chat of messages it lost to this composition at
every size (a token on the newest answer: 24 µs against 67 µs, 329 µs and
4.64 ms at 10, 100 and 1,000 turns), and the one thing it did better, a
message with a very large live table, is what `ComposedMarkdownView` is for.
The measurements and
the decision.

Limits, as in twk. A selection is of what is built: an entry that scrolls out
of the list leaves it. A drag whose first entry scrolls out of the list finds
no anchor when it goes on and clears the selection (twk's `_selectTo`), while
a drag that stays within the entries in view selects across them as above
(`an_entry_scrolled_out_of_the_list_leaves_the_selection_as_twks_select_to_has_it`).

## Markdown alongside other widgets

Use `surrounding` when one streaming Markdown document and adjacent
widgets should scroll together. The viewport lazily mounts Markdown
rows and treats each adjacent widget as one list item.

```rust
use rtwk::scroll::ScrollController;
use rtwk::widgets::Text;
use rtwk_rich::markdown::{MarkdownStream, MarkdownView};

let mut answer = MarkdownStream::new("## Answer\n\n");
let scroll = ScrollController::with(0, true);
answer.add("Here is the result.");
let transcript = MarkdownView::from_snapshot(answer.snapshot())
    .surrounding(
        Some(Text::new("Earlier message").into()),
        Some(Text::new("Tool finished").into()),
    )
    .scroll_controller(scroll.clone());
```

Keep the stream and scroll controller across updates. This API covers
one Markdown document with an optional item on each side. Markdown
selection covers that document. For a conversation of independent
messages, see [Chat transcripts](#chat-transcripts).

## Renderer decision

Composition is the primary path for new bounded document panes. Its
native widgets supply theme inheritance, pointer interaction and
cross-block selection. On matched workloads it also improves large
table appends, while the direct renderer uses less RSS in the measured workloads. See the measurements and parity matrix.

The direct renderer remains a public compatibility path during migration.
Moving it behind an internal interface would leave callers that need
inline height and parent-owned scrolling without an equivalent widget.
Removing it would also discard a measured fast path for streaming
without a replacement for those callers. Both options can be revisited
after an inline composition contract is demonstrated and benchmarked.
Viewport selection is explicit so existing struct literals and outer
scroll layouts retain their semantics.
