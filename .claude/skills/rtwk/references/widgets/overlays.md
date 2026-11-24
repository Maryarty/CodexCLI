<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Overlays

Dialogs, toasts, tooltips, popovers, menus and the command palette, over the app.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Overlay

A layer over the app that dialogs, toasts, menus and tooltips go in; an entry of its own goes in it too.

- `rtwk::overlay::Overlay` (struct) in `crates/rtwk-overlay-base/src/layer.rs:67`: Shows `child`, and over it the widgets its entries build, each entry over those inserted below it (OverlayState::insert); the widgets below find the nearest one with Overlay::of and put their dialogs (`show_dialog`), toasts (show_toast) and tooltips (`Tooltip`) in it.
- `rtwk::overlay::OverlayEntry` (struct) in `crates/rtwk-overlay-base/src/layer.rs:582`: A layer of an Overlay: the widget its builder builds, shown while the entry is inserted (OverlayState::insert) until remove.

### Code

```rust
struct Entries;

struct EntriesState {
    entries: Vec<OverlayEntry>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Insert,
    Remove,
}

impl StatefulWidget for Entries {
    type State = EntriesState;
    fn create_state(&self) -> EntriesState {
        EntriesState { entries: Vec::new() }
    }
}

impl State for EntriesState {
    type Widget = Entries;
    type Msg = Msg;

    // On dispose, remove all.
    fn dispose(&mut self) {
        for entry in &self.entries {
            entry.remove();
        }
    }

    fn update(&mut self, msg: Msg, _: &Entries, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // Insert adds an entry on top of the others; Remove takes the last one off.
            Msg::Insert if self.entries.len() < 3 => {
                let index = i32::try_from(self.entries.len()).expect("at most three entries") + 1;
                let entry = OverlayEntry::new(move |cx| {
                    let theme = Theme::of(cx);
                    let panel = Panel::new(Text::rich([
                        Span::new(format!("Entry {index}")).bold(),
                        Span::styled(" of the overlay", theme.muted.clone()),
                    ]))
                    .look(PanelLook::Raised)
                    .padding(EdgeInsets::x(1))
                    .shadow(true);
                    Positioned::new(panel).right(4 * index - 3).top(2 * index - 1).into()
                });
                Overlay::of(cx).insert(&entry);
                self.entries.push(entry);
            }
            Msg::Remove => {
                if let Some(entry) = self.entries.pop() {
                    entry.remove();
                }
            }
            Msg::Insert => return Rebuild::No,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Entries, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let len = self.entries.len();
        let mut add = Button::new("Insert an entry");
        if len < 3 { add = add.on_pressed(cx.send(Msg::Insert)); }
        let mut remove = Button::new("Remove one");
        if len != 0 { remove = remove.on_pressed(cx.send(Msg::Remove)); }
        column![
            row![add, remove].main_axis_size(MainAxisSize::Min).spacing(2),
            Text::rich([
                Span::styled("entries: ", theme.muted.clone()),
                Span::new(len.to_string()),
                Span::styled(" of the overlay", theme.muted.clone()),
            ]),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Entries);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| positioned | TOP_RIGHT, TOP_LEFT, BOTTOM_LEFT | TOP_RIGHT |
| shadow | false, true | true |
| look | raised, bordered | raised |

## App

The root of an app: its theme over the Overlay its dialogs show in, over the app and its own keys.

- `rtwk::overlay::App` (struct) in `crates/rtwk-overlay/src/app.rs:39`: `child` under theme and the Overlay that its dialogs, toasts, menus and tooltips show in, which a run loop leaves to the app to place.

### Code

```rust
#[derive(Debug, PartialEq)]
struct AppAsk {
    fade_in: Duration,
}

struct AppAskState;

#[derive(Clone, PartialEq)]
enum Msg {
    Ask,
}

impl StatefulWidget for AppAsk {
    type State = AppAskState;
    fn create_state(&self) -> AppAskState {
        AppAskState
    }
}

