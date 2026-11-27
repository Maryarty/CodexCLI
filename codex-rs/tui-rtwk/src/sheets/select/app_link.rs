//! The app link dialog of `/apps` (oracle `bottom_pane/app_link_view.rs`): picking an app opens
//! it. The link screen says what the app is and offers the page of the app (`Manage on ChatGPT`,
//! `Install on ChatGPT`), the switch of an installed app (`Disable app`, `Enable app`) and
//! `Back`; opening the page of an app that is not installed goes on to the confirmation screen
//! (`Finish App Setup`: the URL, and `I already Installed it`, which reads the directory again).
//! In Flutter it is the `AlertDialog` of a `Navigator` route whose `State` keeps which of the two
//! screens is shown, the cursor of its actions and its own copy of the switch (the oracle's view
//! keeps its own too: `toggle_enabled`); what an action asks of the app is a verb to the model
//! (ADR-013: `OpenAppPage`, `SetApp`, `RefreshApps`), and the dialog closes the sheet itself.
//!
//! The same dialog is the oracle's view of a server's request (`suggest.rs`: a tool suggestion to
//! install or enable an app, a sign-in, a URL to visit). Its target is then the request the queue
//! holds, its actions ask `approvals::Intent` verbs (`OpenLink`, `LinkDone`, `LinkEnable`, a
//! decline), and it closes with the request: the request dialog around it watches the queue.

use std::rc::Rc;

use rtwk::focus::{Focus, FocusNode, KeyBindings};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox,
};
use rtwk::overlay::Dialog;
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{
    LIST_ACCEPT, LIST_CANCEL, LIST_MOVE_DOWN, LIST_MOVE_LEFT, LIST_MOVE_RIGHT, LIST_MOVE_UP,
};
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::{self, Decision};
use codex_tui_core::screens::select::apps::{Link, ModalAction, ModalChoice};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::{bind_fixed, bindings, first_of_each, fixed_presses, keymap, label};
use crate::widgets::options::{OptionStyles, Options, RichRow};

pub mod strings;
mod suggest;

pub use suggest::Target;
use suggest::{Action, Screen};

#[derive(Clone, Debug, PartialEq)]
pub struct AppLinkView {
    /// What the dialog was opened for: an app's page, or a request of a server.
    pub target: Target,
}

