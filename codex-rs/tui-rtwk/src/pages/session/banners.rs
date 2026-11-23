//! The notices above the composer: the chat's one-line notice (`NoticeLine`),
//! and the backend's banner with its numbered actions and the plain notices
//! under it (`BackendBanners`: the oracle's `bottom_pane/actionable_banner.rs`).
//! Flutter: a `MaterialBanner` with actions and an `onDismissed`; the number
//! keys and Esc are the page's `Focus.onKeyEvent` while the composer is empty
//! (`handle_inline_banner_key`), told what is shown through a `BannerGate`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, IntrinsicHeight, MainAxisSize, Padding, SizedBox,
};
use rtwk::provider::{read, select, watch};
use rtwk::style::{Attrs, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{
    AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, StatelessWidget, impl_widget,
};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::ids::{BackendId, Ticket};
use codex_tui_core::keymap::actions::{LIST_ACCEPT, LIST_CANCEL};
use codex_tui_core::screens::cells::ClockFormat;
use codex_tui_core::screens::select::list::{Columns, Item, List};
use codex_tui_core::screens::status::reset_time;
use codex_tui_core::screens::{ScreenStates, chat};
use codex_tui_core::threads::ThreadModel;

use crate::app::theme;
use crate::app::view::Hs;
use crate::keymap::{keymap, label};
use crate::l10n::strings as l10n;
use crate::pages::session::cells::status::strings as status_strings;
use crate::pages::session::strings as chat_strings;
use crate::widgets::measure;
use crate::widgets::options::{OptionRow, OptionStyles, Options};
use crate::widgets::picker::{Event, Picker};

/// What commands answered before any thread existed (`chat::Model.notes`):
/// the rows a thread's transcript would hold, drawn above the composer.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct NoticeLine;

impl_widget!(stateless eq NoticeLine);

impl StatelessWidget for NoticeLine {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let states = watch::<ScreenStates, _>(cx);
        if states.chat.notes.is_empty() {
            return SizedBox::shrink().into();
        }
        let mut column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min);
        for note in &states.chat.notes {
            column = column.child(super::cells::note::NoteCell { note: note.clone() });
        }
        column.into()
    }
}

/// The banner's hint line under its actions (`set_inline_banner`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BannerHint {
    /// `Press a number to choose`
    Choose,
    /// `Press a number to choose · esc to dismiss · type to continue`
    ChooseOrDismiss,
    /// `esc to dismiss · type to continue`
    Dismiss,
    None,
}

impl BannerHint {
    fn of(actions: bool, dismissible: bool) -> Self {
        match (actions, dismissible) {
            (true, false) => Self::Choose,
            (true, true) => Self::ChooseOrDismiss,
            (false, true) => Self::Dismiss,
            (false, false) => Self::None,
        }
    }
}

/// What the banner shows now, for the page's key handler (a `ValueNotifier`
/// the widget writes while it builds): the numbers it takes and whether Esc
/// dismisses it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BannerKeys {
    pub backend: Option<(BackendId, Ticket)>,
    pub actions: usize,
    pub dismissible: bool,
}

/// What the page and the banner tell each other: the banner writes what it
/// shows (`keys`), the page's key handler reads it and moves the cursor of
/// the rows (`options`) when a number is pressed.
#[derive(Clone)]
pub(crate) struct BannerGate {
    pub(crate) keys: Rc<Cell<BannerKeys>>,
    pub(crate) options: Options,
}

impl BannerGate {
    pub(crate) fn new() -> Self {
        Self {
            keys: Rc::new(Cell::new(BannerKeys::default())),
            options: Options::new(),
        }
    }
}

impl PartialEq for BannerGate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.keys, &other.keys)
    }
}

impl std::fmt::Debug for BannerGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.keys.get().fmt(f)
    }
}

/// The banner of the backend the thread runs on, if one is visible for the
/// model in use, and the backend's plain notice.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BackendBanners {
    pub(super) gate: BannerGate,
}

impl_widget!(stateful eq BackendBanners);

