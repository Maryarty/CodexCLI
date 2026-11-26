//! The notices above the composer: the chat's one-line notice (`NoticeLine`),
//! and the backend's banner with its numbered actions and the plain notices
//! under it (`BackendBanners`: the oracle's `bottom_pane/actionable_banner.rs`).
//! Flutter: a `MaterialBanner` with actions and an `onDismissed`; the number
//! keys and Esc are the page's `Focus.onKeyEvent` while the composer is empty
//! (`handle_inline_banner_key`), told what is shown through a `BannerGate`.

use std::cell::Cell;
use std::rc::Rc;

use linkify::{LinkFinder, LinkKind};
use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, IntrinsicHeight, MainAxisSize, Padding, SizedBox,
};
use rtwk::provider::{read, select, watch};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{
    AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, StatelessWidget, impl_widget,
};

use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::app::SessionSlice;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::keymap::actions::{LIST_ACCEPT, LIST_CANCEL};
use codex_tui_core::screens::cells::{ClockFormat, ViewerClock};
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
        let notes = select::<ScreenStates, _, _>(cx, |states| states.chat.notes.clone());
        if notes.is_empty() {
            return SizedBox::shrink().into();
        }
        let mut column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min);
        for note in &notes {
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

/// The banner on screen: the backend it is of and its occurrence's ticket, which together are the
/// banner's id (the entity is keyed by the request that raised it), or the security-setup
/// reminder of an occurrence. The page carries it between the number key and the app's answer
/// without reading either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BannerId {
    Backend(BackendId, Ticket),
    /// The reminder (`features::security_setup`): its action is the feature's verb, its dismissal
    /// is the page's (`BannerGate::dismissed`).
    Security(u64),
}

impl BannerId {
    /// The pick of the banner's row `row`: the app's, or the feature's verb.
    pub(crate) fn pick<S: 'static>(self, row: usize, hs: &Hs, cx: &mut Cx<'_, S>) {
        match self {
            BannerId::Backend(backend, ticket) => cx.emit(&hs.pick_banner, (backend, ticket, row)),
            BannerId::Security(_) => crate::features::open_security_page(cx, hs),
        }
    }

    /// The dismissal of the banner: the app's, or the page's own record of the occurrence, which
    /// asks for a new build (the answer is whether it does).
    pub(crate) fn dismiss<S: 'static>(
        self,
        gate: &BannerGate,
        hs: &Hs,
        cx: &mut Cx<'_, S>,
    ) -> Rebuild {
        match self {
            BannerId::Backend(backend, ticket) => {
                cx.emit(&hs.dismiss_banner, (backend, ticket));
                Rebuild::No
            }
            BannerId::Security(occurrence) => {
                gate.dismissed.set(gate.dismissed.get().max(occurrence));
                Rebuild::Yes
            }
        }
    }
}

/// What the banner shows now, for the page's key handler (a `ValueNotifier`
/// the widget writes while it builds): the numbers it takes and whether Esc
/// dismisses it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BannerKeys {
    pub backend: Option<BannerId>,
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
    /// The newest occurrence of the security reminder the user dismissed (0: none): kept here, so
    /// the dismissal survives the usage banner's replacement and the thread switches.
    pub(crate) dismissed: Rc<Cell<u64>>,
    /// The shortcut menu of `/presets` is open over the composer (`quick_dial_menu`): its keys come
    /// first, any other key closes it.
    pub(crate) quick_menu: Rc<Cell<bool>>,
}

