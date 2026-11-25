//! The footer under the composer (oracle `bottom_pane/footer.rs`,
//! `chat_composer/passive_hints.rs`): the passive hints (`← for agents`,
//! `? for shortcuts`, `tab to queue message`, the context meter, the warning
//! notice), or one transient line (the quit reminder, the Esc hint, the close
//! hint of the shortcut overlay).
//!
//! Flutter writes it as a `LayoutBuilder` over a `Row` of hint widgets that
//! picks the spelling of each from the measured width; so does this: the hints
//! are measured with the tree's segmenter, `hints::fit` (a pure function) picks
//! the spelling of each and the row is laid out by `Row` and `Expanded`.
//! What the footer shows comes from the providers (the thread's usage slot,
//! the screen facts, the warnings kept); the page tells it only what lives in
//! the UI (the line, the draft, a running turn).

mod effort;
mod effort_frames;
pub use effort::{Aura, Prompt};
pub mod entries;
mod flash;
pub use flash::{Flash, Timed, Tone};
pub mod hints;
mod indicators;
mod lead;
mod sparkle;
pub use sparkle::Sparkle;
mod status_rows;
pub use status_rows::StatusRows;
mod strings;

use std::rc::Rc;

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::provider::{Provider, select};
use rtwk::style::Span;
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::format::format_tokens_compact;
use codex_tui_core::keymap::actions::COMPOSER_TOGGLE_SHORTCUTS;
use codex_tui_core::kit::vim::VimMode;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::chat::facts::{self as chat_facts, GoalIndicator};
use codex_tui_core::threads::ThreadModel;

use crate::app::theme::Tokens;
use crate::l10n::strings as l10n;
use crate::pages::session::composer::strings as composer_strings;
use crate::widgets::page_keys;
use crate::widgets::status_line::SEPARATOR;

use hints::{Kind, Label};

/// What the footer row says (`FooterMode`, bar the history search that owns
/// the footer in the page).
///
/// The oracle compiles the double-press quit shortcut out
/// (`DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED = false`, `bottom_pane/mod.rs`): the
/// first Ctrl+C interrupts or quits, so `Quit` is a line the footer can draw
/// (its snapshots do) and the page never shows. `EscIdle` is the composer's own
/// answer to an Esc the backtrack did not take; the page's Esc always primes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// The passive hints: navigation, actions, indicators.
    Passive,
    /// `⌃c again to quit`.
    Quit,
    /// `esc esc to edit previous message`.
    EscIdle,
    /// `esc again to edit previous message`: backtrack is primed.
    EscPrimed,
    /// The shortcut overlay is open: how to close it (`close_hint`).
    Close,
}

/// Cells the footer starts in (`FOOTER_INDENT_COLS`).
const INDENT: i32 = 2;
/// The clear cell at the right edge (`hint_area`).
const RIGHT_CLEAR: i32 = 1;

/// The footer line `line`: what the page keeps of the draft and the turn,
/// the rest is read.
#[derive(Clone)]
pub struct Footer {
    pub line: Line,
    /// The agents page is reachable: the first hint is its key.
    pub agents: bool,
    /// The draft is empty (shell mode's `!` counts as content).
    pub draft_empty: bool,
    /// The composer is in shell mode.
    pub shell: bool,
    /// The draft's Vim mode, none with Vim off.
    pub vim: Option<VimMode>,
    /// `set_footer_hint_override`: `(key, label)` items that take the left
    /// side from the passive hints.
    pub hint_override: Option<Vec<(String, String)>>,
    /// `show_footer_flash`: a line over the hint override; `Timed` ends it.
    pub flash: Option<Flash>,
    /// A tap on the queue hint.
    pub on_queue: Option<Handler<()>>,
    /// A tap on the warning notice.
    pub on_warnings: Option<Handler<()>>,
    /// A tap on the close hint of the shortcut overlay.
    pub on_close: Option<Handler<()>>,
}