impl StatefulWidget for BackendBanners {
    type State = BackendBannersState;

    fn create_state(&self) -> BackendBannersState {
        BackendBannersState
    }
}

pub(super) struct BackendBannersState;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum BannerMsg {
    /// The user chose row `row` of the banner `Ticket` (a tap).
    Pick(BackendId, Ticket, usize),
}

impl State for BackendBannersState {
    type Widget = BackendBanners;
    type Msg = BannerMsg;

    fn update(&mut self, msg: BannerMsg, _: &BackendBanners, cx: &mut Cx<'_, Self>) -> Rebuild {
        let BannerMsg::Pick(backend, ticket, row) = msg;
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.pick_banner, (backend, ticket, row));
        Rebuild::No
    }

    fn build(&self, widget: &BackendBanners, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let words = chat_strings::of(cx);
        let session = watch::<SessionSlice, _>(cx);
        let states = watch::<ScreenStates, _>(cx);
        let reset_words = status_strings::of(cx);
        let (id, model) = select::<ThreadModel, _, _>(cx, |thread| {
            (
                thread.id().clone(),
                thread.screen_slots().chat.model.clone(),
            )
        });
        let backend = states.agents.owner_of(id.as_str());
        let active_model = model.as_deref().or(states.chat.status.model.as_deref());
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        let mut keys = BannerKeys::default();
        if let Some(occurrence) = session
            .backend_banners
            .get(&backend)
            // the Luna Reserve banner is the focused selector's (`ReserveSelector`)
            .filter(|occurrence| {
                occurrence.is_visible(active_model) && !occurrence.banner.is_reserve()
            })
        {
            let banner = &occurrence.banner;
            let (title, description) = if banner.reserve_pending(active_model) {
                (
                    strings.chat.usage_limit_title.to_owned(),
                    strings.chat.usage_limit_detail.to_owned(),
                )
            } else {
                let when = reset_when(banner, &states, reset_words);
                (
                    with_time(banner.title(), when.as_deref()),
                    with_time(banner.description(), when.as_deref()),
                )
            };
            let labels: Vec<String> = banner
                .ctas()
                .iter()
                .enumerate()
                .map(|(index, cta)| format!("{}. {}", index + 1, cta.label))
                .collect();
            let ticket = occurrence.ticket;
            keys = BannerKeys {
                backend: Some((backend, ticket)),
                actions: labels.len().min(9),
                dismissible: occurrence.is_dismissible(),
            };
            let hint = words.banner_hint(BannerHint::of(!labels.is_empty(), keys.dismissible));
            let header = header(title, description, &tokens);
            let block = if labels.is_empty() {
                // information only: a blank row above and below, the hint two rows under
                let mut block = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(SizedBox::new(None, Some(1), None))
                    .child(header);
                if let Some(hint) = hint {
                    block = block
                        .child(SizedBox::new(None, Some(1), None))
                        .child(inset(Text::new(hint.to_owned()).style(tokens.dim()).into()));
                }
                block.child(SizedBox::new(None, Some(1), None))
            } else {
                let rows = Rc::new(
                    labels
                        .into_iter()
                        .map(|label| OptionRow {
                            label,
                            description: None,
                        })
                        .collect::<Vec<_>>(),
                );
                let options = widget
                    .gate
                    .options
                    .on_accept(cx.map(move |row: usize| BannerMsg::Pick(backend, ticket, row)));
                let styles = OptionStyles {
                    cursor: tokens.accent().attrs(Attrs::BOLD),
                    label: Style::new(),
                    description: Style::new(),
                };
                let mut block = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(SizedBox::new(None, Some(1), None))
                    .child(header)
                    .child(SizedBox::new(None, Some(1), None))
                    .child(
                        // the rows as tall as they are (`Options` would take
                        // all the room it is given)
                        IntrinsicHeight::new(
                            Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .child(options.view(rows, styles, 0)),
                        ),
                    );
                if let Some(hint) = hint {
                    block = block
                        .child(SizedBox::new(None, Some(1), None))
                        .child(inset(Text::new(hint.to_owned()).style(tokens.dim()).into()));
                }
                block
            };
            column = column.child(AnyWidget::from(block).with_key(Key::value(ticket)));
        }
        widget.gate.keys.set(keys);
        if let Some(notice) = session.backend_notices.get(&backend) {
            column = column.child(Text::new(notice.clone()).style(tokens.dim()));
        }
        column.into()
    }
}

