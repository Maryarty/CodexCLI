//! The startup prompts' panel (S05): one widget for every choice prompt of
//! the oracle's startup (cwd, model migration, hooks review, unarchive,
//! provider picker, daemon recovery).
//!
//! In Flutter: a `Column(min)` on the menu surface: a blank row, the prompt's
//! own lines, the choices (the shared `Options` controller: cursor and
//! scroll, the room the rest leaves), the key hint and a blank row; the keys
//! are a `Focus` with the bindings of `keys.rs` (key -> `Cmd`), answered by
//! core's `Prompt::{step, number, take}` (what a key does is the oracle's,
//! tested in `keys.rs` and core). The prompt is read from the model (`ScreenStates.startup.asked`);
//! the widget keeps the cursor only (ADR-003).

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::layout::{
    Align, Alignment, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, FlexFit, Flexible,
    MainAxisSize, Padding, SizedBox,
};
use rtwk::navigation::Navigator;
use rtwk::provider::{read, select};
use rtwk::scroll::{NeverScrollableScrollPhysics, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::MarkdownTheme;

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::startup::{Action, Answer, Intent, Prompt, Reaction, Status, Trust};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::l10n::strings as shared;
use crate::pages::startup::onboarding::GovCloudGuidance;
use crate::widgets::options::{OptionStyles, Options, RichRow};

use super::keys::{self, Cmd};
use super::strings::{self, Strings};

/// The panel over the prompt the model has open.
///
/// `embedded`: the body of the startup page, which asks the launch's
/// prompts one after the other and leaves when they are answered; else a
/// route of its own, popped when its prompt is answered.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Panel {
    pub embedded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    Cmd(Cmd),
}

/// Widget state: the cursor and the scroll of the choices (R14).
pub struct PanelState {
    options: Options,
    /// The prompt the cursor was placed for, by the number of answers before
    /// it: a new prompt (after an answer) places it again.
    placed: Option<usize>,
}

impl StatefulWidget for Panel {
    type State = PanelState;

    fn create_state(&self) -> PanelState {
        PanelState {
            options: Options::new(),
            placed: None,
        }
    }
}

impl_widget!(stateful eq Panel);

fn open(cx: &mut Cx<'_, PanelState>) -> Option<Prompt> {
    select::<ScreenStates, Option<Prompt>, _>(cx, |states| states.startup.asked.clone())
}