impl State for AppAskState {
    type Widget = AppAsk;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, widget: &AppAsk, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // Nothing waits for the result.
            Msg::Ask => {
                let _result = show_dialog::<(), _>(
                    cx,
                    |cx| Dialog::new(
                        Button::new("OK")
                            .autofocus(true)
                            .on_pressed(close_dialog_handler::<_, ()>(cx, None)),
                    )
                    .title("In the theme")
                    .into(),
                    DialogOptions { fade_in: widget.fade_in, ..DialogOptions::default() },
                );
                return Rebuild::No;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &AppAsk, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Center::new(Button::new("Ask").on_pressed(cx.send(Msg::Ask)))
        .into()
    }
}
impl_widget!(stateful eq AppAsk);

fn preview(fade_in: Duration) -> AnyWidget {
    // The nested App owns the overlay within this 32 × 9 box.
    SizedBox::new(
        Some(32),
        Some(9),
        Some(App::new(AppAsk { fade_in }).theme(ThemeData::NORD).into()),
    )
    .into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| theme | nord, light, dark | nord |

## Dialog

A box over the app, the rest dimmed behind a barrier; Tab stays in it, Escape closes it.

- `rtwk::overlay::Dialog` (struct) in `crates/rtwk-overlay/src/dialog.rs:751`: `child` in a box that stands out of what is around it: a raised panel (look) of the theme's surface and text, a cell of room around padding and no outline, title in bold on the first row inside, over the child; and shadow cast beside it.

### Code

```rust
struct Ask;

struct AskState {
    answer: Option<&'static str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Open,
    Answer(Option<bool>),
}

impl StatefulWidget for Ask {
    type State = AskState;
    fn create_state(&self) -> AskState {
        AskState { answer: None }
    }
}

impl State for AskState {
    type Widget = Ask;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Ask, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Open => {
                let dialog = show_dialog::<bool, _>(
                    cx,
                    move |_| DeletePrompt { shadow: false, look: PanelLook::Raised }.into(),
                    DialogOptions {
                        alignment: Alignment::CENTER,
                        barrier_dismissible: true,
                        ..DialogOptions::default()
                    },
                );
                cx.spawn(async move { Some(Msg::Answer(dialog.await)) });
                return Rebuild::No;
            }
            Msg::Answer(answer) => {
                self.answer = Some(match answer {
                    Some(true) => "deleted",
                    Some(false) => "kept",
                    None => "dismissed",
                });
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Ask, cx: &mut Cx<'_, Self>) -> AnyWidget {
        column![
            Button::new("Ask").on_pressed(cx.send(Msg::Open)),
            Text::new(format!("answer: {}", self.answer.unwrap_or("nothing"))),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Ask);

#[derive(Debug, PartialEq)]
struct DeletePrompt {
    shadow: bool,
    look: PanelLook,
}

struct DeletePromptState;

impl StatefulWidget for DeletePrompt {
    type State = DeletePromptState;
    fn create_state(&self) -> DeletePromptState {
        DeletePromptState
    }
}

impl State for DeletePromptState {
    type Widget = DeletePrompt;
    type Msg = bool;

    // On Delete or Keep, the dialog closes with the answer.
    fn update(&mut self, delete: bool, _: &DeletePrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        close_dialog(cx, Some(delete));
        Rebuild::No
    }

    fn build(&self, _: &DeletePrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Dialog::new(
            column![
                Text::new("Delete 3 files?"),
                row![
                    Button::new("Delete").autofocus(true).on_pressed(cx.send(true)),
                    Button::new("Keep").on_pressed(cx.send(false)),
                ]
                .main_axis_size(MainAxisSize::Min)
                .spacing(2),
            ]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1),
        )
        .title("Delete")
        .look(PanelLook::Raised)
        .into()
    }
}
impl_widget!(stateful eq DeletePrompt);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| alignment | CENTER, TOP_CENTER, BOTTOM_RIGHT | CENTER |
| shadow | false, true | false |
| look | raised, bordered | raised |
| dismissible | false, true | true |

## Toast

A message that shows for a while and goes, over a corner of the app.

- `rtwk::overlay::show_toast` (fn) in `crates/rtwk-overlay-base/src/toast.rs:111`: Shows `message` in a toast of the Overlay above `cx` for the options' duration, three seconds by default, then takes it away.
- `rtwk::overlay::ToastOptions` (struct) in `crates/rtwk-overlay-base/src/toast.rs:41`: How show_toast shows a toast: what twk's named parameters say.

### Code

```rust
struct Toaster;

struct ToasterState {
    next: usize,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Show,
}

impl StatefulWidget for Toaster {
    type State = ToasterState;
    fn create_state(&self) -> ToasterState {
        ToasterState { next: 0 }
    }
}

impl State for ToasterState {
    type Widget = Toaster;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Toaster, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Show => {
                let theme = Theme::of(cx);
                let (message, style) = [
                    ("Saved 3 files.", &theme.success),
                    ("Copied the path.", &theme.info),
                    ("Disk nearly full.", &theme.warning),
                    ("Upload failed.", &theme.error),
                ][self.next];
                self.next = (self.next + 1) % 4;
                show_toast(cx, message, ToastOptions {
                    alignment: Alignment::BOTTOM_RIGHT,
                    duration: Duration::from_millis(3000),
                    style: Some(style.clone()),
                    look: PanelLook::Raised,
                    ..ToastOptions::default()
                });
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Toaster, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Button::new("Show a toast").on_pressed(cx.send(Msg::Show))
        .into()
    }
}
impl_widget!(stateful Toaster);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| alignment | BOTTOM_RIGHT, TOP_RIGHT, BOTTOM_CENTER | BOTTOM_RIGHT |
| duration | 1 s, 3 s, 5 s | 3 s |
| look | raised, bordered | raised |

## Tooltip

A message by a widget while the mouse or the keyboard focus rests on it, or while a TooltipController says, centred under it by an AnchorFollower in the overlay; above it where there is no room below.

- `rtwk::overlay::Tooltip` (struct) in `crates/rtwk-overlay/src/tooltip.rs:197`: Shows message by child, in an entry of the Overlay above, centred under it (above it where there is no room under it), once the mouse has rested on the child for wait (half a second), or the focus has been in it as long; it goes when the mouse leaves and the focus goes, or on Escape while the focus is in the child.

### Code

```rust
struct Tips;

struct TipsState {
    share: TooltipController,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Toggle,
    Toast(&'static str),
}

impl StatefulWidget for Tips {
    type State = TipsState;
    fn create_state(&self) -> TipsState {
        TipsState { share: TooltipController::new() }
    }
}

impl State for TipsState {
    type Widget = Tips;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Tips, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // Toggle uses the controller; the other buttons show a toast.
            Msg::Toggle if self.share.is_shown() => self.share.hide(),
            Msg::Toggle => self.share.show(),
            Msg::Toast(message) => {
                let style = Theme::of(cx).success.clone();
                show_toast(cx, message, ToastOptions {
                    style: Some(style),
                    ..ToastOptions::default()
                });
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Tips, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // The Share button keeps a TooltipController, which the extra button uses.
        let row = Row::new().main_axis_size(MainAxisSize::Min).spacing(2)
            .children([("Save", "Saves the file", "Saved"), ("Open", "Opens a file", "Opened"), ("Share", "Copies a link to it", "Link copied")].into_iter().map(|(label, message, done)| {
                let mut tip = Tooltip::new(
                    message,
                    Button::new(label).on_pressed(cx.send(Msg::Toast(done))),
                )
                .wait(Duration::from_millis(500))
                .show_on_hover(true)
                .show_on_focus(true)
                .look(PanelLook::Raised);
                if label == "Share" { tip = tip.controller(&self.share); }
                tip.into()
            }));
        let share = self.share.clone();
        let toggle = cx.send(Msg::Toggle);
        let show = ListenableBuilder::new(share.listenable(), move |_, _| {
            Button::new(if share.is_shown() { "Hide the tip" } else { "Show the tip" })
                .on_pressed(toggle.clone())
                .into()
        });
        column![row, show]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1)
        .into()
    }
}
impl_widget!(stateful Tips);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| wait | 100 ms, 500 ms, 1 s | 500 ms |
| on hover | false, true | true |
| on focus | false, true | true |
| look | raised, bordered | raised |

## Popover

A widget by another while the app says so, placed by an AnchorTarget around it and an AnchorFollower in the overlay; Escape or a click outside closes it.

- `rtwk::overlay::Popover` (struct) in `crates/rtwk-overlay-base/src/popover.rs:75`: Shows popover by child while open, in an entry of the Overlay above: under the child from its left edge; where it does not fit there, on the side with more room, above the child, ending at its right edge; moved in from the edges of the overlay as far as it has to.

### Code

```rust
struct Pop;

struct PopState {
    open: bool,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Toggle,
    Dismiss,
}

impl StatefulWidget for Pop {
    type State = PopState;
    fn create_state(&self) -> PopState {
        PopState { open: false }
    }
}

impl State for PopState {
    type Widget = Pop;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Pop, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // Toggle flips open; Dismiss sets it to false.
            Msg::Toggle => self.open = !self.open,
            Msg::Dismiss => self.open = false,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Pop, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let panel = Panel::new(column![
            Text::new("The popover, by the button."),
            Text::new("Escape or a click outside closes it.").style(theme.muted.clone()),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start))
        .look(PanelLook::Raised)
        .padding(EdgeInsets::x(1));
        Popover::new(
            self.open,
            panel,
            Button::new(if self.open { "Close" } else { "Open" })
                .on_pressed(cx.send(Msg::Toggle)),
            Some(cx.send(Msg::Dismiss)),
        )
        .into()
    }
}
impl_widget!(stateful Pop);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| dismissible | false, true | true |
| look | raised, bordered | raised |