/// The banner's `{time}`: when the limit resets, in the viewer's clock and zone, with its day
/// when that is not the day of the read (oracle `actionable_banner`, `format_reset_timestamp`).
/// The zone is the host's (`status.utc_offset_secs`) unless the viewer's clock fixes one; a
/// clock the host has not read yet (`now` 0) counts the reset as today.
fn reset_when(
    banner: &chat::Banner,
    states: &ScreenStates,
    words: &status_strings::Strings,
) -> Option<String> {
    let at = banner.reset_at()?;
    let clock = states.cells.clock;
    let offset = clock.utc_offset.unwrap_or(states.status.utc_offset_secs);
    let read_at = if states.status.now > 0 {
        states.status.now
    } else {
        at
    };
    let hour24 = clock.format == ClockFormat::TwentyFourHour;
    Some(words.when(&reset_time(at, read_at, offset), hour24))
}

/// `text` with its `{time}` read as `when`; a banner without a reset time keeps it as written.
fn with_time(text: &str, when: Option<&str>) -> String {
    match when {
        Some(when) => text.replace("{time}", when),
        None => text.to_owned(),
    }
}

/// The Luna Reserve banner the selector shows now: the backend of the thread, and its occurrence
/// (visible for the model in use). The oracle shows it as a focused list in place of the composer
/// (`show_selection_view`), not as the inline banner.
pub(super) fn reserve_of<'a>(
    session: &'a SessionSlice,
    states: &ScreenStates,
    thread: &ThreadModel,
) -> Option<(BackendId, &'a chat::BannerOccurrence)> {
    let backend = states.agents.owner_of(thread.id().as_str());
    let model = thread.screen_slots().chat.model.clone();
    let active = model.as_deref().or(states.chat.status.model.as_deref());
    session
        .backend_banners
        .get(&backend)
        .filter(|occurrence| occurrence.banner.is_reserve() && occurrence.is_visible(active))
        .map(|occurrence| (backend, occurrence))
}

/// The Luna Reserve selector: `showModalBottomSheet`'s content (`Picker`) put in the bottom pane
/// where the composer was; the composer's draft is in `UiApp.ui`, so it is back untouched when
/// the selector closes (Esc, a pick: `picker_dismissed`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReserveSelector;

impl_widget!(stateful eq ReserveSelector);

impl StatefulWidget for ReserveSelector {
    type State = ReserveSelectorState;

    fn create_state(&self) -> ReserveSelectorState {
        ReserveSelectorState
    }
}

pub(super) struct ReserveSelectorState;

/// The row of the selector that closes it with no action (`Continue with Luna Reserve`).
const CONTINUE: &str = "continue";

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ReserveMsg {
    Event(Event),
}

impl State for ReserveSelectorState {
    type Widget = ReserveSelector;
    type Msg = ReserveMsg;

