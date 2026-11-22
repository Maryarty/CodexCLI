//! The notices above the composer: the chat's one-line notice (`NoticeLine`),
//! and the backend's banner with its links and the plain notices under it
//! (`BackendBanners`: the oracle's `bottom_pane/actionable_banner.rs`).
//! Flutter: a `MaterialBanner` with actions and an `onDismissed`.

use rtwk::controls::{Callout, Hyperlink, StatusKind};
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::provider::{read, select, watch};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, Rebuild, State, StatefulWidget, StatelessWidget, impl_widget};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::ids::{BackendId, Ticket};
use codex_tui_core::screens::{ScreenStates, chat};
use codex_tui_core::threads::ThreadModel;

use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::pages::session::strings as chat_strings;

/// The chat screen's notice (a command's answer, or why a command is not
/// available): `add_info_message` is plain text, `add_error_message` is red.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct NoticeLine;

impl_widget!(stateless eq NoticeLine);

impl StatelessWidget for NoticeLine {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = chat_strings::of(cx);
        let states = watch::<ScreenStates, _>(cx);
        let Some(notice) = &states.chat.notice else {
            return SizedBox::shrink().into();
        };
        let notice = match notice {
            chat::ChatNotice::Command(notice) => notice.clone(),
            chat::ChatNotice::NotAvailable(command) => {
                codex_tui_core::commands::Notice::NotAvailable(*command)
            }
        };
        let text = Text::new(strings.notice(&notice));
        if notice.is_info() {
            text.into()
        } else {
            text.style(tokens.error()).into()
        }
    }
}

/// The banner of the backend the thread runs on, if one is visible for the
/// model in use, and the backend's plain notice.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BackendBanners;

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
    /// The user closed the banner of `Ticket` (a `Callout`'s `on_dismissed`).
    Dismiss(BackendId, Ticket),
}

impl State for BackendBannersState {
    type Widget = BackendBanners;
    type Msg = BannerMsg;

    fn update(&mut self, msg: BannerMsg, _: &BackendBanners, cx: &mut Cx<'_, Self>) -> Rebuild {
        let BannerMsg::Dismiss(backend, ticket) = msg;
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.dismiss_banner, (backend, ticket));
        Rebuild::No
    }

    fn build(&self, _: &BackendBanners, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let session = watch::<SessionSlice, _>(cx);
        let states = watch::<ScreenStates, _>(cx);
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
        if let Some(occurrence) = session
            .backend_banners
            .get(&backend)
            .filter(|occurrence| occurrence.is_visible(active_model))
        {
            let banner = &occurrence.banner;
            let (title, description) = if banner.reserve_pending(active_model) {
                (
                    strings.chat.usage_limit_title,
                    strings.chat.usage_limit_detail,
                )
            } else {
                (banner.title(), banner.description())
            };
            let mut callout = Callout::text(description.to_owned())
                .title(title.to_owned())
                .kind(StatusKind::Warning)
                .actions(
                    banner
                        .links()
                        .map(|(label, url)| Hyperlink::new(url).text(label.to_owned()).into()),
                );
            if occurrence.is_dismissible() {
                let ticket = occurrence.ticket;
                callout = callout.on_dismissed(cx.send(BannerMsg::Dismiss(backend, ticket)));
            }
            column = column.child(AnyWidget::from(callout).with_key(Key::value(occurrence.ticket)));
        }
        if let Some(notice) = session.backend_notices.get(&backend) {
            column = column.child(Text::new(notice.clone()).style(tokens.dim()));
        }
        column.into()
    }
}