impl BannerGate {
    pub(crate) fn new() -> Self {
        Self {
            keys: Rc::new(Cell::new(BannerKeys::default())),
            options: Options::new(),
            dismissed: Rc::new(Cell::new(0)),
            quick_menu: Rc::new(Cell::new(false)),
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
    /// What the page keeps of the dismissed reminders (`gate.dismissed`), told by the page's
    /// build: a dismissal asks the page for a build, and this is what reaches the banner by it.
    pub(super) dismissed: u64,
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
    /// The user chose row `row` of the banner `BannerId` names (a tap).
    Pick(BannerId, usize),
}

impl State for BackendBannersState {
    type Widget = BackendBanners;
    type Msg = BannerMsg;

    fn update(&mut self, msg: BannerMsg, _: &BackendBanners, cx: &mut Cx<'_, Self>) -> Rebuild {
        let BannerMsg::Pick(banner, row) = msg;
        let hs = read::<Hs, _>(cx);
        banner.pick(row, &hs, cx);
        Rebuild::No
    }

    fn build(&self, widget: &BackendBanners, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let session = watch::<SessionSlice, _>(cx);
        let reset_words = status_strings::of(cx);
        let (id, model, running) = select::<ThreadModel, _, _>(cx, |thread| {
            (
                thread.id().clone(),
                thread.screen_slots().chat.model.clone(),
                thread.is_running(),
            )
        });
        let place = place_of(cx, &id);
        let backend = place.backend;
        let active_model = model.as_deref().or(place.status_model.as_deref());
        let dismissed = widget.dismissed.max(widget.gate.dismissed.get());
        let security = select::<FeatureStates, _, _>(cx, move |features| {
            let setup = &features.security_setup;
            setup
                .notice()
                .filter(|_| setup.occurrence() > dismissed)
                .map(|notice| (notice.clone(), setup.occurrence()))
        });
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        widget.gate.keys.set(BannerKeys::default());
        if !running
            && let Some(occurrence) = session
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
                let when = reset_when(banner, &place.clock, reset_words);
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
            let id = BannerId::Backend(backend, occurrence.ticket);
            let block = banner_block(
                cx,
                &widget.gate,
                &tokens,
                id,
                (title, description),
                BannerHint::of(!labels.is_empty(), occurrence.is_dismissible()),
                labels,
            );
            column = column.child(block.with_key(Key::value(occurrence.ticket)));
        } else if !running && let Some((notice, occurrence)) = security {
            // the reminder shows while no banner of the backend applies and the user has not
            // dismissed this occurrence of it (`show_security_setup`)
            let id = BannerId::Security(occurrence);
            let block = banner_block(
                cx,
                &widget.gate,
                &tokens,
                id,
                (notice.title, notice.description),
                BannerHint::ChooseOrDismiss,
                vec![format!("1. {}", notice.action.label)],
            );
            column = column.child(block.with_key(Key::value(("security", occurrence))));
        }
        if let Some(notice) = session.backend_notices.get(&backend) {
            column = column.child(Text::new(notice.clone()).style(tokens.dim()));
        }
        column.into()
    }
}

/// A banner's block: a blank row above, the header, its numbered actions (a tap picks a row; none:
/// an information banner) and the hint under them (`set_inline_banner`).
fn banner_block(
    cx: &mut Cx<'_, BackendBannersState>,
    gate: &BannerGate,
    tokens: &theme::Tokens,
    id: BannerId,
    copy: (String, String),
    hint: BannerHint,
    labels: Vec<String>,
) -> AnyWidget {
    let words = chat_strings::of(cx);
    let options = gate
        .options
        .on_accept(cx.map(move |row: usize| BannerMsg::Pick(id, row)));
    let styles = OptionStyles {
        cursor: tokens.accent().attrs(Attrs::BOLD),
        label: Style::new(),
        description: Style::new(),
    };
    let gate = gate.clone();
    let tokens = tokens.clone();
    LayoutBuilder::new(move |cx, constraints| {
        let available = usize::try_from(constraints.max_height).unwrap_or_default();
        if available == 0 {
            gate.keys.set(BannerKeys::default());
            return SizedBox::shrink().into();
        }
        let (header, header_rows) = header(cx, &copy, &tokens, constraints.max_width);
        // A clipped banner prioritizes the visible actions over its decorations and heading.
        let compact = available < header_rows + 2 + labels.len();
        let actions = available.min(labels.len());
        gate.keys.set(BannerKeys {
            backend: Some(id),
            actions: actions.min(9),
            dismissible: matches!(hint, BannerHint::Dismiss | BannerHint::ChooseOrDismiss),
        });
        let hint = (available >= header_rows + 4 + labels.len())
            .then(|| words.banner_hint(hint))
            .flatten();
        if actions == 0 {
            if compact {
                return header;
            }
            // information only: a blank row above and below, the hint two rows under
            let mut block = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(None, Some(1), None))
                .child(header);
            if let Some(hint) = hint {
                block = block
                    .child(SizedBox::new(None, Some(1), None))
                    .child(banner_hint(hint, &tokens));
            }
            return block.child(SizedBox::new(None, Some(1), None)).into();
        }
        let rows = Rc::new(
            labels
                .iter()
                .take(actions)
                .cloned()
                .map(|label| OptionRow {
                    label,
                    description: None,
                })
                .collect::<Vec<_>>(),
        );
        let list = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(options.view(rows, styles.clone(), 0));
        let actions: AnyWidget = if compact {
            // A tight viewport gives Options its real room on the first layout.
            SizedBox::new(
                None,
                Some(i32::try_from(actions).unwrap_or(i32::MAX)),
                Some(list.into()),
            )
            .into()
        } else {
            IntrinsicHeight::new(list).into()
        };
        if compact {
            return actions;
        }
        let mut block = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SizedBox::new(None, Some(1), None))
            .child(header)
            .child(SizedBox::new(None, Some(1), None))
            .child(actions);
        if let Some(hint) = hint {
            block = block
                .child(SizedBox::new(None, Some(1), None))
                .child(banner_hint(hint, &tokens));
        }
        block.into()
    })
    .into()
}