    fn update(&mut self, msg: ReserveMsg, _: &ReserveSelector, cx: &mut Cx<'_, Self>) -> Rebuild {
        let ReserveMsg::Event(event) = msg;
        let hs = read::<Hs, _>(cx);
        let session = read::<SessionSlice, _>(cx);
        let states = read::<ScreenStates, _>(cx);
        let thread = read::<ThreadModel, _>(cx);
        let Some((backend, occurrence)) = reserve_of(&session, &states, &thread) else {
            return Rebuild::No;
        };
        let ticket = occurrence.ticket;
        match event {
            // a purchase row opens its page and closes the selector
            Event::Choose(id) => match id.parse::<usize>() {
                Ok(row) => cx.emit(&hs.pick_banner, (backend, ticket, row)),
                Err(_) => cx.emit(&hs.dismiss_banner, (backend, ticket)),
            },
            Event::Cancel => cx.emit(&hs.dismiss_banner, (backend, ticket)),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, _: &ReserveSelector, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = l10n::of(cx);
        let words = chat_strings::of(cx);
        let keys = keymap(cx);
        let session = watch::<SessionSlice, _>(cx);
        let states = watch::<ScreenStates, _>(cx);
        let (id, model) = select::<ThreadModel, _, _>(cx, |thread| {
            (
                thread.id().clone(),
                thread.screen_slots().chat.model.clone(),
            )
        });
        let backend = states.agents.owner_of(id.as_str());
        let active = model.as_deref().or(states.chat.status.model.as_deref());
        let Some(banner) = session
            .backend_banners
            .get(&backend)
            .filter(|occurrence| occurrence.banner.is_reserve() && occurrence.is_visible(active))
            .map(|occurrence| &occurrence.banner)
        else {
            return SizedBox::shrink().into();
        };
        let pending = banner.reserve_pending(active);
        let (title, description) = if pending {
            (
                strings.chat.usage_limit_title,
                strings.chat.usage_limit_detail,
            )
        } else {
            (banner.title(), banner.description())
        };
        let mut items: Vec<Item> = banner
            .ctas()
            .into_iter()
            .enumerate()
            .map(|(row, cta)| Item {
                id: row.to_string(),
                name: cta.label,
                ..Item::default()
            })
            .collect();
        // Reserve in use: continuing is a local dismissal next to the purchase rows, and the
        // footer says esc continues working; otherwise the list's standard footer.
        let mut list = List {
            title: Some(title.to_owned()),
            subtitle: Some(description.to_owned()),
            numbered: true,
            panel: true,
            columns: Columns::AutoAllRows,
            hide_description_below: Some(24),
            standard_hint: true,
            ..List::default()
        };
        if !pending {
            items.push(Item {
                id: CONTINUE.to_owned(),
                name: words.reserve_continue.to_owned(),
                ..Item::default()
            });
            if let (Some(accept), Some(cancel)) =
                (label(&keys, LIST_ACCEPT), label(&keys, LIST_CANCEL))
            {
                list.footer_hint = Some(words.reserve_hint(&accept, &cancel));
            }
        }
        list.items = items;
        Picker::new(Rc::new(list), cx.map(ReserveMsg::Event)).into()
    }
}

/// Two cells in from the left and right (`Insets::tlbr(_, 2, _, 2)`).
fn inset(child: AnyWidget) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}

/// The bold title and the description, wrapped to the room less the inset;
/// more than eight rows keep seven and a dimmed `…` (`BannerContent`).
fn header(title: String, description: String, tokens: &theme::Tokens) -> AnyWidget {
    let (bold, plain, dim) = (tokens.bold(), Style::new(), tokens.dim());
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let room = usize::try_from(constraints.max_width - 4)
            .unwrap_or(1)
            .max(1);
        let rows = |text: &str| -> usize {
            if text.is_empty() {
                0
            } else {
                measure::rows(seg, text, room).len()
            }
        };
        let (title_rows, description_rows) = (rows(&title), rows(&description));
        let cut = title_rows + description_rows > 8;
        let budget = if cut { 7 } else { usize::MAX };
        let title_shown = title_rows.min(budget);
        let description_shown = description_rows.min(budget - title_shown.min(budget));
        let part = |text: &str, lines: usize, style| -> Option<AnyWidget> {
            (lines > 0).then(|| {
                Text::new(text.to_owned())
                    .max_lines(Some(lines))
                    .overflow(TextOverflow::Clip)
                    .style(style)
                    .into()
            })
        };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        for piece in [
            part(&title, title_shown, bold.clone()),
            part(&description, description_shown, plain.clone()),
        ]
        .into_iter()
        .flatten()
        {
            column = column.child(piece);
        }
        if cut {
            column = column.child(Text::line("…".to_owned()).style(dim.clone()));
        }
        inset(column.into())
    })
    .into()
}