## MenuBar

Menus in a bar: F10 or a click opens one, Alt and a mnemonic too, or a MenuController from the app; the arrows, Enter and the mnemonics choose.

- `rtwk::overlay::MenuBar` (struct) in `crates/rtwk-overlay/src/menu/bar.rs:119`: A row of the titles of menus over child, which fills the rest of the room it gets (along an axis with no bound, it is as long as it likes): the menu bar of an app.

### Code

```rust
struct Editor;

struct EditorState {
    menu: MenuController,
    chosen: Option<&'static str>,
    wrap: bool,
    numbers: bool,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Chosen(&'static str),
    Recent,
    SetWrap(bool),
    ToggleWrap,
    ToggleNumbers,
}

impl StatefulWidget for Editor {
    type State = EditorState;
    fn create_state(&self) -> EditorState {
        EditorState {
            menu: MenuController::new(),
            chosen: None,
            wrap: true,
            numbers: false,
        }
    }
}

impl State for EditorState {
    type Widget = Editor;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Editor, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Chosen(chosen) => self.chosen = Some(chosen),
            // Recent opens File → Open recent.
            Msg::Recent => {
                let _ = self.menu.open(&[0, 2], None);
                return Rebuild::No;
            }
            Msg::SetWrap(on) => self.wrap = on,
            Msg::ToggleWrap => {
                self.wrap = !self.wrap;
                self.chosen = Some("Wrap lines");
            }
            Msg::ToggleNumbers => {
                self.numbers = !self.numbers;
                self.chosen = Some("Line numbers");
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Editor, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // Keep a MenuController in the state.
        let select = |label, action| MenuItem::new(label)
            .on_selected(cx.send(Msg::Chosen(action)));
        let file = Menu::new("&File", [
            select("&New", "New").into(),
            select("&Open…", "Open").into(),
            Menu::new("Open &recent",
                ["main.rs", "page.rs", "overlay.rs"].map(|file| select(file, file).into())
            ).into(),
            MenuDivider.into(),
            select("&Save", "Save").into(),
            MenuItem::new("&Print").into(),
        ]);
        let edit = Menu::new("&Edit", [
            select("&Undo", "Undo").into(),
            MenuItem::new("&Redo").into(),
            MenuDivider.into(),
            select("Cu&t", "Cut").into(),
            select("&Copy", "Copy").into(),
            select("&Paste", "Paste").into(),
        ]);
        let view = Menu::new("&View", [
            MenuItem::new("&Wrap lines").checked(self.wrap)
                .on_selected(cx.send(Msg::ToggleWrap)).into(),
            MenuItem::new("Line &numbers").checked(self.numbers)
                .on_selected(cx.send(Msg::ToggleNumbers)).into(),
        ]);
        let body = Padding::new(EdgeInsets::all(1), column![
            Text::new(format!("chosen: {}", self.chosen.unwrap_or("nothing"))),
            row![
                Checkbox::new(self.wrap).label("wrap lines").on_changed(cx.map(Msg::SetWrap)),
                Button::new("Recent files").on_pressed(cx.send(Msg::Recent)),
            ].spacing(2),
        ].cross_axis_alignment(CrossAxisAlignment::Start).spacing(1));
        let body = KeyBindings::new([
            ("ctrl+o", cx.send(Msg::Chosen("Open"))),
            ("ctrl+s", cx.send(Msg::Chosen("Save"))),
            ("ctrl+z", cx.send(Msg::Chosen("Undo"))),
        ], body);
        let menu = MenuBar::new([file, edit, view], body)
            .look(PanelLook::Raised)
            .controller(&self.menu);
        SizedBox::new(Some(48), Some(8), Some(menu.into()))
        .into()
    }
}
impl_widget!(stateful Editor);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| keys bound | false, true | true |
| look | raised, bordered | raised |

## ContextMenu

A menu at the mouse on a right click, under the focus on Shift+F10, or where a MenuController opens it.

- `rtwk::overlay::ContextMenu` (struct) in `crates/rtwk-overlay/src/menu/context.rs:232`: child, with a menu of items that a right click on it opens at the mouse, and Shift+F10 under the focused widget while the focus is in it (show_context_menu); in a layer of the focus tree around it, whose Shift+F10 a key hints bar lists, `shift+f10 menu`.

### Code

```rust
struct Notes;

