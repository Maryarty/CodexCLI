//! The provider setup wizards of the sign-in step (S03): the body the
//! step shows while a provider is chosen (`State::Provider`) and once it is
//! written (`State::ProviderConfigured`). Oracle: `onboarding/gcp.rs`,
//! `bedrock.rs`, `gateway.rs`, `provider_setup.rs`.
//!
//! In Flutter each wizard is a `StatefulWidget` over its core model (the
//! stage, its error) whose `State` holds the `TextEditingController`s and
//! `FocusNode`s of its fields; the keys are the fields' own and the step's
//! (Esc bubbles up to it: one stage back); what a wizard does is an intent
//! to core, the setup runs in the host.

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, SizedBox};
use rtwk::style::Span;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::onboarding::auth::{Provider, provider::Wizard};

use super::{Look, paragraph, rows_of};

pub mod bedrock;
pub mod gateway;
pub mod gateway_login;
pub mod gcp;
pub mod gov_cloud;
pub mod strings;

/// The wizard of the provider chosen; none while its wizard is not built.
pub(super) fn body(
    wizard: &Wizard,
    guard: &crate::widgets::key_guard::KeyGuard,
    resting: &super::Resting,
) -> AnyWidget {
    match wizard {
        Wizard::Bedrock(model) => bedrock::BedrockWizard {
            bedrock: model.clone(),
            guard: guard.clone(),
        }
        .into(),
        Wizard::Gcp(model) => gcp::GcpWizard {
            gcp: model.clone(),
            guard: guard.clone(),
        }
        .into(),
        Wizard::Gateway(model) => gateway::GatewayWizard {
            gateway: model.clone(),
            guard: guard.clone(),
            resting: resting.clone(),
        }
        .into(),
    }
}

/// `✓ <provider> configured`, green.
pub(super) fn configured(
    provider: Provider,
    look: &Look,
    strings: &'static strings::Strings,
) -> AnyWidget {
    let text = match provider {
        Provider::Gcp => strings.gcp_configured,
        Provider::Bedrock => strings.bedrock_configured,
        Provider::Gateway => strings.gateway_selected,
    };
    Text::rich([Span::styled(text, look.green.clone())]).into()
}

/// The footer's hint line: `  enter continue · esc back`, the keys bold and
/// the words dim.
pub(super) fn hint(
    strings: &'static strings::Strings,
    look: &Look,
    what: &'static str,
) -> Vec<Span> {
    vec![
        Span::new(strings.indent),
        Span::styled(strings.enter, look.key.clone()),
        Span::styled(what, look.dim.clone()),
        Span::styled(strings.separator, look.dim.clone()),
        Span::styled(strings.esc, look.key.clone()),
        Span::styled(strings.back, look.dim.clone()),
    ]
}

/// A paragraph of exactly the rows its lines take at the room's width
/// (`Paragraph::wrap`: the oracle's wrapping at blanks).
pub(super) fn sized<S>(cx: &mut Cx<'_, S>, lines: Vec<Vec<Span>>, width: usize) -> AnyWidget {
    let rows = rows_of(cx, &lines, width);
    SizedBox::new(
        None,
        Some(i32::try_from(rows).unwrap_or(i32::MAX)),
        Some(paragraph(lines)),
    )
    .into()
}

/// A wizard's layout (`render`): its text from the top, its footer (hints,
/// error) at the bottom of the room, which the text gives way to.
pub(super) fn frame<S>(
    cx: &mut Cx<'_, S>,
    body: AnyWidget,
    footer: Vec<Vec<Span>>,
    width: usize,
    height: i32,
) -> AnyWidget {
    let footer = (!footer.is_empty()).then(|| sized(cx, footer, width));
    frame_of(body, footer, height)
}

/// The same over a footer that is a widget (a note that wraps under its
/// own inset, lines after it).
pub(super) fn frame_of(body: AnyWidget, footer: Option<AnyWidget>, height: i32) -> AnyWidget {
    let mut column = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .main_axis_size(MainAxisSize::Max)
        .child(Expanded::new(1, body));
    if let Some(footer) = footer {
        column = column.child(footer);
    }
    SizedBox::new(None, Some(height), Some(column.into())).into()
}
