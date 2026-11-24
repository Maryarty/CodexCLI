//! The dialog of a presented request (S13): the question, the header, the
//! options and the hint, drawn from `present::body` of the prompt the model
//! holds for the request. Enter, the digits and each option's shortcut answer
//! it; Esc cancels (the oracle's `cancel_current_request`); `o` opens the
//! thread a request of another thread came from. A request whose prompt the
//! model does not hold gets the skeleton's dismiss dialog.

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::style::{Attrs, Span, Style, TerminalBrightness};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::app::RequestQueue;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::{Decision, Intent, Prompt};

use super::clip::{Block, Pane};
use super::dismiss::DismissDialog;
use super::form::FormView;
use super::present::{self, Body, Line, Piece, Tone};
use super::strings;
use super::user_input::UserInputView;
use crate::app::theme;
use crate::app::view::Hs;
use crate::keymap::bind_fixed;
use crate::widgets::options::{OptionStyles, Options, RichRow};

#[derive(Clone, Debug, PartialEq)]
pub struct RequestDialog {
    pub key: RequestKey,
}

pub struct RequestDialogState {
    options: Options,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Answer(usize),
    Move(i64),
    Enter,
    Cancel,
    OpenThread,
}

impl StatefulWidget for RequestDialog {
    type State = RequestDialogState;

    fn create_state(&self) -> RequestDialogState {
        RequestDialogState {
            options: Options::new(),
        }
    }
}

impl_widget!(stateful eq RequestDialog);

/// The prompt the model holds for `key`, read where it is drawn or answered.
fn prompt_of(key: &RequestKey, cx: &mut Cx<'_, RequestDialogState>) -> Option<Rc<Prompt>> {
    let key = key.clone();
    select::<RequestQueue, Option<Rc<Prompt>>, _>(cx, move |queue| {
        queue.get(&key).and_then(|entry| entry.prompt.clone())
    })
}