impl PartialEq for Footer {
    fn eq(&self, other: &Footer) -> bool {
        self.line == other.line
            && self.agents == other.agents
            && self.draft_empty == other.draft_empty
            && self.shell == other.shell
            && self.vim == other.vim
            && self.hint_override == other.hint_override
            && self.flash == other.flash
            && self.on_queue == other.on_queue
            && self.on_warnings == other.on_warnings
            && self.on_close == other.on_close
    }
}

impl_widget!(stateless eq Footer);

/// What the context meter says (`context_window_line`).
enum Meter {
    Left(i64),
    Used(i64),
    Unknown,
}

impl StatelessWidget for Footer {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        // the flash wins over the hint override, both over the line
        let lead = match (&self.flash, &self.hint_override) {
            (Some(flash), _) => Some(lead::Lead::Flash(flash.clone())),
            (None, Some(items)) => Some(lead::Lead::Hints(items.clone())),
            (None, None) => None,
        };
        if let Some(lead) = lead {
            let spec = self.spec(cx);
            let passive = self.line == Line::Passive;
            return LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                spec.lead_row(
                    &lead,
                    passive,
                    &|text: &str| seg.width(text),
                    usize::try_from(constraints.max_width).unwrap_or(0),
                )
            })
            .into();
        }
        if self.line == Line::Close {
            return close_row(cx, self.on_close.clone());
        }
        if let Some(line) = self.transient(cx) {
            return line;
        }
        let spec = self.spec(cx);
        LayoutBuilder::new(move |cx, constraints| {
            let seg = segmenter_of(cx);
            spec.row(
                &|text: &str| seg.width(text),
                usize::try_from(constraints.max_width).unwrap_or(0),
            )
        })
        .into()
    }
}

impl Footer {
    /// The transient line (`Quit`, `EscIdle`, `EscPrimed`, `Close`), if the
    /// footer is one.
    fn transient(&self, cx: &mut Cx<'_>) -> Option<AnyWidget> {
        let tokens = Tokens::of(cx);
        let own = strings::of(cx);
        match self.line {
            Line::Passive => None,
            Line::Quit => Some(transient(vec![
                Span::styled(own.key_ctrl_c, tokens.bold()),
                Span::styled(own.quit_again, tokens.dim()),
            ])),
            Line::EscIdle => Some(transient(vec![
                Span::styled(own.esc_key, tokens.bold()),
                Span::styled(own.esc_between, tokens.dim()),
                Span::styled(own.esc_key, tokens.bold()),
                Span::styled(own.esc_idle_rest, tokens.dim()),
            ])),
            Line::EscPrimed => Some(transient(vec![
                Span::styled(own.esc_key, tokens.bold()),
                Span::styled(own.esc_primed_rest, tokens.dim()),
            ])),
            Line::Close => None,
        }
    }

    /// What the passive hints are made of, read from the providers.
    fn spec(&self, cx: &mut Cx<'_>) -> Rc<Spec> {
        let tokens = Tokens::of(cx);
        let own = strings::of(cx);
        let pages = l10n::of(cx).pages;
        let shell_label = composer_strings::of(cx).shell_mode;
        let (running, received, percent, used) =
            select::<ThreadModel, (bool, bool, Option<i64>, i64), _>(cx, |thread| {
                let usage = &thread.screen_slots().status;
                (
                    thread.is_running(),
                    usage.received,
                    usage.context.map(|context| context.percent_left),
                    usage.tokens.blended_total(),
                )
            });
        let (now, status_line_enabled, warnings, chat) =
            select::<ScreenStates, _, _>(cx, |states| {
                (
                    states.agents.now,
                    states
                        .chat
                        .status
                        .items
                        .as_ref()
                        .is_none_or(|items| !items.is_empty()),
                    states.dialogs.retained.len(),
                    states.chat.clone(),
                )
            });
        let facts = select::<ThreadModel, chat_facts::Footer, _>(cx, move |thread| {
            chat_facts::footer(&chat, thread.screen_slots())
        });
        let goal = select::<ThreadModel, Option<GoalIndicator>, _>(cx, move |thread| {
            chat_facts::goal_indicator(thread, now)
        });
        // `/ide` on: the small commands' state, absent where the app's models are not provided
        let ide = Provider::<FeatureStates>::maybe_of(cx, false).is_some()
            && select::<FeatureStates, bool, _>(cx, |features| features.small_cmds.ide.enabled);
        let meter = match (percent, received) {
            (Some(percent), _) => Meter::Left(percent),
            (None, true) if used > 0 => Meter::Used(used),
            _ => Meter::Unknown,
        };
        Rc::new(Spec {
            running,
            agents: self.agents && facts.agents_navigation,
            draft_empty: self.draft_empty,
            shell: self.shell,
            vim: self.vim,
            plan: facts.plan_indicator,
            // the key cycles the mode only while the thread is idle
            // (`show_cycle = can_type && !is_task_running`)
            show_cycle: !running,
            goal,
            ide,
            side: facts
                .side_label
                .or_else(|| facts.side.as_ref().map(strings::side_footer))
                .map(|label| (label, facts.side_active)),
            agent_label: facts.agent_label,
            context_pending: facts.usage_pending,
            status_line_enabled,
            warnings,
            meter,
            on_queue: self.on_queue.clone(),
            on_warnings: self.on_warnings.clone(),
            tokens,
            own,
            agent_hint: (pages.key_agents, pages.hint_agents),
            shortcuts: (pages.key_shortcuts, pages.hint_shortcuts),
            shell_label,
            warning_style: Tokens::of(cx).warning(),
        })
    }
}