fn banner_hint(hint: &str, tokens: &theme::Tokens) -> AnyWidget {
    inset(
        Text::new(hint.to_owned())
            .style(tokens.dim())
            .max_lines(Some(1))
            .into(),
    )
}

/// The viewer's clock and the host's zone and time, which the banner's `{time}` is read in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Clock {
    viewer: ViewerClock,
    utc_offset_secs: i32,
    now: i64,
}

/// What the banners draw of the screens' states (a slice, ADR-013): the backend that owns the
/// thread, the model the status line shows and the clock.
#[derive(Clone, Debug, PartialEq)]
struct Place {
    backend: BackendId,
    status_model: Option<String>,
    clock: Clock,
}

fn place_of<S: 'static>(cx: &mut Cx<'_, S>, thread: &ThreadId) -> Place {
    let thread = thread.clone();
    select::<ScreenStates, _, _>(cx, move |states| Place {
        backend: states.agents.owner_of(thread.as_str()),
        status_model: states.chat.status.model.clone(),
        clock: Clock {
            viewer: states.cells.clock,
            utc_offset_secs: states.status.utc_offset_secs,
            now: states.status.now,
        },
    })
}

/// The banner's `{time}`: when the limit resets, in the viewer's clock and zone, with its day
/// when that is not the day of the read (oracle `actionable_banner`, `format_reset_timestamp`).
/// The zone is the host's (`status.utc_offset_secs`) unless the viewer's clock fixes one; a
/// clock the host has not read yet (`now` 0) counts the reset as today.
fn reset_when(
    banner: &chat::Banner,
    clock: &Clock,
    words: &status_strings::Strings,
) -> Option<String> {
    let at = banner.reset_at()?;
    let offset = clock.viewer.utc_offset.unwrap_or(clock.utc_offset_secs);
    let read_at = if clock.now > 0 { clock.now } else { at };
    let clock = clock.viewer;
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
    backend: BackendId,
    status_model: Option<&str>,
    thread: &ThreadModel,
) -> Option<(BackendId, &'a chat::BannerOccurrence)> {
    let model = thread.screen_slots().chat.model.clone();
    let active = model.as_deref().or(status_model);
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
        let thread = read::<ThreadModel, _>(cx);
        let place = place_of(cx, thread.id());
        let Some((backend, occurrence)) = reserve_of(
            &session,
            place.backend,
            place.status_model.as_deref(),
            &thread,
        ) else {
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
        let (id, model) = select::<ThreadModel, _, _>(cx, |thread| {
            (
                thread.id().clone(),
                thread.screen_slots().chat.model.clone(),
            )
        });
        let place = place_of(cx, &id);
        let backend = place.backend;
        let active = model.as_deref().or(place.status_model.as_deref());
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
fn header(
    cx: &mut Cx<'_>,
    (title, description): &(String, String),
    tokens: &theme::Tokens,
    width: i32,
) -> (AnyWidget, usize) {
    let (bold, plain, dim) = (tokens.bold(), Style::new(), tokens.dim());
    let seg = segmenter_of(cx);
    let room = usize::try_from(width - 4).unwrap_or(1).max(1);
    let rows = |text: &str| -> usize {
        if text.is_empty() {
            0
        } else {
            measure::rows(seg, text, room).len()
        }
    };
    let (title_rows, description_rows) = (rows(title), rows(description));
    let cut = title_rows + description_rows > 8;
    let budget = if cut { 7 } else { usize::MAX };
    let title_shown = title_rows.min(budget);
    let description_shown = description_rows.min(budget - title_shown.min(budget));
    let part = |text: &str, lines: usize, style| -> Option<AnyWidget> {
        (lines > 0).then(|| {
            Text::rich(banner_spans(text, style))
                .max_lines(Some(lines))
                .overflow(TextOverflow::Clip)
                .into()
        })
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    for piece in [
        part(title, title_shown, bold),
        part(description, description_shown, plain),
    ]
    .into_iter()
    .flatten()
    {
        column = column.child(piece);
    }
    if cut {
        column = column.child(Text::line("…".to_owned()).style(dim.clone()));
    }
    let height = title_shown + description_shown + usize::from(cut);
    (inset(column.into()), height)
}

/// Keep the source copy literal, and give every painted fragment of a web URL its full target.
fn banner_spans(text: &str, style: Style) -> Vec<Span> {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    finder
        .spans(text)
        .map(|span| {
            let text = span.as_str();
            let style = if span.kind().is_some()
                && url::Url::parse(text).is_ok_and(|url| {
                    matches!(url.scheme(), "http" | "https") && url.host().is_some()
                }) {
                style.clone().link(text)
            } else {
                style.clone()
            };
            Span::styled(text.to_owned(), style)
        })
        .collect()
}