impl State for RequestDialogState {
    type Widget = RequestDialog;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn did_change_dependencies(&mut self, w: &RequestDialog, cx: &mut Cx<'_, Self>) {
        let key = w.key.clone();
        let queued = select::<RequestQueue, bool, _>(cx, move |queue| queue.contains(&key));
        if !queued {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn update(&mut self, msg: Msg, w: &RequestDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(prompt) = prompt_of(&w.key, cx) else {
            return Rebuild::No;
        };
        let s = strings::of(cx);
        // A prompt with a view of its own sends its messages to that view.
        let Some(body) = present::body(&prompt, s) else {
            return Rebuild::No;
        };
        let hs = read::<Hs, _>(cx);
        let decision = match msg {
            Msg::Move(delta) => {
                self.options.step(delta, body.options.len());
                return Rebuild::No;
            }
            Msg::OpenThread => {
                if body.open_thread {
                    cx.emit(&hs.open_thread, w.key.thread.clone());
                }
                return Rebuild::No;
            }
            Msg::Enter => body
                .options
                .get(self.options.cursor())
                .map(|option| option.decision.clone()),
            Msg::Answer(index) => body
                .options
                .get(index)
                .map(|option| option.decision.clone()),
            // The oracle's cancel: the list's cancel option.
            Msg::Cancel => body
                .options
                .iter()
                .map(|option| &option.decision)
                .find(|d| matches!(d, Decision::Cancel | Decision::DenyPermissions))
                .cloned(),
        };
        let Some(decision) = decision else {
            if msg == Msg::Cancel {
                cx.emit(&hs.dismiss, w.key.clone());
                close_dialog(cx, Some(()));
            }
            return Rebuild::No;
        };
        cx.emit(
            &hs.screen,
            ScreenIntent::Approvals(Intent::Answer {
                key: w.key.clone(),
                decision,
            }),
        );
        Rebuild::No
    }

    fn build(&self, w: &RequestDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(prompt) = prompt_of(&w.key, cx) else {
            return DismissDialog { key: w.key.clone() }.into();
        };
        // An elicitation whose schema the form takes is the form's dialog.
        if matches!(&*prompt, Prompt::Elicitation(elicitation) if elicitation.form.is_some()) {
            return FormView {
                key: w.key.clone(),
                prompt,
            }
            .into();
        }
        // The questions of a `request_user_input` are the dialog of their own.
        if matches!(&*prompt, Prompt::UserInput(_)) {
            return UserInputView {
                key: w.key.clone(),
                prompt,
            }
            .into();
        }
        let tokens = theme::Tokens::of(cx);
        let syntax = SyntaxTheme::of(cx);
        let s = strings::of(cx);
        let Some(body) = present::body(&prompt, s) else {
            return DismissDialog { key: w.key.clone() }.into();
        };
        let tone = |tone: Tone| match tone {
            Tone::Plain => Style::new(),
            Tone::Dim => tokens.dim(),
            Tone::Bold => tokens.bold(),
            Tone::Italic => Style::new().attrs(Attrs::ITALIC),
            Tone::Accent => tokens.accent(),
        };
        let spans = |pieces: &[Piece]| -> Vec<Span> {
            pieces
                .iter()
                .map(|piece| Span::styled(piece.text.clone(), tone(piece.tone)))
                .collect()
        };
        // The oracle's bottom pane: a blank row, the question in bold, the
        // header, the options (the cursor row in the accent colour, marked
        // `›`), a blank row and the hint, all two cells in.
        let flat = body.flat;
        let inset = |child: AnyWidget| if flat { child } else { inset(child) };
        // The header's row groups: the title, a blank row, then the lines the request has.
        let mut header: Vec<Block> = Vec::new();
        let blank = || Block {
            widget: Text::new(String::new()).into(),
            text: String::new(),
        };
        if let Some(title) = &body.title {
            header.push(Block {
                widget: inset(Text::new(title.clone()).style(tokens.bold()).into()),
                text: title.clone(),
            });
            header.push(blank());
        }
        for line in &body.header {
            header.push(match line {
                Line::Blank => blank(),
                Line::Pieces(pieces) => Block {
                    widget: inset(Text::rich(spans(pieces)).wrap(TextWrap::Word).into()),
                    text: pieces.iter().map(|piece| piece.text.as_str()).collect(),
                },
                Line::Command(command) => {
                    let code = Highlighter::new(language_named("bash"), command);
                    let mut lines = Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch);
                    for index in 0..code.line_count() {
                        let mut row = Vec::new();
                        if index == 0 {
                            row.push(Span::new(s.shell_prompt));
                        }
                        row.extend(
                            syntax
                                .spans(code.line(index), code.tokens(index))
                                .into_iter()
                                .map(|span| Span::styled(span.text, span.style)),
                        );
                        lines = lines.child(Text::rich(row).wrap(TextWrap::Word));
                    }
                    Block {
                        widget: inset(lines.into()),
                        text: format!("{}{command}", s.shell_prompt),
                    }
                }
            });
        }
        // The waiting view of a user verification has no list: Esc and the cancel key only.
        if flat {
            let mut pane = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            for block in header {
                pane = pane.child(block.widget);
            }
            let mut focus = bind_fixed(
                Focus::new(Dialog::bare(
                    BoxWidget::new().style(Style::new()).child(pane),
                ))
                .can_request_focus(false),
                fixed::DISMISS,
                cx.send(Msg::Cancel),
            );
            for option in body.options.iter().filter_map(|option| option.key) {
                focus = focus.bind(option, cx.send(Msg::Cancel));
            }
            return focus.into();
        }
        // `selection_style`: where the terminal told nothing of its background the cursor row is
        // the terminal's own bold reverse, else the accent.
        let known = TerminalBrightness::background_of(cx).is_some();
        let styles = OptionStyles {
            cursor: if known {
                tokens.accent().attrs(Attrs::BOLD)
            } else {
                Style::new().attrs(Attrs::BOLD | Attrs::REVERSE)
            },
            label: Style::new(),
            description: Style::new(),
        };
        // A tap moves the cursor, a tap on the cursor's row answers (ADR-011).
        let rows = self.options.on_accept(cx.map(Msg::Answer));
        let options = rows.view_rich(Rc::new(approval_rows(&body, s, &tokens)), styles, 0);
        let option_texts = body
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| match &option.key_label {
                Some(key) => format!(
                    "{}. {}{}{key}{}",
                    index + 1,
                    option.label,
                    s.key_open_paren,
                    s.key_close_paren
                ),
                None => format!("{}. {}", index + 1, option.label),
            })
            .collect();
        // The gap rows are the pane's, they yield to the header (`clip.rs`).
        header.truncate(header.len().saturating_sub(present::GAP));
        let pane = Pane {
            header,
            gap: present::GAP,
            options,
            option_texts,
            hint: inset(Text::rich(spans(&body.hint)).into()),
            hint_text: body.hint.iter().map(|piece| piece.text.as_str()).collect(),
            dim: tokens.dim(),
            strings: s,
        };
        let mut focus = Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(pane.view()),
        ))
        .can_request_focus(false);
        for (id, msg) in [
            (fixed::DISMISS, Msg::Cancel),
            (fixed::CONFIRM_PREVIOUS, Msg::Move(-1)),
            (fixed::CONFIRM_NEXT, Msg::Move(1)),
            (fixed::ENTER, Msg::Enter),
        ] {
            focus = bind_fixed(focus, id, cx.send(msg));
        }
        if body.open_thread
            && let Some(open) = s.key_open.chars().next()
        {
            focus = focus.bind(open, cx.send(Msg::OpenThread));
        }
        for (index, option) in body.options.iter().enumerate() {
            if let Some(key) = option.key {
                focus = focus.bind(key, cx.send(Msg::Answer(index)));
            }
            if let Some(digit) = char::from_digit(u32::try_from(index + 1).unwrap_or(0), 10) {
                focus = focus.bind(digit, cx.send(Msg::Answer(index)));
            }
        }
        focus.into()
    }
}

/// The approval's rows: the number, then the label with its shortcut after
/// it (`Yes, proceed (y)`, the key in bold).
fn approval_rows(body: &Body, s: &strings::Strings, tokens: &theme::Tokens) -> Vec<RichRow> {
    body.options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut label = vec![Span::new(option.label.clone())];
            if let Some(key) = &option.key_label {
                label.extend([
                    Span::new(s.key_open_paren),
                    Span::styled(key.clone(), tokens.bold()),
                    Span::new(s.key_close_paren),
                ]);
            }
            RichRow {
                prefix: format!("{}. ", index + 1),
                label,
                description: option.description.clone(),
            }
        })
        .collect()
}

fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}
