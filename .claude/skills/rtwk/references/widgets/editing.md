<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Editing

Text fields and what is drawn around them, text areas and completion as the user types.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## TextField

A line of text to edit: Tab to it and type; Enter submits it, Ctrl+Z undoes.

- `rtwk::editing::TextField` (struct) in `crates/rtwk-editing/src/text_field.rs:164`: A field of a line of text the user edits, which its controller holds (twk's `TextField`).

### Code

```rust
struct Field;

struct FieldState {
    controller: TextEditingController,
    submitted: String,
    heard: String,
    sender: Option<Sender<Msg>>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Submitted(String),
    Heard(String),
}

impl StatefulWidget for Field {
    type State = FieldState;
    fn create_state(&self) -> FieldState {
        FieldState {
            controller: TextEditingController::default(),
            submitted: String::new(),
            heard: String::new(),
            sender: None
        }
    }
}

impl State for FieldState {
    type Widget = Field;
    type Msg = Msg;

    fn init_state(&mut self, _: &Field, cx: &mut Cx<'_, Self>) {
        self.sender = Some(cx.channel());
    }

    fn update(&mut self, msg: Msg, _: &Field, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Submitted(text) => self.submitted = text,
            Msg::Heard(text) => self.heard = text,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Field, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let sender = self.sender.as_ref().expect("field initialized").clone();
        let mut field = TextField::new()
            .controller(self.controller.clone())
            .placeholder("type here")
            .obscure_text(false)
            .cursor_shape(CursorShape::Bar)
            .text_align(TextAlign::Left)
            .read_only(false)
            .enabled(true)
            .on_submitted(cx.map(|text: Rc<str>| Msg::Submitted(text.to_string())))
            .on_editing_complete(cx.send(Msg::Heard("editing complete".into())))
            .on_paste(move |text| {
                let _ = sender.send(Msg::Heard(format!(
                    "a paste of {} characters",
                    grapheme_boundaries(text).count(),
                )));
                false
            });
        field.max_length = None;
        field
        .into()
    }
}
impl_widget!(stateful Field);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| obscured | false, true | false |
| max length | none, 8 | none |
| cursor | bar, block, underline, terminal | bar |
| blinking | false, true | false |
| text align | left, center, right, start, end | left |
| read only | false, true | false |
| enabled | false, true | true |
| a paste | goes in, taken | goes in |

## InputDecoration

What is drawn around a field: a label, an icon, a prefix and a suffix, a helper, an error and a counter, in a box, over a line or bare, filled.

- `rtwk::editing::InputDecoration` (struct) in `crates/rtwk-editing/src/input_decoration.rs:142`: What an InputDecorator draws around a field: an icon before it, a label naming it, what goes before and after its text, a border around or under it, a fill and a row under it of a helper or an error text and a counter.

### Code

```rust
fn decoration(theme: &'static str, site: TextEditingController, search: TextEditingController, password: TextEditingController, on_theme: Handler<&'static str>) -> Row {
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
        .placeholder("rtwk")
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| border | outline, underline, none | outline |
| filled | false, true | false |
| enabled | false, true | true |

## TextArea

Lines of text to edit, wrapped to the width: Enter breaks a line, the area grows from its least lines to its most, then scrolls.

- `rtwk::editing::TextArea` (struct) in `crates/rtwk-editing/src/text_area.rs:111`: A text area: lines of text the user edits, which its controller holds, each line broken by word into rows as wide as the area (twk's `TextArea`).

### Code

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

fn notes(controller: TextEditingController) -> Column {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| min lines | 1, 3 | 3 |
| max lines | none, 3, 6 | 6 |
| enter | breaks the line, submits | breaks the line |
| cursor | bar, block, underline, terminal | bar |
| blinking | false, true | false |
| read only | false, true | false |
| enabled | false, true | true |

## Autocomplete

Suggestions under a field as one types, after a trigger or for the whole text: Up and Down pick one, Enter or Tab puts it in.

- `rtwk::editing::Autocomplete` (struct) in `crates/rtwk-editing/src/autocomplete.rs:227`: Suggests what to put in the field in `child` that edits `controller`, a `TextField` or a `TextArea`, as the user types.

### Code

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

fn completion(cx: &mut Cx<'_>, controller: TextEditingController, chosen: Option<String>, on_selected: Handler<Suggestion>) -> Column {
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
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| trigger | /, none | / |
| rows | 8, 4 | 8 |
| popup look | raised, bordered | raised |