/// How to close the shortcut overlay (`close_hint`): the toggle's key from the
/// keymap, ` / `, `esc`, ` close`; just `esc close` while the toggle is unbound
/// or the whole does not fit the room.
fn close_row(cx: &mut Cx<'_>, on_tap: Option<Handler<()>>) -> AnyWidget {
    let tokens = Tokens::of(cx);
    let own = strings::of(cx);
    let key = crate::keymap::label(&crate::keymap::keymap(cx), COMPOSER_TOGGLE_SHORTCUTS);
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let mut short = vec![
            Span::styled(own.esc_key, tokens.dim()),
            Span::styled(own.close_rest, tokens.dim()),
        ];
        let mut long = key.as_ref().map(|key| {
            vec![
                Span::styled(key.clone(), tokens.dim()),
                Span::styled(own.close_between, tokens.dim()),
                Span::styled(own.esc_key, tokens.dim()),
                Span::styled(own.close_rest, tokens.dim()),
            ]
        });
        // a tap on the hint closes the overlay (ADR-011)
        tapped(&mut short, &on_tap);
        if let Some(spans) = long.as_mut() {
            tapped(spans, &on_tap);
        }
        let room = usize::try_from(constraints.max_width - INDENT).unwrap_or(0);
        let fits = |spans: &[Span]| {
            spans
                .iter()
                .map(|span| seg.width(&span.text))
                .sum::<usize>()
                <= room
        };
        transient(long.filter(|spans| fits(spans)).unwrap_or(short))
    })
    .into()
}

/// One line of spans, `INDENT` cells in, cut at the right edge where it is longer than the
/// window (the oracle's `render_footer_line` is a `Paragraph` the area clips): the text is the
/// row's `Expanded`, so a row as wide as the text never overflows by the indent.
fn transient(spans: Vec<Span>) -> AnyWidget {
    Row::new()
        .child(SizedBox::new(Some(INDENT), None, None))
        .child(Expanded::new(
            1,
            Text::rich_line(spans)
                .max_lines(Some(1))
                .overflow(TextOverflow::Clip),
        ))
        .into()
}

/// The cells the hints share: the width less the indent and the clear cell.
fn room_of(total: usize) -> usize {
    total
        .saturating_sub(INDENT as usize)
        .saturating_sub(RIGHT_CLEAR as usize)
}

/// The widths of each hint's spellings, for `hints::fit`.
fn labels_of(hints: &[(Kind, Vec<Form>)]) -> Vec<Label> {
    hints
        .iter()
        .map(|(kind, forms)| Label {
            kind: *kind,
            widths: forms.iter().map(|form| form.cells).collect(),
        })
        .collect()
}