impl AppLinkView {
    /// The dialog of the app picked in `/apps`.
    pub fn apps(link: Link) -> AppLinkView {
        AppLinkView {
            target: Target::Apps(link),
        }
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
            enabled: self.target.enabled(),
            options: Options::new(),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq AppLinkView);

/// Says a verb of the request to the model.
fn ask(cx: &mut Cx<'_, AppLinkState>, intent: approvals::Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Approvals(intent));
}

/// Ends the dialog: `/apps` closes it, a request is declined and goes from the queue, and the
/// request dialog around the view closes with it.
fn close(cx: &mut Cx<'_, AppLinkState>, target: &Target) {
    match target {
        Target::Apps(_) => super::apps::finish(cx, None),
        Target::Request(key, _) => ask(
            cx,
            approvals::Intent::Answer {
                key: key.clone(),
                decision: Decision::Decline,
            },
        ),
    }
}

impl State for AppLinkState {
    type Widget = AppLinkView;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, w: &AppLinkView, cx: &mut Cx<'_, Self>) -> Rebuild {
        let flavor = w.target.flavor();
        let rows = flavor.actions(self.screen, w.target.installed());
        let row = match msg {
            Msg::Move(delta) => {
                let last = i64::try_from(rows.len().saturating_sub(1)).unwrap_or(0);
                let from = i64::try_from(self.options.cursor()).unwrap_or(0);
                self.options
                    .set(usize::try_from((from + delta).clamp(0, last)).unwrap_or(0));
                return Rebuild::No;
            }
            Msg::Cancel => {
                close(cx, &w.target);
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
                match &w.target {
                    Target::Apps(link) => super::apps::act(cx, ModalAction::Visit(link.id.clone())),
                    Target::Request(key, _) => {
                        ask(cx, approvals::Intent::OpenLink { key: key.clone() });
                    }
                }
                // An installed app's page is its management page: the dialog stays.
                if !flavor.goes_on(w.target.installed()) {
                    return Rebuild::No;
                }
                self.screen = Screen::Confirm;
                self.options.set(0);
                Rebuild::Yes
            }
            Action::Switch => match &w.target {
                Target::Apps(link) => {
                    self.enabled = !self.enabled;
                    super::apps::act(
                        cx,
                        ModalAction::Enable {
                            id: link.id.clone(),
                            enabled: self.enabled,
                        },
                    );
                    Rebuild::Yes
                }
                // `toggle_enabled` of a suggestion: the switch is written and the request accepted.
                Target::Request(key, _) => {
                    ask(cx, approvals::Intent::LinkEnable { key: key.clone() });
                    Rebuild::No
                }
            },
            Action::Close => {
                close(cx, &w.target);
                Rebuild::No
            }
            // `complete_external_flow_and_close`: the page was visited, the directory is read
            // again (when the apps are what it was about) and the dialog closes.
            Action::Done => {
                match &w.target {
                    Target::Apps(_) => {
                        super::apps::finish(cx, Some(ModalChoice::RefreshAfterVisit));
                    }
                    Target::Request(key, _) => {
                        ask(cx, approvals::Intent::LinkDone { key: key.clone() })
                    }
                }
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
        let target = &w.target;
        let flavor = target.flavor();
        let installed = target.installed();
        let url = target.url().to_owned();
        let rows = flavor.actions(self.screen, installed);

        let text = |text: &str, style: Style| -> AnyWidget {
            Text::new(text.to_owned())
                .style(style)
                .wrap(TextWrap::Word)
                .into()
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
                content = content.child(text(target.title(words), tokens.bold()));
                if let Some(description) = target
                    .description(words)
                    .filter(|description| !description.trim().is_empty())
                {
                    content = content.child(text(description.trim(), tokens.dim()));
                }
                content = content.child(gap());
                if let Some(reason) = target.reason().filter(|reason| !reason.trim().is_empty()) {
                    content = content
                        .child(text(reason.trim(), Style::new().attrs(Attrs::ITALIC)))
                        .child(gap());
                }
                if installed && !flavor.browser() {
                    content = content
                        .child(text(words.use_mention, Style::new()))
                        .child(gap());
                }
                if flavor.browser() {
                    content = content
                        .child(text(words.url_label, tokens.dim()))
                        .child(
                            Text::rich(vec![Span::new(url.clone()).link(url)]).wrap(TextWrap::Word),
                        )
                        .child(gap());
                }
                let instructions = flavor.instructions(installed, words);
                if !instructions.trim().is_empty() {
                    content = content.child(text(instructions.trim(), Style::new()));
                    if !flavor.browser() {
                        content = content.child(text(words.appears_later, Style::new()));
                        if !installed {
                            content = content.child(text(words.use_after_install, Style::new()));
                        }
                    }
                    content = content.child(gap());
                }
            }
            Screen::Confirm => {
                let finish = flavor.finish(words);
                content = content
                    .child(text(finish.title, tokens.bold()))
                    .child(gap())
                    .child(text(finish.open, Style::new()))
                    .child(text(finish.then, Style::new()))
                    .child(gap())
                    .child(text(finish.url, tokens.dim()))
                    .child(
                        Text::rich(vec![
                            Span::styled(url.clone(), tokens.accent())
                                .underline()
                                .link(url),
                        ])
                        // a URL is not broken at a hyphen or a slash: it runs on to the line's end
                        // (the oracle's pane does not split a URL-like token either)
                        .wrap(TextWrap::Grapheme),
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
                label: vec![Span::new(
                    flavor
                        .label(*action, installed, self.enabled, words)
                        .to_owned(),
                )],
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

        let focus = Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .focus_node(self.node.clone())
        .autofocus();
        let focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Cancel));
        let focus = bind_fixed(focus, fixed::PICK_PREVIOUS, cx.send(Msg::Move(-1)));
        let focus = bind_fixed(focus, fixed::PICK_NEXT, cx.send(Msg::Move(1)));
        let focus = bind_fixed(focus, fixed::STEP_PREVIOUS, cx.send(Msg::Move(-1)));
        let focus = bind_fixed(focus, fixed::STEP_NEXT, cx.send(Msg::Move(1)));
        let focus = bind_fixed(focus, fixed::ENTER, cx.send(Msg::Enter));
        // A digit answers the row of its place.
        let focus = fixed_presses(fixed::PICK_NUMBERS)
            .into_iter()
            .take(rows.len())
            .enumerate()
            .fold(focus, |focus, (at, key)| {
                focus.bind(key, cx.send(Msg::Pick(at)))
            });
        // The keymap's own list keys (a remapped one moves or answers too).
        let mut named = Vec::new();
        named.extend(bindings(&keys, LIST_CANCEL, &cx.send(Msg::Cancel)));
        named.extend(bindings(&keys, LIST_MOVE_UP, &cx.send(Msg::Move(-1))));
        named.extend(bindings(&keys, LIST_MOVE_LEFT, &cx.send(Msg::Move(-1))));
        // A terminal sends Ctrl+H as the C0 byte, which arrives as Backspace holding Ctrl: the
        // keymap's `ctrl-h` (the oracle reads control characters so).
        if bindings(&keys, LIST_MOVE_LEFT, &())
            .iter()
            .any(|(name, ())| name == "ctrl+h")
        {
            named.push(("ctrl+backspace".to_owned(), cx.send(Msg::Move(-1))));
        }
        named.extend(bindings(&keys, LIST_MOVE_DOWN, &cx.send(Msg::Move(1))));
        named.extend(bindings(&keys, LIST_MOVE_RIGHT, &cx.send(Msg::Move(1))));
        named.extend(bindings(&keys, LIST_ACCEPT, &cx.send(Msg::Enter)));
        KeyBindings::new(first_of_each(named), focus).into()
    }
}
