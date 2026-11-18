<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Editing

[All widgets](../widgets.md)

## TextField

A line of text to edit: Tab to it and type; Enter submits it, Ctrl+Z undoes.

API:

- `rtwk::editing::TextField` (struct, `crates/rtwk-editing/src/text_field.rs:160`) A field of a line of text the user edits, which its `controller` holds (twk's `TextField`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `text_align: TextAlign`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`
- `rtwk::editing::TextEditingController` (struct, `crates/rtwk-editing/src/text_editing.rs:418`) The `TextEditingValue` a field edits, which notifies its listeners of each change.

### Code

The gallery's Code tab at the initial options (route `#/editing/textfield`, `crates/rtwk-gallery/src/pages/mod.rs:323`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let sender = self.sender.as_ref().expect("field initialized").clone();
let mut field = TextField::new()
    .controller(self.controller.clone())
    .placeholder("type here")
    .obscure_text(false)
    .cursor_shape(CursorShape::Bar)
    .text_align(TextAlign::Left)
    .read_only(false)
    .enabled(true)
    .on_submitted(cx.map(|text: Rc<str>| Message::Submitted(text.to_string())))
    .on_editing_complete(cx.send(Message::Heard("editing complete".into())))
    .on_paste(move |text| {
        let _ = sender.send(Message::Heard(format!(
            "a paste of {} characters",
            character_count(text),
        )));
        false
    });
field.max_length = None;
field
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| obscure | on, off | false |
| maxLength | none, 8 | none |
| cursor | bar, block, underline, terminal | bar |
| blink | on, off | false |
| align | left, center, right, start, end | left |
| readOnly | on, off | false |
| enabled | on, off | true |
| paste | goes in, taken | goes in |

### Methods

#### `TextField`: constructors and associated functions

- `pub fn new() -> TextField` - A field of a controller of its own, empty.

#### `TextField`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<TextEditingController>) -> Self` - Edits the supplied shared controller.
- `pub fn placeholder(mut self, placeholder: impl Into<Str>) -> TextField` - Showing `placeholder` while empty.
- `pub fn semantics_label(mut self, label: impl Into<Str>) -> TextField` - Called `label` by a screen reader.
- `pub fn obscuring_character(mut self, c: impl Into<Str>) -> TextField` - Each character shown as `c` with `obscure_text`.
- `pub fn on_paste(mut self, f: impl Fn(&str) -> bool + 'static) -> TextField` - Asking `f` first with what is pasted.

#### `TextEditingController`: constructors and associated functions

- `pub fn new(text: &str) -> TextEditingController` - `text` with the caret at its end.
- `pub fn from_value(value: TextEditingValue) -> TextEditingController` - `value`, kept within its text.

#### `TextEditingController`: methods

- `pub fn listenable(&self) -> Rc<impl Listenable + 'static>` - The same notification source for every clone of this controller.
- `pub fn value(&self) -> TextEditingValue` - The value: the text, the selection, the composing (a clone: the text is shared, not copied).
- `pub fn with_value_ref<R>(&self, f: impl FnOnce(&TextEditingValue) -> R) -> R` - Reads the value without cloning it.
- `pub fn buffer(&self) -> TextBuffer` - The text in pieces (twk's `textBufferOf(controller.value)`).
- `pub fn set_value(&self, new_value: TextEditingValue)` - Sets the value, kept within its text; a change of the text is a step to undo.
- `pub fn can_undo(&self) -> bool` - Whether there is a step to take back.
- `pub fn can_redo(&self) -> bool` - Whether there is a step undone to make again.
- `pub fn undo(&self)` - Takes the last step back: its text as it was, and the selection as it was before it.
- `pub fn redo(&self)` - Makes the last step undone again, the selection as it was after it.
- `pub fn text(&self) -> Rc<str>` - The text, put together from its pieces the first time it is asked for after a change.
- `pub fn set_text(&self, text: &str)` - Sets the text, the caret at its end, the composing ended.
- `pub fn selection(&self) -> TextSelection` - What is selected, the caret at its `extent_offset`.
- `pub fn set_selection(&self, selection: TextSelection)` - Sets what is selected.
- `pub fn composing(&self) -> TextRange` - The part of the text being composed; `TextRange::EMPTY` for none.
- `pub fn set_composing(&self, composing: TextRange)` - Sets the part being composed.
- `pub fn clear(&self)` - Empties the text.
- `pub fn reset(&self, text: &str)` - Puts `text` in, the caret at its end and the composing ended, and starts the history afresh: the text is no step to undo, and the steps before it, those undone too, are gone.
- `pub fn replace(&self, start: usize, end: usize, text: &str)` - Puts `text` in place of the part from `start` to `end`, the caret after it.
- `pub fn insert(&self, text: &str)` - Puts `text` in place of the selection, the caret after it.
- `pub fn move_caret(&self, offset: usize, extend: bool)` - Moves the caret to `offset`: the selection ends, or with `extend` it grows or shrinks from its base to there.
- `pub fn delete_to(&self, offset: usize)` - Deletes what is selected, if anything; else the text between the caret and `offset`.
- `pub fn select_all(&self)` - Selects the whole text, the caret at its end.
- `pub fn notifier(&self) -> &ChangeNotifier` - The notifier the listeners are on.
- `pub fn notify_listeners(&self)` - Tells the listeners now (twk's `notifyListeners`).
- `pub fn dispose(&self)` - Releases the shared listeners for every clone (twk's `dispose`).

### Also used in the code

- `rtwk_core::presenter::CursorShape` (enum, `crates/rtwk-core/src/presenter/mod.rs:121`) The cursor's shape (`DECSCUSR`).
  - variants: `TerminalDefault`, `BlinkingBlock`, `Block`, `BlinkingUnderline`, `Underline`, `BlinkingBar`, `Bar`
- `rtwk::text::TextAlign` (enum, `crates/rtwk-unicode/src/fit.rs:11`) Where a paragraph puts each line across its width.
  - variants: `Left`, `Center`, `Right`, `Justify`, `Start`, `End`

## InputDecoration

What is drawn around a field: a label, an icon, a prefix and a suffix, a helper, an error and a counter, in a box, over a line or bare, filled.

API:

- `rtwk::editing::InputDecoration` (struct, `crates/rtwk-editing/src/input_decoration.rs:142`) What an `InputDecorator` draws around a field: an icon before it, a label naming it, what goes before and after its text, a border around or under it, a fill and a row under it of a helper or an error text and a counter.
  - public fields: `icon: Option<AnyWidget>`, `label_text: Option<Str>`, `helper_text: Option<Str>`, `error_text: Option<Str>`, `counter_text: Option<Str>`, `counter: Option<AnyWidget>`, `prefix_icon: Option<AnyWidget>`, `prefix_text: Option<Str>`, `suffix_icon: Option<AnyWidget>`, `suffix_text: Option<Str>`, `border: InputBorder`, `filled: bool`, `fill_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/editing/inputdecoration`, `crates/rtwk-gallery/src/pages/editing.rs:54`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// The three TextEditingController handles and the theme selection live in state.
let decoration = InputDecoration::new().border(OutlineInputBorder::new()).filled(false);
let themes = Select::new(
    [
        Choice::new("dark", "Dark"),
        Choice::new("light", "Light"),
        Choice::new("nord", "Nord"),
    ],
    Some(theme),
)
.decoration(decoration.clone().icon(Text::new("◐"))
    .label_text("Theme").helper_text("Enter opens it"))
.on_changed(on_theme);
let site = TextField::new()
    .controller(site)
    .placeholder("twk")
    .enabled(true)
    .decoration(decoration.clone().label_text("Site")
        .prefix_text("https://").suffix_text(".dev").helper_text("where it lives"));
let search = TextField::new()
    .controller(search)
    .placeholder("files, commands")
    .enabled(true)
    .decoration(decoration.clone().label_text("Search")
        .prefix_icon(Text::new("⌕")).suffix_icon(Text::new("↵")).helper_text("by name"));
let editor = password.clone();
let password = ListenableBuilder::new(password.listenable(), move |_, _| {
    let text = editor.text();
    let mut decoration = decoration.clone().label_text("Password");
    if !text.is_empty() && grapheme_boundaries(&text).count() < 8 {
        decoration = decoration.error_text("at least 8 chars");
    }
    TextField::new()
        .controller(editor.clone())
        .placeholder("secret")
        .obscure_text(true)
        .max_length(16)
        .enabled(true)
        .decoration(decoration)
        .into()
});

row![
    SizedBox::new(Some(24), None, Some(
        column![themes, site].main_axis_size(MainAxisSize::Min).spacing(1).into(),
    )),
    SizedBox::new(Some(24), None, Some(
        column![search, password].main_axis_size(MainAxisSize::Min).spacing(1).into(),
    )),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(2)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| border | outline, underline, none | outline |
| filled | on, off | false |
| enabled | on, off | true |

### Methods

#### `InputDecoration`: constructors and associated functions

- `pub fn new() -> InputDecoration` - A decoration of nothing but a line under the field.

#### `InputDecoration`: builder methods (take and return `self`)

- `pub fn icon(mut self, icon: impl Into<AnyWidget>) -> InputDecoration` - With `icon` before the field.
- `pub fn label_text(mut self, label: impl Into<Str>) -> InputDecoration` - Labelled `label`.
- `pub fn helper_text(mut self, text: impl Into<Str>) -> InputDecoration` - With `text` under the field.
- `pub fn error_text(mut self, text: impl Into<Str>) -> InputDecoration` - With the error `text` under the field.
- `pub fn counter_text(mut self, text: impl Into<Str>) -> InputDecoration` - With the counter `text`.
- `pub fn counter(mut self, counter: impl Into<AnyWidget>) -> InputDecoration` - With `counter` for a counter.
- `pub fn prefix_icon(mut self, icon: impl Into<AnyWidget>) -> InputDecoration` - With `icon` inside the border, before the text.
- `pub fn prefix_text(mut self, text: impl Into<Str>) -> InputDecoration` - With `text` right before the text.
- `pub fn suffix_icon(mut self, icon: impl Into<AnyWidget>) -> InputDecoration` - With `icon` inside the border, after the text.
- `pub fn suffix_text(mut self, text: impl Into<Str>) -> InputDecoration` - With `text` right after the text.
- `pub fn border(mut self, border: impl Into<InputBorder>) -> InputDecoration` - With `border`.
- `pub fn filled(mut self, filled: bool) -> InputDecoration` - Filled or not.
- `pub fn fill_style(mut self, style: Style) -> InputDecoration` - Filled with `style`.

#### `InputDecoration`: methods

- `pub fn copy_with(&self, f: impl FnOnce(&mut InputDecoration)) -> InputDecoration` - This decoration changed by `f` (twk's `copyWith`).

### Also used in the code

- `rtwk::editing::OutlineInputBorder` (struct, `crates/rtwk-editing/src/input_decoration.rs:83`) A box around the row of an `InputDecorator`, its label on the box's top edge and a cell between its sides and the row: three rows.
- `rtwk::controls::Select` (struct, `crates/rtwk-controls/src/select.rs:330`) A drop-down: a field that shows the label of the choice whose value is `value`, and opens a list of the `choices` to choose another from.
  - public fields: `choices: Rc<[Choice<T>]>`, `value: Option<T>`, `on_changed: Option<Handler<T>>`, `controller: Option<PopupController>`, `hint: Str`, `inline: bool`, `rows: usize`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`, `arrow_style: Option<Style>`, `popup_look: PanelLook`, `popup_fade_in: Duration`, `decoration: Option<InputDecoration>`
- `rtwk::controls::Choice` (struct, `crates/rtwk-controls/src/choice.rs:68`) One of the values a `RadioGroup`, a `Select` or a `MultiSelect` offers: `value`, shown as `label`.
  - public fields: `value: T`, `label: Str`, `enabled: bool`
- `rtwk::editing::TextField` (struct, `crates/rtwk-editing/src/text_field.rs:160`) A field of a line of text the user edits, which its `controller` holds (twk's `TextField`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `text_align: TextAlign`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`

## TextArea

Lines of text to edit, wrapped to the width: Enter breaks a line, the area grows from its least lines to its most, then scrolls.

API:

- `rtwk::editing::TextArea` (struct, `crates/rtwk-editing/src/text_area.rs:109`) A text area: lines of text the user edits, which its `controller` holds, each line broken by word into rows as wide as the area (twk's `TextArea`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `min_lines: usize`, `max_lines: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_edited: Option<Handler<TextEditingValue>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`, `disabled_style: Option<Style>`, `cursor_shape: CursorShape`
- `rtwk::editing::TextEditingController` (struct, `crates/rtwk-editing/src/text_editing.rs:418`) The `TextEditingValue` a field edits, which notifies its listeners of each change.

### Code

The gallery's Code tab at the initial options (route `#/editing/textarea`, `crates/rtwk-gallery/src/pages/editing.rs:66`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn focus_boxed(child: impl Into<AnyWidget>) -> FocusBuilder {
    FocusBuilder::new(|cx, focused, child| {
        let theme = Theme::of(cx);
        BoxWidget::new()
            .border(Border::all(if *focused {
                theme.primary.clone()
            } else {
                theme.border.clone()
            }).kind(BorderType::Rounded))
            .child(child.expect("editing field").clone())
            .into()
    }).child(child)
}

// Keep one controller initialized with this text in your state:
// let controller = TextEditingController::new("Notes wrap at the width of the area, between words.\nEnter breaks a line.");
let area = TextArea::new()
    .controller(controller.clone())
    .placeholder("notes")
    .min_lines(3)
    .max_lines(6)
    .cursor_shape(CursorShape::Bar)
    .read_only(false)
    .enabled(true);
let editor = controller.clone();
let caret_line = ListenableBuilder::new(controller.listenable(), move |cx, _| {
    let text = editor.text();
    let before = &text[..editor.selection().extent_offset];
    let line = before.split('\n').count();
    let lines = text.split('\n').count();
    let column = grapheme_boundaries(before.rsplit('\n').next().unwrap_or("")).count() + 1;
    Text::new(format!("line {line} of {lines}, column {column}"))
        .style(Theme::of(cx).muted.clone())
        .into()
});
let status = column![caret_line,
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start);

column![
    SizedBox::new(Some(40), None, Some(focus_boxed(area).into())),
    status,
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| minLines | 1, 3 | 3 |
| maxLines | none, 3, 6 | 6 |
| enter | newline, submit | newline |
| cursor | bar, block, underline, terminalDefault | bar |
| blink | on, off | false |
| readOnly | on, off | false |
| enabled | on, off | true |

### Methods

#### `TextArea`: constructors and associated functions

- `pub fn new() -> TextArea` - An area of a controller of its own, empty, one row at least.

#### `TextArea`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<TextEditingController>) -> TextArea` - Editing `controller`.
- `pub fn placeholder(mut self, placeholder: impl Into<Str>) -> TextArea` - Showing `placeholder` while empty.
- `pub fn semantics_label(mut self, label: impl Into<Str>) -> TextArea` - Called `label` by a screen reader.
- `pub fn min_lines(mut self, n: usize) -> TextArea` - At least `n` rows.
- `pub fn max_lines(mut self, n: usize) -> TextArea` - At most `n` rows.
- `pub fn on_changed(mut self, h: Handler<Rc<str>>) -> TextArea` - Telling `h` each new text.
- `pub fn on_edited(mut self, h: Handler<TextEditingValue>) -> TextArea` - Telling `h` each new value.
- `pub fn on_submitted(mut self, h: Handler<Rc<str>>) -> TextArea` - Telling `h` the text on Enter.
- `pub fn on_editing_complete(mut self, h: Handler<()>) -> TextArea` - Calling `h` when Enter submits and as the focus leaves after a change.
- `pub fn on_paste(mut self, f: impl Fn(&str) -> bool + 'static) -> TextArea` - Asking `f` first with what is pasted.
- `pub fn read_only(mut self, on: bool) -> TextArea` - Read-only.
- `pub fn enabled(mut self, on: bool) -> TextArea` - Taking the focus and the mouse.
- `pub fn focus_node(mut self, node: FocusNode) -> TextArea` - With `node` for its focus.
- `pub fn autofocus(mut self, on: bool) -> TextArea` - Taking the focus when first shown.
- `pub fn can_request_focus(mut self, on: bool) -> TextArea` - Whether its node can take the focus.
- `pub fn style(mut self, style: Style) -> TextArea` - In `style`.
- `pub fn placeholder_style(mut self, style: Style) -> TextArea` - The placeholder in `style`.
- `pub fn selection_style(mut self, style: Style) -> TextArea` - The selection in `style`.
- `pub fn disabled_style(mut self, style: Style) -> TextArea` - In `style` over the style while disabled.
- `pub fn cursor_shape(mut self, shape: CursorShape) -> TextArea` - The cursor in `shape`.
- `pub fn key_hints(mut self, on: bool) -> TextArea` - The keys' hints listed or not.

#### `TextEditingController`: constructors and associated functions

- `pub fn new(text: &str) -> TextEditingController` - `text` with the caret at its end.
- `pub fn from_value(value: TextEditingValue) -> TextEditingController` - `value`, kept within its text.

#### `TextEditingController`: methods

- `pub fn listenable(&self) -> Rc<impl Listenable + 'static>` - The same notification source for every clone of this controller.
- `pub fn value(&self) -> TextEditingValue` - The value: the text, the selection, the composing (a clone: the text is shared, not copied).
- `pub fn with_value_ref<R>(&self, f: impl FnOnce(&TextEditingValue) -> R) -> R` - Reads the value without cloning it.
- `pub fn buffer(&self) -> TextBuffer` - The text in pieces (twk's `textBufferOf(controller.value)`).
- `pub fn set_value(&self, new_value: TextEditingValue)` - Sets the value, kept within its text; a change of the text is a step to undo.
- `pub fn can_undo(&self) -> bool` - Whether there is a step to take back.
- `pub fn can_redo(&self) -> bool` - Whether there is a step undone to make again.
- `pub fn undo(&self)` - Takes the last step back: its text as it was, and the selection as it was before it.
- `pub fn redo(&self)` - Makes the last step undone again, the selection as it was after it.
- `pub fn text(&self) -> Rc<str>` - The text, put together from its pieces the first time it is asked for after a change.
- `pub fn set_text(&self, text: &str)` - Sets the text, the caret at its end, the composing ended.
- `pub fn selection(&self) -> TextSelection` - What is selected, the caret at its `extent_offset`.
- `pub fn set_selection(&self, selection: TextSelection)` - Sets what is selected.
- `pub fn composing(&self) -> TextRange` - The part of the text being composed; `TextRange::EMPTY` for none.
- `pub fn set_composing(&self, composing: TextRange)` - Sets the part being composed.
- `pub fn clear(&self)` - Empties the text.
- `pub fn reset(&self, text: &str)` - Puts `text` in, the caret at its end and the composing ended, and starts the history afresh: the text is no step to undo, and the steps before it, those undone too, are gone.
- `pub fn replace(&self, start: usize, end: usize, text: &str)` - Puts `text` in place of the part from `start` to `end`, the caret after it.
- `pub fn insert(&self, text: &str)` - Puts `text` in place of the selection, the caret after it.
- `pub fn move_caret(&self, offset: usize, extend: bool)` - Moves the caret to `offset`: the selection ends, or with `extend` it grows or shrinks from its base to there.
- `pub fn delete_to(&self, offset: usize)` - Deletes what is selected, if anything; else the text between the caret and `offset`.
- `pub fn select_all(&self)` - Selects the whole text, the caret at its end.
- `pub fn notifier(&self) -> &ChangeNotifier` - The notifier the listeners are on.
- `pub fn notify_listeners(&self)` - Tells the listeners now (twk's `notifyListeners`).
- `pub fn dispose(&self)` - Releases the shared listeners for every clone (twk's `dispose`).

### Also used in the code

- `rtwk::focus::FocusBuilder` (struct, `crates/rtwk-framework/src/focus/widgets.rs:905`) Shows what its builder returns for whether the focus is in this widget's subtree, built again when the focus enters or leaves it; the widgets around it are not (twk's `FocusBuilder`).
  - public fields: `builder: ValueWidgetBuilder<bool>`, `child: Option<AnyWidget>`, `debug_label: Option<Str>`
- `rtwk_core::presenter::CursorShape` (enum, `crates/rtwk-core/src/presenter/mod.rs:121`) The cursor's shape (`DECSCUSR`).
  - variants: `TerminalDefault`, `BlinkingBlock`, `Block`, `BlinkingUnderline`, `Underline`, `BlinkingBar`, `Bar`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`

## Autocomplete

Suggestions under a field as one types, after a trigger or for the whole text: Up and Down pick one, Enter or Tab puts it in.

API:

- `rtwk::editing::Autocomplete` (struct, `crates/rtwk-editing/src/autocomplete.rs:126`) Suggests what to put in the field in `child` that edits `controller`, a `TextField` or a `TextArea`, as the user types.
  - public fields: `controller: TextEditingController`, `suggestions: Suggest`, `child: AnyWidget`, `trigger: Option<Str>`, `on_selected: Option<Handler<Suggestion>>`, `rows: usize`, `key_hints: bool`

### Code

The gallery's Code tab at the initial options (route `#/editing/autocomplete`, `crates/rtwk-gallery/src/pages/editing.rs:92`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn focus_boxed(child: impl Into<AnyWidget>) -> FocusBuilder {
    FocusBuilder::new(|cx, focused, child| {
        let theme = Theme::of(cx);
        BoxWidget::new()
            .border(Border::all(if *focused {
                theme.primary.clone()
            } else {
                theme.border.clone()
            }).kind(BorderType::Rounded))
            .child(child.expect("editing field").clone())
            .into()
    }).child(child)
}

// Keep the text controller and the chosen suggestion in application state.
// The Overlay host positions suggestions next to the field.
let commands = [("help", "the keys"), ("open", "a file"), ("quit", "the app"), ("save", "the file"), ("search", "in the files"), ("settings", "of the app"), ("share", "a link"), ("sort", "the lines")];
let suggest = move |query: &str| commands.into_iter()
    .filter(|(name, _)| fuzzy_match(query, name).is_some())
    .map(|(name, detail)| Suggestion::new(name).detail(detail))
    .collect();
let field = TextField::new().controller(controller.clone()).placeholder("/ and a command");
let autocomplete = Autocomplete::new(controller, suggest, focus_boxed(field))
    .trigger("/")
    .rows(8)
    .on_selected(on_selected)
    .popup_look(PanelLook::Raised);
let muted = Theme::of(cx).muted.clone();
let chosen = Text::rich([
    Span::styled("chosen: ", muted.clone()),
    match chosen {
        Some(value) => Span::new(value),
        None => Span::styled("nothing", muted),
    },
]);

column![
    SizedBox::new(Some(30), None, Some(autocomplete)),
    chosen,
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| trigger | /, none | / |
| rows | 8, 4 | 8 |
| popup | raised, bordered | raised |

### Methods

#### `Autocomplete`: constructors and associated functions

- `pub fn new(controller: impl Into<TextEditingController>, suggestions: impl Fn(&str) -> Vec<Suggestion> + 'static, child: impl Into<AnyWidget>) -> Autocomplete` - Suggestions for the field in `child` that edits `controller`.
- `pub fn deferred(controller: impl Into<TextEditingController>, on_request: Handler<SuggestionRequest>, child: impl Into<AnyWidget>) -> DeferredAutocomplete` - Asks the application for each query.

#### `Autocomplete`: builder methods (take and return `self`)

- `pub fn trigger(mut self, trigger: impl Into<Str>) -> Autocomplete` - Queries after `trigger`.
- `pub fn on_selected(mut self, h: Handler<Suggestion>) -> Autocomplete` - Telling `h` each suggestion put in.
- `pub fn rows(mut self, rows: usize) -> Autocomplete` - Showing at most `rows` at once.
- `pub fn inline_suggestions(self) -> AnyWidget` - Finishes building this widget with suggestions in the field's layout, even inside an overlay.
- `pub fn popup_look(self, look: PanelLook) -> AnyWidget` - Finishes building this widget with the selected look for suggestions in an overlay.
- `pub fn popup_options(self, options: AutocompletePopup) -> AnyWidget` - Finishes building this widget with the given overlay popup options.
- `pub fn key_hints(mut self, on: bool) -> Autocomplete` - The keys' hints listed or not.

### Also used in the code

- `rtwk::focus::FocusBuilder` (struct, `crates/rtwk-framework/src/focus/widgets.rs:905`) Shows what its builder returns for whether the focus is in this widget's subtree, built again when the focus enters or leaves it; the widgets around it are not (twk's `FocusBuilder`).
  - public fields: `builder: ValueWidgetBuilder<bool>`, `child: Option<AnyWidget>`, `debug_label: Option<Str>`
- `rtwk::editing::Suggestion` (struct, `crates/rtwk-editing/src/autocomplete.rs:42`) What an `Autocomplete` suggests: `text` to put in place of the query, shown as `label` with `detail` after it.
  - public fields: `text: Str`, `label: Option<Str>`, `detail: Option<Str>`
- `rtwk::editing::TextField` (struct, `crates/rtwk-editing/src/text_field.rs:160`) A field of a line of text the user edits, which its `controller` holds (twk's `TextField`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `text_align: TextAlign`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
