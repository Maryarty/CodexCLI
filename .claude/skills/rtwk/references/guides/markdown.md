<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Markdown documents

<!-- not rtwk: TranscriptView Markdown RenderMarkdown -->

Show a document with `MarkdownView::new(source)` in a
`SelectionArea(SingleChildScrollView(..))`, as twk shows its `Markdown`. The
view is a column of `Text`s and `CodeView`s as tall as its content: the app
scrolls it and selects in it as with any other text, and gives it the width it
has. It builds every block, so a document of a few thousand blocks is what it
is for (see "Very large documents" below for the rest).

```rust
use rtwk::gestures::{SelectionArea, SelectionAreaController};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk_rich::markdown::{MarkdownController, MarkdownView};

let headings = MarkdownController::new();
let scroll = ScrollController::with(0, true);
let selection = SelectionAreaController::new();

let pane = SelectionArea::new(
    SingleChildScrollView::new(
        MarkdownView::new("# Report\n\nReady to review.")
            .controller(headings.clone())
            .tab_width(8),
    )
    .controller(scroll.clone()),
)
.controller(selection.clone());
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
    .controller(headings.clone());
// shown in the same SelectionArea(SingleChildScrollView(..)) as above
```

Mount each revision at the same position. If the application already owns
the complete source, use `.appended(piece)` when the suffix is known.
Use `MarkdownView::blocks(blocks, definitions)` for an existing parsed
document. The view accepts theme, hyperlink and code settings, whether it
comes from a source, blocks or a snapshot. Its tables follow twk: one-line
cells, cut with an ellipsis where they do not fit.

## Migrating an existing pane

A pane made with `.into_viewport()` (`ComposedMarkdownView`, now
`#[deprecated]`) moves to the view above:

* Replace `.into_viewport()` by the `SelectionArea(SingleChildScrollView(..))`
  around the view. The former scroll controller goes on the
  `SingleChildScrollView` and the former selection controller on the
  `SelectionArea`.
* Replace `.surrounding(before, after)` by a `Column` of the neighbours and
  the view in the same scroll view (see "Markdown alongside other widgets").
* `.wrap_table_cells(true)` has no counterpart: the view's cells are one line,
  as twk's.
* The scrollbar's metrics are exact, the view being laid out whole, and a
  selection reaches the whole document, not the blocks that are built.

## Very large documents

The viewport mounts the blocks in view and nothing else, which is what a
document of some 20 000 blocks needs: the one view builds all of them (about
`16 kB` for each block, and every paragraph wraps again when the width
changes). `ComposedMarkdownView`, reached through `into_viewport()` or
`ComposedMarkdownView::snapshot`, stays for that case alone, until the lazy
region of the one view replaces it, and is then removed. It is deprecated: its
selection is a plain `SelectionArea` over the blocks that are built, so what
was scrolled out of its list is not selected or copied, and a selection does
not outlive the block it began in once that is unmounted; its texts are joined
as the area joins them. Do not start new panes with it.

## Selecting and copying

As in twk, a `SelectionArea` around a `MarkdownView` selects and
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
(4.7 ms per token at 2,000 rows). For such a message, until the lazy region replaces it, make that one entry a
`ComposedMarkdownView` (deprecated; a bounded item of the list, 86 µs per
token in the same benchmark, scrolling inside its item, with no selection
across it and its neighbours) or give it a pane of its own with
`ComposedMarkdownView::snapshot`.
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

## Fenced blocks of your own

A fenced block of code is twk's `CodeView` in the language of its info
string. An app that draws a language itself (a diagram, a math block, a
chart of its own) gives the view a builder for it. The builder returns a
widget, or `None` to leave the block as code.

```rust
use rtwk::widgets::Text;
use rtwk::AnyWidget;
use rtwk_rich::markdown::{Fence, FenceBuilders, FenceBuildersScope, MarkdownView};

// make the map once: builders are equal when they are the same map, and a
// map made at every build makes every block again
let builders = FenceBuilders::new().with(&["note", "aside"], |fence: &Fence<'_>| {
    if fence.code.trim().is_empty() {
        return None;
    }
    Some(Text::new(format!("Note: {}", fence.code)).into())
});

// for one view
let view = MarkdownView::new("```note\nBack up first.\n```").fence_builders(builders.clone());
// or for every view below a widget
let app = FenceBuildersScope::new(builders, MarkdownView::new("..."));
```

* The language is the first word of the info string, in any case; the
  builder is given the whole string (`fence.info`) and the code
  (`fence.code`). A fence with no language and an indented block are code.
* The view's builders come before the scope's, which come before
  `CodeView`. `fence.code_block()` is that code block, for a builder that
  puts a note above it.
* The widget is laid out at the width the block has (in a list item and in a
  quote, less). A builder that depends on the width makes its content in a
  `LayoutBuilder`. A built block selects and copies as any other text.
* **A fence still open is code.** In a stream, the builder is asked when
  the closing fence comes, once; the widget is kept from then on. A builder
  that wants the code as it arrives says `builds_open` (the `FenceBuilder`
  trait) and gets `fence.open == true` until the close.
* A document given as text cannot tell whether it is whole, so its unclosed
  last fence counts as open. Show a finished reply from the snapshot of a
  closed stream (`stream.close()`, then `MarkdownView::from_snapshot(stream.snapshot())`)
  to have that fence built.
* `markdown_lines`, the lines of a document as data, builds no widgets and
  uses no builders.

## Markdown alongside other widgets

Put the neighbours and the view in one `Column` inside the scroll view, so
that they scroll and select together; the area joins their texts as it joins
any.

```rust
use rtwk::gestures::SelectionArea;
use rtwk::layout::Column;
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::widgets::Text;
use rtwk_rich::markdown::{MarkdownStream, MarkdownView};

let mut answer = MarkdownStream::new("## Answer\n\n");
let scroll = ScrollController::with(0, true);
answer.add("Here is the result.");
let transcript = SelectionArea::new(
    SingleChildScrollView::new(
        Column::new()
            .child(Text::new("Earlier message"))
            .child(MarkdownView::from_snapshot(answer.snapshot()))
            .child(Text::new("Tool finished")),
    )
    .controller(scroll.clone()),
);
```

Keep the stream and scroll controller across updates. Every neighbour is
built with the document, and one selection covers all of them. For a
conversation of independent messages, see
[Chat transcripts](#chat-transcripts).

## One view

`MarkdownView` is composition, as twk's `Markdown` is: native `Text`s and
`CodeView`s, so that it inherits the theme, takes the pointer and selects
across blocks as any other widgets do. The direct renderer (`RenderMarkdown`)
has been removed. The composed viewport, which mounts only the blocks in view,
is deprecated and stays for documents of some 20 000 blocks until the lazy
region replaces it (see
the design).