impl State for PanelState {
    type Widget = Panel;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &Panel, cx: &mut Cx<'_, Self>) {
        let Some(prompt) = open(cx) else {
            return;
        };
        // The same prompt with a new trust state (the hooks review writing)
        // keeps its cursor; another prompt starts on its first row.
        let answered = read::<ScreenStates, _>(cx).startup.answers.len();
        if self.placed != Some(answered) {
            self.options.set(prompt.initial_cursor());
        }
        self.placed = Some(answered);
    }

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, w: &Panel, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Cmd(cmd) = msg;
        let Some(prompt) = open(cx) else {
            return Rebuild::No;
        };
        match cmd.react(&prompt, self.options.cursor()) {
            Reaction::Nothing => Rebuild::No,
            Reaction::Cursor(row) => {
                self.options.set(row);
                // The provider picker's description follows the cursor.
                Rebuild::Yes
            }
            Reaction::Answer(answer) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Startup(Intent::Answer(answer)));
                if !w.embedded {
                    Navigator::maybe_pop(cx);
                }
                Rebuild::No
            }
            Reaction::TrustAll => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Startup(Intent::TrustHooks));
                // The oracle builds the "trusting" view afresh: cursor on top.
                self.options.set(0);
                Rebuild::Yes
            }
        }
    }

    fn build(&self, _: &Panel, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(prompt) = open(cx) else {
            return SizedBox::new(None, Some(0), None).into();
        };
        // The GovCloud guidance is a view of its own (focus, scroll, the
        // acknowledge row): Enter or a tap on the row answers it.
        if matches!(prompt, Prompt::GovCloud) {
            return GovCloudGuidance {
                acknowledge: cx.map(|()| Msg::Cmd(Cmd::Answer(Answer::GovCloud))),
            }
            .into();
        }
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let tokens = Tokens::of(cx);
        let warning = Theme::of(cx).warning.clone();
        let ink = Ink {
            dim: tokens.dim(),
            bold: tokens.bold(),
            accent: tokens.accent(),
            success: tokens.success(),
            warning,
            error: tokens.error(),
            markdown: tokens.transcript_markdown(cx),
        };
        let parts = Parts::of(&prompt, strings, &num, &ink, self.options.cursor());
        let rows: Rc<Vec<RichRow>> = Rc::new(parts.rows);
        let picker = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(
                self.options
                    .arrow(true, strings.more_above, ink.accent.clone()),
            )
            .child(
                // A tap on the row the cursor is on (or a double tap) is
                // Enter's action at that row (`Cmd::Take`); a tap that moves
                // the cursor tells the panel, as the arrows do (`Cmd::Cursor`).
                self.options
                    .on_accept(cx.map(|row: usize| Msg::Cmd(Cmd::Take(row))))
                    .on_move(cx.map(|row: usize| Msg::Cmd(Cmd::Cursor(row))))
                    .view_rich(
                        rows,
                        OptionStyles {
                            cursor: tokens.selected(),
                            label: Style::new(),
                            description: Style::new(),
                        },
                        0,
                    ),
            )
            .child(self.options.arrow(false, strings.more_below, ink.accent));
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let give_way = |widget: AnyWidget| -> AnyWidget {
            Flexible {
                fit: FlexFit::Loose,
                ..Flexible::new(1, widget)
            }
            .into()
        };
        let squeezes = parts.squeeze != Give::Fixed;
        // The notes shelf gives its blank rows way too; the body shelf keeps
        // the leading blank inside the body (clipped first) and the trailing
        // one fixed.
        let edge = |column: Column| {
            column.child(if parts.squeeze == Give::Notes {
                give_way(blank())
            } else {
                blank()
            })
        };
        if parts.squeeze != Give::Body {
            column = edge(column);
        }
        if parts.squeeze == Give::Body {
            let mut body = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .overflow_intended(true)
                .child(blank());
            for top in parts.top {
                body = body.child(top);
            }
            column = column.child(give_way(body.into()));
        } else {
            for (at, top) in parts.top.into_iter().enumerate() {
                column = column.child(if parts.squeeze == Give::Notes && at > 0 {
                    give_way(top)
                } else {
                    top
                });
            }
        }
        if parts.gap {
            column = column.child(blank());
        }
        if parts.has_rows {
            column = column.child(if squeezes {
                AnyWidget::from(picker)
            } else {
                Flexible {
                    fit: FlexFit::Loose,
                    ..Flexible::new(1, picker)
                }
                .into()
            });
        }
        for after in parts.after {
            column = column.child(if parts.squeeze == Give::Notes {
                give_way(after)
            } else {
                after
            });
        }
        if let Some(hint) = parts.hint {
            column = column.child(inset(hint));
        }
        column = edge(column);
        let panel = BoxWidget::new()
            .style(tokens.prompt_surface(cx, false))
            .child(column);
        let mut focus = Focus::new(Align::new(Alignment::TOP_LEFT, panel)).autofocus();
        for (press, cmd) in keys::bindings(&prompt) {
            focus = focus.bind(press, cx.send(Msg::Cmd(cmd)));
        }
        focus.into()
    }
}

/// The styles the lines are drawn with.
struct Ink {
    dim: Style,
    bold: Style,
    accent: Style,
    success: Style,
    warning: Style,
    error: Style,
    markdown: MarkdownTheme,
}

/// What gives way when the viewport is short (the oracle's `FlexRenderable`
/// shelves): nothing, the notes one by one (OSS), or the whole body above the
/// choices (the migration prompt), clipped.
#[derive(Clone, Copy, PartialEq)]
enum Give {
    Fixed,
    Notes,
    Body,
}

/// What a prompt is made of, in column order: the lines above the choices,
/// the choices, the lines under them, the hint.
struct Parts {
    top: Vec<AnyWidget>,
    /// The list shelf's extra gap row (the oracle's `ListSelectionView`
    /// header gap).
    gap: bool,
    has_rows: bool,
    rows: Vec<RichRow>,
    after: Vec<AnyWidget>,
    hint: Option<AnyWidget>,
    /// The oracle's `FlexRenderable` shelf (OSS): the notes and the blanks
    /// give way (flex 1) when the viewport is short, the title, the rows and
    /// the hint never do.
    squeeze: Give,
}

fn blank() -> AnyWidget {
    SizedBox::new(None, Some(1), None).into()
}

fn inset(child: AnyWidget) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}

fn line(text: impl Into<String>, style: &Style) -> AnyWidget {
    inset(Text::new(text.into()).style(style.clone()).into())
}

fn hint(spans: Vec<Span>) -> AnyWidget {
    Text::rich(spans).into()
}