struct NotesState {
    menu: MenuController,
    chosen: Option<&'static str>,
    copied: bool,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Choose(&'static str),
    Open,
    Focus,
}

impl StatefulWidget for Notes {
    type State = NotesState;
    fn create_state(&self) -> NotesState {
        NotesState {
            menu: MenuController::new(),
            chosen: None,
            copied: false,
        }
    }
}

impl State for NotesState {
    type Widget = Notes;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Notes, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Choose(action) => {
                // Cut or Copy enables Paste.
                self.copied |= matches!(action, "Cut" | "Copy");
                self.chosen = Some(action);
            }
            // Open positions the menu within the panel.
            Msg::Open => {
                let _ = self.menu.open(&[], Some((22, 1)));
                return Rebuild::No;
            }
            Msg::Focus => return Rebuild::No,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Notes, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let select = |label, action| MenuItem::new(label)
            .on_selected(cx.send(Msg::Choose(action)));
        let paste = if self.copied { select("&Paste", "Paste") } else { MenuItem::new("&Paste") };
        let items = [
            select("Cu&t", "Cut").into(),
            select("&Copy", "Copy").into(),
            paste.into(),
            MenuDivider.into(),
            select("Select &all", "Select all").into(),
        ];
        let panel = BoxWidget::new()
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .padding(EdgeInsets::x(1))
            .child(column![
                Text::new("Right click in here."),
                row![
                    Button::new("Or focus me").on_pressed(cx.send(Msg::Focus)),
                    Button::new("Open the menu").on_pressed(cx.send(Msg::Open)),
                ].spacing(2),
                Text::new(format!("chosen: {}", self.chosen.unwrap_or("nothing"))),
            ].cross_axis_alignment(CrossAxisAlignment::Start));
        ContextMenu::new(items, SizedBox::new(Some(40), Some(5), Some(panel.into())))
            .controller(&self.menu)
            .look(PanelLook::Raised)
        .into()
    }
}
impl_widget!(stateful Notes);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| look | raised, bordered | raised |

