//! The app link dialog of `/apps` (oracle `bottom_pane/app_link_view.rs`): picking an app opens
//! it. The link screen says what the app is and offers the page of the app (`Manage on ChatGPT`,
//! `Install on ChatGPT`), the switch of an installed app (`Disable app`, `Enable app`) and
//! `Back`; opening the page of an app that is not installed goes on to the confirmation screen
//! (`Finish App Setup`: the URL, and `I already Installed it`, which reads the directory again).
//! In Flutter it is the `AlertDialog` of a `Navigator` route whose `State` keeps which of the two
//! screens is shown, the cursor of its actions and its own copy of the switch (the oracle's view
//! keeps its own too: `toggle_enabled`); what an action asks of the app is a verb to the model
//! (ADR-013: `OpenAppPage`, `SetApp`, `RefreshApps`), and the dialog closes the sheet itself.

use std::rc::Rc;

use rtwk::focus::{Focus, FocusNode, KeyBindings, KeyPress};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::read;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{
    LIST_ACCEPT, LIST_CANCEL, LIST_MOVE_DOWN, LIST_MOVE_LEFT, LIST_MOVE_RIGHT, LIST_MOVE_UP,
};
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::apps::Link;

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::{bindings, first_of_each, keymap, label};
use crate::widgets::options::{OptionStyles, Options, RichRow};

pub mod strings;

#[derive(Clone, Debug, PartialEq)]
pub struct AppLinkView {
    /// The app's page the dialog was opened for.
    pub link: Link,
}

/// Which screen of the dialog is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Link,
    Confirm,
}

/// What an action row of a screen does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Open the page of the app in the browser.
    Open,
    /// Switch the app on or off.
    Switch,
    /// Close the dialog.
    Close,
    /// The page was visited: read the directory again and close.
    Done,
    /// Back to the link screen.
    Back,
}

/// The actions of a screen, in order (the oracle's `action_labels`).
fn actions(screen: Screen, installed: bool) -> &'static [Action] {
    match (screen, installed) {
        (Screen::Link, true) => &[Action::Open, Action::Switch, Action::Close],
        (Screen::Link, false) => &[Action::Open, Action::Close],
        (Screen::Confirm, _) => &[Action::Done, Action::Back],
    }
}

pub struct AppLinkState {
    screen: Screen,
    /// The dialog's copy of the app's switch: it shows the change at once, the directory follows
    /// the saved write.
    enabled: bool,
    options: Options,
    /// The dialog's keys: its route takes the focus when it is pushed (or shown alone).
    node: FocusNode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The cursor goes a row on (1) or back (-1), stopping at the ends.
    Move(i64),
    /// The action under the cursor.
    Enter,
    /// The action of this row (a digit, a tap on the cursor's row).
    Pick(usize),
    /// Esc: close, whichever screen is shown.
    Cancel,
}

impl StatefulWidget for AppLinkView {
    type State = AppLinkState;