/// What the passive hints are made of, with the styles and strings.
struct Spec {
    running: bool,
    agents: bool,
    draft_empty: bool,
    shell: bool,
    vim: Option<VimMode>,
    plan: bool,
    show_cycle: bool,
    goal: Option<GoalIndicator>,
    /// `/ide` is on (`ide_context_active`): `IDE context` after the mode or goal.
    ide: bool,
    /// The side conversation's label and whether it is the one shown.
    side: Option<(String, bool)>,
    agent_label: Option<String>,
    /// The resumed thread has no token usage yet: no context meter.
    context_pending: bool,
    status_line_enabled: bool,
    warnings: usize,
    meter: Meter,
    on_queue: Option<Handler<()>>,
    on_warnings: Option<Handler<()>>,
    tokens: Tokens,
    own: &'static strings::Strings,
    agent_hint: (&'static str, &'static str),
    shortcuts: (&'static str, &'static str),
    shell_label: &'static str,
    warning_style: rtwk::style::Style,
}

/// `TextSpan.recognizer`: every span of a hint taps through `on_tap`.
fn tapped(spans: &mut [Span], on_tap: &Option<Handler<()>>) {
    if let Some(tap) = on_tap {
        for span in spans {
            *span = span.clone().tap(tap.clone());
        }
    }
}

/// A spelling of a hint: its cells and the widget that draws them.
struct Form {
    cells: usize,
    widget: AnyWidget,
}