/// A key hint: the keys plain, the words between dim (`enter`, ` continue ·
/// `, `esc`, ...: the odd entries are keys).
fn key_hint(words: &[&'static str], dim: &Style) -> AnyWidget {
    hint(
        words
            .iter()
            .enumerate()
            .map(|(at, word)| {
                if at % 2 == 0 {
                    Span::new(*word)
                } else {
                    Span::styled(*word, dim.clone())
                }
            })
            .collect(),
    )
}

fn action_index(action: Action) -> usize {
    match action {
        Action::Resume => 0,
        Action::Fork => 1,
    }
}

impl Parts {
    fn of(
        prompt: &Prompt,
        s: &'static Strings,
        num: &rtwk::l10n::NumSyms,
        ink: &Ink,
        cursor: usize,
    ) -> Parts {
        let numbered = |labels: Vec<String>| -> Vec<RichRow> {
            labels
                .into_iter()
                .enumerate()
                .map(|(at, label)| RichRow {
                    prefix: s.numbered(at + 1, "", num),
                    label: vec![Span::new(label)],
                    description: None,
                })
                .collect()
        };
        match prompt {
            // Drawn by `GovCloudGuidance` before the parts are asked for.
            Prompt::GovCloud => Parts {
                top: Vec::new(),
                gap: false,
                has_rows: false,
                rows: Vec::new(),
                after: Vec::new(),
                hint: None,
                squeeze: Give::Fixed,
            },
            Prompt::Cwd(p) => {
                let at = action_index(p.action);
                let mut labels = vec![
                    s.cwd_use_session(&p.session, num),
                    s.cwd_use_current(&p.current, num),
                    s.cwd_always_session.to_owned(),
                ];
                if p.allow_remember_current {
                    labels.push(if p.shows_remembered_current() {
                        s.cwd_always_current_in(&p.remembered_current, num)
                    } else {
                        s.cwd_always_current.to_owned()
                    });
                }
                Parts {
                    top: vec![
                        line(s.cwd_title(s.cwd_verbs[at], num), &ink.bold),
                        line(s.cwd_session_note(s.cwd_past[at], num), &ink.dim),
                        line(s.cwd_current_note, &ink.dim),
                    ],
                    gap: false,
                    has_rows: true,
                    rows: numbered(labels),
                    after: Vec::new(),
                    hint: Some(key_hint(&s.cwd_hint, &ink.dim)),
                    squeeze: Give::Fixed,
                }
            }
            Prompt::Migration(p) => {
                let mut top = Vec::new();
                if let Some(markdown) = p.filled_markdown() {
                    // The body is clipped where the viewport is short (the
                    // oracle's paragraph is): Flutter's `SingleChildScrollView`
                    // that never scrolls and takes no focus lays the Markdown
                    // out whole under a bounded column.
                    top.push(inset(
                        SingleChildScrollView::new(crate::widgets::markdown::source(
                            markdown,
                            ink.markdown.clone(),
                        ))
                        .physics(Rc::new(NeverScrollableScrollPhysics))
                        .can_request_focus(false)
                        .into(),
                    ));
                } else {
                    top.push(line(
                        s.migration_heading(&p.target_display_name, num),
                        &ink.bold,
                    ));
                    top.push(blank());
                    let description = p
                        .copy
                        .clone()
                        .or_else(|| p.target_description.clone().filter(|text| !text.is_empty()));
                    if p.copy.is_none() {
                        top.push(line(
                            s.migration_recommend(&p.current_model, &p.target_model, num),
                            &Style::new(),
                        ));
                        top.push(blank());
                    }
                    let description = description
                        .unwrap_or_else(|| s.migration_fallback(&p.target_display_name, num));
                    match &p.link {
                        Some(link) => top.push(inset(
                            Text::rich(vec![
                                Span::new(s.migration_with_link(
                                    &description,
                                    &p.target_display_name,
                                    num,
                                )),
                                Span::styled(
                                    link.clone(),
                                    ink.accent.clone().attrs(Attrs::UNDERLINE),
                                ),
                            ])
                            .into(),
                        )),
                        None => top.push(line(description, &Style::new())),
                    }
                    top.push(blank());
                    if p.can_opt_out {
                        top.push(line(s.migration_keep(&p.current_model, num), &Style::new()));
                    }
                }
                let tail = if p.can_opt_out {
                    s.migration_confirm
                } else {
                    s.migration_continue
                };
                Parts {
                    top,
                    gap: false,
                    has_rows: p.can_opt_out,
                    rows: numbered(
                        s.migration_choices
                            .iter()
                            .map(|c| (*c).to_owned())
                            .collect(),
                    ),
                    after: Vec::new(),
                    hint: Some(key_hint(
                        &[s.migration_keys, tail, s.migration_ctrl_c, s.migration_quit],
                        &ink.dim,
                    )),
                    squeeze: Give::Body,
                }
            }
            Prompt::Hooks(p) => {
                let mut top = vec![
                    line(s.hooks_title, &ink.bold),
                    line(s.hooks_count(p.count, num), &ink.warning),
                    line(s.hooks_sandbox, &ink.dim),
                ];
                match &p.trust {
                    Trust::Failed(detail) => {
                        top.push(line(s.hooks_failed(detail, num), &ink.error))
                    }
                    Trust::Trusting(_) => top.push(line(s.hooks_trusting, &ink.dim)),
                    Trust::Idle => {}
                }
                let trusting = matches!(p.trust, Trust::Trusting(_));
                let rows = s
                    .hooks_choices
                    .iter()
                    .enumerate()
                    .map(|(at, label)| RichRow {
                        prefix: if trusting {
                            "   ".to_owned()
                        } else {
                            s.numbered(at + 1, "", num)
                        },
                        label: vec![Span::new(*label)],
                        description: None,
                    })
                    .collect();
                Parts {
                    top,
                    gap: true,
                    has_rows: true,
                    rows,
                    after: Vec::new(),
                    hint: (!trusting).then(|| key_hint(&s.hooks_hint, &ink.dim)),
                    squeeze: Give::Fixed,
                }
            }
            Prompt::Unarchive(p) => Parts {
                top: vec![
                    line(s.unarchive_title, &ink.bold),
                    line(p.thread.clone(), &ink.dim),
                ],
                gap: false,
                has_rows: true,
                rows: numbered(vec![
                    s.unarchive_choice(s.unarchive_verbs[action_index(p.action)], num),
                    s.unarchive_cancel.to_owned(),
                ]),
                after: Vec::new(),
                hint: Some(key_hint(&s.unarchive_hint, &ink.dim)),
                squeeze: Give::Fixed,
            },
            Prompt::Oss(p) => {
                // The oracle: the first letter of the provider underlined (its
                // key), then ` · ` dim and the status in green, red or yellow.
                let label = |at: usize, status: Status| {
                    let (state, colour) = match status {
                        Status::Running => (0, &ink.success),
                        Status::NotRunning => (1, &ink.error),
                        Status::Unknown => (2, &ink.warning),
                    };
                    let name = s.oss_providers[at];
                    let first = name.chars().next().map_or(0, char::len_utf8);
                    RichRow {
                        prefix: s.numbered(at + 1, "", num),
                        label: vec![
                            Span::styled(&name[..first], Style::new().attrs(Attrs::UNDERLINE)),
                            Span::new(&name[first..]),
                            Span::styled(s.oss_separator, ink.dim.clone()),
                            Span::styled(s.oss_statuses[state], colour.clone()),
                        ],
                        description: None,
                    }
                };
                Parts {
                    top: vec![line(s.oss_title, &ink.bold), line(s.oss_subtitle, &ink.dim)],
                    gap: false,
                    has_rows: true,
                    rows: vec![label(0, p.lmstudio), label(1, p.ollama)],
                    after: vec![line(s.oss_descriptions[cursor.min(1)], &ink.dim), blank()],
                    hint: Some(key_hint(&s.oss_hint, &ink.dim)),
                    squeeze: Give::Notes,
                }
            }
            Prompt::Daemon(p) => {
                let mut top = vec![line(
                    if p.restart_features.is_some() {
                        s.daemon_incompatible
                    } else {
                        s.daemon_cannot
                    },
                    &ink.bold,
                )];
                top.push(line(p.reason.clone(), &Style::new()));
                if let Some(features) = &p.restart_features {
                    top.push(line(s.daemon_persist, &Style::new()));
                    top.push(line(s.daemon_features, &Style::new()));
                    for (name, enabled) in features.iter().take(3) {
                        top.push(line(s.daemon_feature(name, *enabled, num), &ink.dim));
                    }
                    if features.len() > 3 {
                        top.push(line(s.daemon_more(features.len() - 3, num), &ink.dim));
                    }
                }
                let can_restart = p.can_restart();
                let restart = if can_restart {
                    s.daemon_choices[1].to_owned()
                } else {
                    [s.daemon_choices[2], s.daemon_disabled_suffix].concat()
                };
                let mut number = 0;
                let mut row = |label: String, reason: Option<&'static str>| {
                    let disabled = reason.is_some();
                    RichRow {
                        prefix: if disabled {
                            "   ".to_owned()
                        } else {
                            number += 1;
                            s.numbered(number, "", num)
                        },
                        label: vec![Span::new(label)],
                        description: reason.map(str::to_owned),
                    }
                };
                let reason = if can_restart {
                    None
                } else if !p.managed {
                    Some(s.daemon_unmanaged)
                } else {
                    Some(s.daemon_unresolvable)
                };
                let rows = vec![
                    row(s.daemon_choices[0].to_owned(), None),
                    row(restart, reason),
                    row(s.daemon_choices[3].to_owned(), None),
                ];
                Parts {
                    top,
                    gap: true,
                    has_rows: true,
                    rows,
                    after: Vec::new(),
                    hint: None,
                    squeeze: Give::Fixed,
                }
            }
        }
    }
}