## CommandPalette

What can be done, found by typing: Tab to a button, then Ctrl+K, or press Rust files, whose CommandPaletteController opens it with .rs typed; commands_for adds the files that match. The gallery's own, Ctrl+P, goes to a page.

- `rtwk::overlay::CommandPalette` (struct) in `crates/rtwk-overlay/src/command_palette.rs:451`: A palette of commands over `child`, opened by a key (Ctrl+P): a list over the app, in an entry of the Overlay above, of the commands that can be done from the node that had the focus, then the keys hinted there (each key listed under its hint), each with the key that does the same at its right.

### Code

```rust
struct Runner;

struct RunnerState {
    palette: CommandPaletteController,
    ran: Option<String>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Run(&'static str),
    File(&'static str),
    Open,
}

impl StatefulWidget for Runner {
    type State = RunnerState;
    fn create_state(&self) -> RunnerState {
        RunnerState {
            palette: CommandPaletteController::new(),
            ran: None,
        }
    }
}

impl State for RunnerState {
    type Widget = Runner;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Runner, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Run(label) => self.ran = Some(label.into()),
            Msg::File(file) => self.ran = Some(format!("open {file}")),
            // Open uses a prefilled query.
            Msg::Open => {
                let _ = self.palette.open(".rs");
                return Rebuild::No;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Runner, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let body = KeyBindings::new([
            ("ctrl+b", cx.send(Msg::Run("Build"))),
            ("f8", cx.send(Msg::Run("Run tests"))),
            ("ctrl+s", cx.send(Msg::Run("Save"))),
        ], column![
            row![
                Button::new("Focus me").on_pressed(cx.send(Msg::Run("press"))),
                Button::new("Rust files").on_pressed(cx.send(Msg::Open)),
            ].main_axis_size(MainAxisSize::Min).spacing(2),
            Text::new(format!("ran: {}", self.ran.as_deref().unwrap_or("nothing"))),
        ].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1))
        .hints([("ctrl+s", "Save")]);
        let palette = CommandPalette::new(body)
            .controller(&self.palette)
            .open_key("ctrl+k")
            .width(40).rows(10)
            .look(PanelLook::Raised)
            .rule(false).bar(false).thumb(false)
            .commands(["Build", "Run tests", "Format", "Deploy"].map(|label| {
                Command::new(label).on_selected(cx.send(Msg::Run(label)))
            }));

        let files = ["src/main.rs", "src/app.rs", "src/theme.rs", "tests/app.rs", "README.md", "Cargo.toml"].map(|file| {
            Command::new(file).on_selected(cx.send(Msg::File(file)))
        });
        let palette = palette.commands_for(move |query| {
            CommandsAnswer::Now(files.iter().filter(|item| {
                !query.is_empty() && fuzzy_match(query, &item.label).is_some()
            }).cloned().collect())
        });

        palette
        .into()
    }
}
impl_widget!(stateful Runner);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 40, 60 | 40 |
| rows | 3, 10 | 10 |
| files | false, true | true |
| rule | false, true | false |
| bar | false, true | false |
| thumb | false, true | false |
| placeholder | false, true | false |
| look | raised, bordered | raised |

## OptionList

Options in rows, one highlighted, the characters a query matched standing out and a detail at the right edge: what an autocomplete, a Select and a command palette list.

- `rtwk::overlay::OptionList` (struct) in `crates/rtwk-overlay-base/src/options.rs:143`: options one a row in the theme's surface and text, a column of room either side, each label cut with the theme's ellipsis where the row is too narrow; the highlighted one in the theme's selection, with a bar of the theme's cursor mark at its left (bar); at most rows rows, scrolled to keep the highlighted one in view, a slim thumb at the right edge saying where (thumb).

### Code

```rust
fn items() -> Vec<OptionItem> {
    let commands = [("Open file", "ctrl+o"), ("Save", "ctrl+s"), ("Save as…", "ctrl+shift+s"), ("Close tab", "ctrl+w"), ("Find", "ctrl+f"), ("Replace", "ctrl+h"), ("Go to line", "ctrl+g"), ("Toggle theme", "ctrl+t"), ("Quit", "ctrl+q")];
    commands.into_iter().filter_map(|(label, detail)| {
        let matched = fuzzy_match("", label)?;
        let option = OptionItem::new(label).marks(matched.positions);
    let option = option.detail(detail);
        Some(option)
    }).collect()
}

