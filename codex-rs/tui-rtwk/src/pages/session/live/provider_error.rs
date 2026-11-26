//! The notice that stands in place of the composer when the server refused the
//! chat for its model provider (oracle `chatwidget/provider_error.rs`,
//! `bottom_pane_renderable`): the history stays readable, the draft waits in
//! the thread's composer controller, and three keys leave (`c` continues in a
//! new chat, `n` starts fresh, Esc opens the chats to resume).
//!
//! Flutter's `MaterialBanner` with its actions: a panel of the prompt's surface
//! (`BoxWidget`), the title and the words beside the lock (`Row`), and the
//! actions under it as a `Wrap` of tappable hints, which breaks between the
//! hints where the width gives out.

use rtwk::focus::KeyPress;
use rtwk::gestures::GestureDetector;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row, Wrap,
};
use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::keymap::{Keymap, fixed};

use crate::app::theme::Tokens;
use crate::keymap::presses;
use crate::pages::session::session_page::{ChatMsg, ChatState};
use crate::pages::session::strings::Strings;

/// Why the server refused the chat: the failure's kind and the ids of the
/// providers it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::pages::session) enum Lock {
    /// The chat's provider is not the one Codex uses now.
    Mismatch { provider: String, current: String },
    /// The chat's provider has no configuration any more.
    NotConfigured { provider: String },
    /// The organization requires another provider.
    PolicyProvider { provider: String, required: String },
    /// The chat's provider is the required one, its settings are not.
    PolicySettings,
}

impl Lock {
    /// The kind of a typed failure, read as the server sends it (`type` and the
    /// camelCase ids): the view never names the protocol's type.
    ///
    /// NEED(core): a projection on the thread (`ThreadModel::provider_lock`);
    /// this reads the wire shape until core carries one.
    pub(in crate::pages::session) fn of(value: &serde_json::Value) -> Option<Lock> {
        let id = |name: &str| value.get(name)?.as_str().map(str::to_owned);
        match value.get("type")?.as_str()? {
            "modelProviderMismatch" => Some(Lock::Mismatch {
                provider: id("modelProvider")?,
                current: id("currentModelProvider")?,
            }),
            "modelProviderNotConfigured" => Some(Lock::NotConfigured {
                provider: id("modelProvider")?,
            }),
            "modelProviderPolicyConflict" => {
                let (provider, required) = (id("modelProvider")?, id("requiredModelProvider")?);
                Some(if provider == required {
                    Lock::PolicySettings
                } else {
                    Lock::PolicyProvider { provider, required }
                })
            }
            _ => None,
        }
    }
}

/// The panel: the lock, the title and the words (`ProviderErrorNotice::as_renderable`).
pub(in crate::pages::session) fn notice(
    cx: &mut Cx<'_, ChatState>,
    strings: &'static Strings,
    tokens: &Tokens,
    lock: &Lock,
    can_continue: bool,
    location: Option<&str>,
    provider_names: &[codex_tui_core::screens::chat::ModelProviderName],
) -> AnyWidget {
    let dim = tokens.dim();
    let mut body = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::new(strings.provider_title(lock, can_continue)).style(tokens.bold()))
        .child(Text::new(strings.provider_message(lock, provider_names)).style(dim.clone()));
    if let Some(address) = location {
        body = body.child(Text::new(strings.provider_location(address)).style(dim));
    }
    BoxWidget::new()
        .style(tokens.prompt_surface(cx, false))
        .padding(EdgeInsets::only(2, 1, 2, 1))
        .child(
            Row::new()
                .spacing(2)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Text::line(strings.provider_lock))
                .child(Expanded::new(1, body)),
        )
        .into()
}

/// The actions under the panel, each a hint that is also a tap target.
pub(in crate::pages::session) fn footer(
    cx: &mut Cx<'_, ChatState>,
    strings: &'static Strings,
    tokens: &Tokens,
    can_continue: bool,
) -> AnyWidget {
    let (key_style, dim) = (tokens.bold(), tokens.dim());
    let mut entries = Vec::new();
    if can_continue {
        entries.push((
            strings.provider_continue_key,
            strings.provider_continue,
            ChatMsg::ProviderContinue,
        ));
    }
    entries.push((
        strings.provider_new_key,
        strings.provider_new,
        ChatMsg::ProviderNew,
    ));
    entries.push((
        strings.provider_other_key,
        strings.provider_other,
        ChatMsg::ProviderOthers,
    ));
    let mut hints = Wrap::new().spacing(1).run_spacing(0);
    for (index, (key, label, msg)) in entries.into_iter().enumerate() {
        let mut spans = Vec::new();
        if index > 0 {
            spans.push(Span::styled(strings.provider_separator, dim.clone()));
        }
        spans.push(Span::styled(key, key_style.clone()));
        spans.push(Span::styled(label, dim.clone()));
        hints = hints.child(
            GestureDetector::new()
                .child(Text::rich(spans))
                .on_tap(cx.map(move |_| msg.clone())),
        );
    }
    Padding::new(EdgeInsets::only(2, 0, 0, 0), hints).into()
}

/// The keys of the notice (`handle_key_event`'s `has_provider_error` arm):
/// plain presses only; Esc is the page's own.
pub(in crate::pages::session) fn keys(
    keymap: &Keymap,
    can_continue: bool,
) -> Vec<(KeyPress, ChatMsg)> {
    let mut actions = vec![(fixed::PROVIDER_NEW, ChatMsg::ProviderNew)];
    if can_continue {
        actions.push((fixed::PROVIDER_CONTINUE, ChatMsg::ProviderContinue));
    }
    actions
        .into_iter()
        .flat_map(|(id, msg)| {
            presses(keymap, id)
                .into_iter()
                .map(move |key| (key, msg.clone()))
        })
        .collect()
}