impl Spec {
    /// The hints in the oracle's order of `passive_hints`, each with its
    /// spellings widest first.
    fn forms(&self, width: &dyn Fn(&str) -> usize) -> Vec<(Kind, Vec<Form>)> {
        let mut out: Vec<(Kind, Vec<Form>)> = Vec::new();
        let dim = self.tokens.dim();
        let plain = |text: String, style: rtwk::style::Style| -> Form {
            Form {
                cells: width(&text),
                widget: Text::line(text).style(style).into(),
            }
        };
        if self.shell {
            out.push((
                Kind::Shell,
                vec![plain(self.shell_label.to_owned(), self.tokens.shell())],
            ));
        }
        if let Some(forms) = indicators::vim(self, width) {
            out.push((Kind::Vim, forms));
        }
        if self.plan {
            out.push((Kind::Plan, indicators::plan(self, width)));
        } else if let Some(forms) = indicators::goal(self, width) {
            out.push((Kind::Goal, forms));
        }
        if let Some(forms) = indicators::ide(self, width) {
            out.push((Kind::Ide, forms));
        }
        if self.running && !self.draft_empty && !self.shell {
            let key = self.own.queue_key;
            let spelling = |rest: &'static str| -> Form {
                let mut spans = vec![
                    Span::styled(key, self.tokens.bold()),
                    Span::styled(rest, dim.clone()),
                ];
                tapped(&mut spans, &self.on_queue);
                Form {
                    cells: width(key) + width(rest),
                    widget: Text::rich_line(spans).into(),
                }
            };
            out.push((
                Kind::Action,
                vec![
                    spelling(self.own.queue_long),
                    spelling(self.own.queue_short),
                ],
            ));
        }
        if let Some((label, active)) = &self.side {
            let kind = if *active {
                Kind::SideActive
            } else {
                Kind::SideNav
            };
            out.push((kind, indicators::side(self, label, width)));
        }
        if !self.status_line_enabled
            && let Some(forms) = indicators::agent_name(self, width)
        {
            out.push((Kind::AgentName, forms));
        }
        if self.draft_empty {
            let bar = |(key, hint): (&'static str, &'static str)| -> Form {
                Form {
                    cells: width(key) + 1 + width(hint),
                    widget: page_keys::footer_with(
                        [(key, hint)],
                        self.tokens.bold(),
                        self.tokens.dim(),
                        SEPARATOR,
                    )
                    // a click on the hint presses the key it names (ADR-011)
                    .pressable(true)
                    .into(),
                }
            };
            if self.agents {
                out.push((Kind::Agents, vec![bar(self.agent_hint)]));
            }
            out.push((Kind::Shortcuts, vec![bar(self.shortcuts)]));
        }
        if !self.status_line_enabled && !self.context_pending && self.side.is_none() && !self.shell
        {
            let text = match self.meter {
                Meter::Left(percent) => self.own.left(percent),
                Meter::Used(used) => self.own.used(&format_tokens_compact(used)),
                Meter::Unknown => self.own.context_unknown.to_owned(),
            };
            out.push((Kind::Context, vec![plain(text, dim)]));
        }
        // the notice yields to a draft kept while a task runs: the queue hint is the row's
        // (`shows_passive_line`)
        if self.warnings > 0 && (self.draft_empty || !self.running) {
            out.push((Kind::Warning, self.warning_forms(width)));
        }
        out
    }

    /// `warning_labels`: the full notice, the compact one, the count.
    fn warning_forms(&self, width: &dyn Fn(&str) -> usize) -> Vec<Form> {
        let own = self.own;
        let count = self.warnings;
        let dim = self.tokens.dim();
        let accent = self.warning_style.clone();
        let join = SEPARATOR;
        let badge = || Span::styled(own.warning_badge, dim.clone());
        let spans: [Vec<Span>; 3] = [
            vec![
                badge(),
                Span::styled(own.warnings(count), accent.clone()),
                Span::styled(join, dim.clone()),
                Span::styled(own.warning_key, self.tokens.bold()),
                Span::styled(own.warning_view, dim.clone()),
            ],
            vec![
                badge(),
                Span::styled(count.to_string(), accent.clone()),
                Span::styled(join, dim.clone()),
                Span::styled(own.warning_key, self.tokens.bold()),
            ],
            vec![badge(), Span::styled(count.to_string(), accent)],
        ];
        spans
            .into_iter()
            .map(|mut spans| {
                let cells = spans.iter().map(|span| width(&span.text)).sum();
                tapped(&mut spans, &self.on_warnings);
                Form {
                    cells,
                    widget: Text::rich_line(spans).into(),
                }
            })
            .collect()
    }

    /// The rows the hints want at `total` cells (`desired_rows`).
    fn wanted_rows(&self, width: &dyn Fn(&str) -> usize, total: usize) -> usize {
        let hints = self.forms(width);
        let labels = labels_of(&hints);
        usize::from(hints::desired_rows(&labels, room_of(total)))
    }

    /// The footer at `total` cells: the spellings that fit, in one row or two.
    fn row(&self, width: &dyn Fn(&str) -> usize, total: usize) -> AnyWidget {
        let mut hints = self.forms(width);
        let room = room_of(total);
        let labels = labels_of(&hints);
        let rows = usize::from(hints::desired_rows(&labels, room)).max(1);
        let layout = hints::fit(&labels, room, rows > 1);
        let separator = || -> AnyWidget { Text::line(SEPARATOR).style(self.tokens.dim()).into() };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        for row in 0..rows {
            let mut left: Vec<AnyWidget> = Vec::new();
            let mut right: Vec<AnyWidget> = Vec::new();
            let mut lone_meter = false;
            for item in layout.placed.iter().filter(|item| item.row == row) {
                let Some((_, forms)) = hints.iter_mut().find(|(kind, _)| *kind == item.kind) else {
                    continue;
                };
                let form = forms.swap_remove(item.form);
                let group = if item.kind.left() {
                    &mut left
                } else {
                    &mut right
                };
                if !group.is_empty() {
                    group.push(separator());
                }
                lone_meter |= item.kind == Kind::Context
                    && layout.rows[row].right == form.cells
                    && layout.rows[row].width() < room;
                group.push(
                    SizedBox::new(i32::try_from(form.cells).ok(), Some(1), Some(form.widget))
                        .into(),
                );
            }
            // a row of no labels is no row (`desired_rows` is 0 when no label fits)
            if left.is_empty() && right.is_empty() {
                continue;
            }
            let mut line = Row::new().child(SizedBox::new(Some(INDENT), Some(1), None));
            for widget in left {
                line = line.child(widget);
            }
            line = line.child(Expanded::new(1, SizedBox::shrink()));
            for widget in right {
                line = line.child(widget);
            }
            line = line.child(SizedBox::new(
                Some(RIGHT_CLEAR + i32::from(lone_meter)),
                None,
                None,
            ));
            column = column.child(line);
        }
        column.into()
    }
}