    fn create_state(&self) -> AppLinkState {
        AppLinkState {
            screen: Screen::Link,
            enabled: self.link.enabled,
            options: Options::new(),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq AppLinkView);

/// Says a verb to the model.
fn say(cx: &mut Cx<'_, AppLinkState>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Select(intent));
}

impl State for AppLinkState {
    type Widget = AppLinkView;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, w: &AppLinkView, cx: &mut Cx<'_, Self>) -> Rebuild {
        let rows = actions(self.screen, w.link.installed);
        let row = match msg {
            Msg::Move(delta) => {
                let last = i64::try_from(rows.len().saturating_sub(1)).unwrap_or(0);
                let from = i64::try_from(self.options.cursor()).unwrap_or(0);
                self.options
                    .set(usize::try_from((from + delta).clamp(0, last)).unwrap_or(0));
                return Rebuild::No;
            }
            Msg::Cancel => {
                close_dialog::<_, ()>(cx, None);
                return Rebuild::No;
            }
            Msg::Enter => self.options.cursor(),
            Msg::Pick(row) => {
                if row >= rows.len() {
                    return Rebuild::No;
                }
                self.options.set(row);
                row
            }
        };
        let Some(action) = rows.get(row) else {
            return Rebuild::No;
        };
        match action {
            Action::Open => {
                say(cx, Intent::OpenAppPage(w.link.id.clone()));
                // An installed app's page is its management page: the dialog stays.
                if w.link.installed {
                    return Rebuild::No;
                }
                self.screen = Screen::Confirm;
                self.options.set(0);
                Rebuild::Yes
            }
            Action::Switch => {
                self.enabled = !self.enabled;
                say(
                    cx,
                    Intent::SetApp {
                        id: w.link.id.clone(),
                        enabled: self.enabled,
                    },
                );
                Rebuild::Yes
            }
            Action::Close => {
                close_dialog::<_, ()>(cx, None);
                Rebuild::No
            }
            // `complete_external_flow_and_close`: the page was visited, the directory is read
            // again and the dialog closes.
            Action::Done => {
                say(cx, Intent::RefreshApps);
                close_dialog::<_, ()>(cx, None);
                Rebuild::No
            }
            Action::Back => {
                self.screen = Screen::Link;
                self.options.set(0);
                Rebuild::Yes
            }
        }
    }

    fn build(&self, w: &AppLinkView, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let words = strings::of(cx);
        let keys = keymap(cx);
        let link = &w.link;
        let rows = actions(self.screen, link.installed);

        let text = |text: &'static str, style: Style| -> AnyWidget {
            Text::new(text).style(style).wrap(TextWrap::Word).into()
        };
        let gap = || -> AnyWidget { SizedBox::new(None, Some(1), None).into() };
        let sides = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::symmetric(2, 0), child).into()
        };

        // The content: what the screen says.
        let mut content = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        match self.screen {
            Screen::Link => {
                content = content.child(
                    Text::new(link.name.clone())
                        .style(tokens.bold())
                        .wrap(TextWrap::Word),
                );
                if let Some(description) = &link.description {
                    content = content.child(
                        Text::new(description.clone())
                            .style(tokens.dim())
                            .wrap(TextWrap::Word),
                    );
                }
                content = content.child(gap());
                if link.installed {
                    content = content
                        .child(text(words.use_mention, Style::new()))
                        .child(gap());
                }
                content = content
                    .child(text(
                        if link.installed {
                            words.instructions_manage
                        } else {
                            words.instructions_install
                        },
                        Style::new(),
                    ))
                    .child(text(words.appears_later, Style::new()));
                if !link.installed {
                    content = content.child(text(words.use_after_install, Style::new()));
                }
                content = content.child(gap());
            }
            Screen::Confirm => {
                content = content
                    .child(text(words.finish_title, tokens.bold()))
                    .child(gap())
                    .child(text(words.finish_open, Style::new()))
                    .child(text(words.finish_then, Style::new()))
                    .child(gap())
                    .child(text(words.setup_url, tokens.dim()))
                    .child(
                        Text::rich(vec![
                            Span::styled(link.url.clone(), tokens.accent())
                                .underline()
                                .link(link.url.clone()),
                        ])
                        .wrap(TextWrap::Word),
                    );
            }
        }

        // The actions: `› 1. Manage on ChatGPT`, a tap on a row moves the cursor to it and a tap
        // on the cursor's row does it (ADR-011).
        let labels: Vec<RichRow> = rows
            .iter()
            .enumerate()
            .map(|(index, action)| RichRow {
                prefix: format!("{}. ", index + 1),
                label: vec![Span::new(match action {
                    Action::Open if link.installed => words.manage,
                    Action::Open => words.install,
                    Action::Switch if self.enabled => words.disable,
                    Action::Switch => words.enable,
                    Action::Close | Action::Back => words.back,
                    Action::Done => words.installed_it,
                })],
                description: None,
            })
            .collect();
        let styles = OptionStyles {
            cursor: tokens.selected(),
            label: Style::new(),
            description: Style::new(),
        };
        let options =
            self.options
                .on_accept(cx.map(Msg::Pick))
                .view_rich(Rc::new(labels), styles, 0);

        // `tab / ↑ ↓ move · enter select · esc close`: the keys the keymap names.
        let mut hint = String::from(words.hint_tab);
        if let Some(up) = label(&keys, LIST_MOVE_UP) {
            hint.push_str(words.hint_or);
            hint.push_str(&up);
        }
        if let Some(down) = label(&keys, LIST_MOVE_DOWN) {
            hint.push_str(words.hint_space);
            hint.push_str(&down);
        }
        hint.push_str(words.hint_move);
        if let Some(accept) = label(&keys, LIST_ACCEPT) {
            hint.push_str(words.hint_join);
            hint.push_str(&accept);
            hint.push_str(words.hint_select);
        }
        if let Some(cancel) = label(&keys, LIST_CANCEL) {
            hint.push_str(words.hint_join);
            hint.push_str(&cancel);
            hint.push_str(words.hint_close);
        }

        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(gap())
            .child(sides(content.into()))
            .child(gap())
            .child(options)
            .child(Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::new(hint).style(tokens.dim()).wrap(TextWrap::Word),
            ));

        let mut focus = Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .focus_node(self.node.clone())
        .autofocus()
        .bind(KeyPress::ESC, cx.send(Msg::Cancel))
        .bind(KeyPress::UP, cx.send(Msg::Move(-1)))
        .bind(KeyPress::DOWN, cx.send(Msg::Move(1)))
        .bind(KeyPress::TAB, cx.send(Msg::Move(1)))
        .bind(KeyPress::BACKTAB, cx.send(Msg::Move(-1)))
        .bind('k', cx.send(Msg::Move(-1)))
        .bind('j', cx.send(Msg::Move(1)))
        .bind(KeyPress::ENTER, cx.send(Msg::Enter));
        for index in 0..rows.len() {
            if let Some(digit) = char::from_digit(u32::try_from(index + 1).unwrap_or(0), 10) {
                focus = focus.bind(digit, cx.send(Msg::Pick(index)));
            }
        }
        // The keymap's own list keys (a remapped one moves or answers too).
        let mut named = Vec::new();
        named.extend(bindings(&keys, LIST_CANCEL, &cx.send(Msg::Cancel)));
        named.extend(bindings(&keys, LIST_MOVE_UP, &cx.send(Msg::Move(-1))));
        named.extend(bindings(&keys, LIST_MOVE_LEFT, &cx.send(Msg::Move(-1))));
        named.extend(bindings(&keys, LIST_MOVE_DOWN, &cx.send(Msg::Move(1))));
        named.extend(bindings(&keys, LIST_MOVE_RIGHT, &cx.send(Msg::Move(1))));
        named.extend(bindings(&keys, LIST_ACCEPT, &cx.send(Msg::Enter)));
        KeyBindings::new(first_of_each(named), focus).into()
    }
}