struct Commands;

struct CommandsState {
    highlighted: i32,
    ran: Option<Str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Highlight(i32),
    Move(i32),
    Run(i32),
}

impl StatefulWidget for Commands {
    type State = CommandsState;
    fn create_state(&self) -> CommandsState {
        CommandsState { highlighted: 0, ran: None }
    }
}

impl State for CommandsState {
    type Widget = Commands;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Commands, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // The update stores the highlight, clamps keyboard moves and records the selected label.
            Msg::Highlight(index) => self.highlighted = index,
            Msg::Move(by) => {
                let last = i32::try_from(items().len()).unwrap() - 1;
                self.highlighted = (self.highlighted.clamp(0, last) + by).clamp(0, last);
            }
            Msg::Run(index) => {
                let Some(item) = usize::try_from(index).ok().and_then(|index| items().into_iter().nth(index)) else {
                    return Rebuild::No;
                };
                self.ran = Some(item.label.clone());
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Commands, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let items = items();
        let highlighted = self.highlighted.clamp(0, i32::try_from(items.len()).unwrap() - 1);
        let list = OptionList::new(items)
            .highlighted(highlighted)
            .rows(5).bar(true).thumb(true)
            .on_highlight(cx.map(Msg::Highlight))
            .on_selected(cx.map(Msg::Run));
        let pressable = Pressable::new(move |cx, focused| {
            let theme = Theme::of(cx);
            let style = if focused { &theme.primary } else { &theme.border };
            BoxWidget::new()
                .border(Border::all(style.clone()).kind(BorderType::Rounded))
                .child(list.clone())
                .into()
        }).keys(
            [
                ("up", Some(cx.send(Msg::Move(-1)))),
                ("down", Some(cx.send(Msg::Move(1)))),
                ("enter", Some(cx.send(Msg::Run(highlighted)))),
            ],
            [("up", "select"), ("down", "select"), ("enter", "run")],
        );
        column![
            pressable,
            Text::new(format!("ran: {}", self.ran.as_deref().unwrap_or("nothing"))),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Commands);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| marks of | none, o, sa | none |
| rows | 3, 5, 8 | 5 |
| details | false, true | true |
| bar | false, true | true |
| thumb | false, true | true |

## ModalBarrier

What dims the app under a dialog and keeps the mouse from it.

- `rtwk::overlay::ModalBarrier` (struct) in `crates/rtwk-overlay/src/barrier.rs:72`: Covers the room it gets and dims what is under it: each cell keeps its glyph and its attributes, its background, its text color and the color of its underline taken opacity of the way to color, channel by channel and rounded once (`a + (color - a) * opacity`; the terminal's default colors where a cell has them); where they are none, the theme's shade and its alpha (ThemeData::shade, ThemeData::shade_alpha: black at half on a dark theme, at 28 % on a light one), not rounded to 255ths on the way.

### Code

```rust
struct Dimmed;

struct DimmedState {
    pressed: usize,
    dismissed: usize,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Press,
    Dismiss,
}

impl StatefulWidget for Dimmed {
    type State = DimmedState;
    fn create_state(&self) -> DimmedState {
        DimmedState { pressed: 0, dismissed: 0 }
    }
}

impl State for DimmedState {
    type Widget = Dimmed;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Dimmed, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Press => self.pressed += 1,
            Msg::Dismiss => self.dismissed += 1,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Dimmed, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let body = column![
            Text::new("The text under the barrier, dimmed by it: the mouse does not reach it."),
            Button::new("Press me").on_pressed(cx.send(Msg::Press)),
            Text::rich([
                Span::styled("pressed ", theme.muted.clone()),
                Span::new(self.pressed.to_string()),
                Span::styled(", the barrier ", theme.muted.clone()),
                Span::new(self.dismissed.to_string()),
            ]),
        ].cross_axis_alignment(CrossAxisAlignment::Start).spacing(1);
        let stack = Stack::new().fit(StackFit::Expand).child(body);
        let stack = stack.child(ModalBarrier::new()
            .on_dismiss(cx.send(Msg::Dismiss)));
        let front = Center::new(Panel::new(Text::new("over it"))
            .padding(EdgeInsets::x(1))
            .shadow(false));
        SizedBox::new(Some(40), Some(8), Some(stack.child(front).into()))
        .into()
    }
}
impl_widget!(stateful Dimmed);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| barrier | false, true | true |
| alpha | ¼, ½, ¾ | ½ |
| fade | false, true | false |

## WindowLayer

Windows over the app: a title bar drags one, an edge resizes it, its buttons minimise, maximise and close it; Tab to a title bar, then the arrows move it.

- `rtwk::overlay::WindowLayer` (struct) in `crates/rtwk-overlay/src/window/mod.rs:569`: Shows windows over child, each in a frame (frame) with its title on top, the title bar; laid out as `child` is, as big as it is.

### Code

```rust
struct Desk;

struct DeskState {
    // Keep the two controllers in state, even while a window is closed.
    notes: WindowController,
    counter: WindowController,
    closed: [bool; 2],
    count: usize,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Close(usize),
    Count,
    Reopen,
}

impl StatefulWidget for Desk {
    type State = DeskState;
    fn create_state(&self) -> DeskState {
        DeskState {
            notes: WindowController::new().placed(2, 3, 30, 6),
            counter: WindowController::new().placed(24, 5, 26, 6),
            closed: [false; 2],
            count: 0,
        }
    }
}

impl State for DeskState {
    type Widget = Desk;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Desk, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // Close marks a window closed, Count increments the counter,
            // and Reopen clears the closed flags without resetting either controller.
            Msg::Close(index) => self.closed[index] = true,
            Msg::Count => self.count += 1,
            Msg::Reopen => self.closed.fill(false),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Desk, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let window = |title: &'static str, index, controller, child: AnyWidget| {
            Window::new(title, Padding::new(EdgeInsets::x(1), child))
                .key(title)
                .controller(controller)
                .on_close(cx.send(Msg::Close(index)))
                .resizable(true)
                .minimisable(true)
                .maximisable(true)
                .shadow(Panel::shadow_of(&theme))
        };
        let mut windows = Vec::new();
        if !self.closed[0] {
            windows.push(window("Notes", 0, self.notes.clone(), Text::new("Drag me by the title bar, or resize me by an edge.").into()));
        }
        if !self.closed[1] {
            windows.push(window("Counter", 1, self.counter.clone(), column![
                Text::new(format!("count: {}", self.count)),
                Button::new("Add one").on_pressed(cx.send(Msg::Count)),
            ].cross_axis_alignment(CrossAxisAlignment::Start).into()));
        }
        let mut reopen = Button::new("Open the windows closed");
        if self.closed.iter().any(|closed| *closed) {
            reopen = reopen.on_pressed(cx.send(Msg::Reopen));
        }
        let layer = WindowLayer::new(windows, column![
            Text::new("The app, under its windows."),
            reopen,
        ].cross_axis_alignment(CrossAxisAlignment::Start)).frame(WindowFrame::Raised);
        SizedBox::new(Some(50), Some(12), Some(layer.into()))
        .into()
    }
}
impl_widget!(stateful Desk);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| frame | raised, bordered | raised |
| shadow | false, true | true |
| resizable | false, true | true |
| minimisable | false, true | true |
| maximisable | false, true | true |
