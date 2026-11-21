//! Google Cloud Vertex AI setup (`onboarding/gcp.rs`, `GcpState::render`):
//! the project, the location and the credential file in turn; the keys are
//! the field's (Enter submits it, the step's Esc goes one stage back); the
//! setup is asked of the host by the credentials' Enter and the step stays
//! "configuring" until it answers.

use std::rc::Rc;
use std::time::Duration;

use rtwk::AnyWidget;
use rtwk::binding::AppBinding;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::provider::read;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;
use codex_tui_core::screens::onboarding::auth::{self, provider};
use provider::gcp::{Error, Fields, Gcp, Stage};

use super::super::{Look, guarded_keys, width_of};
use super::{frame, hint, sized, strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::widgets::key_guard::KeyGuard;

/// The wizard over its model.
#[derive(Clone, PartialEq, Eq)]
pub struct GcpWizard {
    pub gcp: Gcp,
    pub guard: KeyGuard,
}

/// Widget state: the three fields' controllers and nodes (R14).
pub struct GcpState {
    project: TextEditingController,
    location: TextEditingController,
    credential: TextEditingController,
    /// The stages' fields', and the one that holds the focus while the
    /// setup runs (no field then; Esc and Ctrl+C still reach the step).
    nodes: [FocusNode; 4],
}

pub enum Msg {
    /// Enter in the field of the stage.
    Submit,
    /// The text changed: any key handled clears the error.
    Edited,
    Quit,
    /// The stage's field takes the focus once the frame mounted it.
    Focus,
}

impl StatefulWidget for GcpWizard {
    type State = GcpState;

    fn create_state(&self) -> GcpState {
        GcpState {
            project: TextEditingController::new(""),
            location: TextEditingController::new(""),
            credential: TextEditingController::new(""),
            nodes: [
                FocusNode::new(),
                FocusNode::new(),
                FocusNode::new(),
                FocusNode::new(),
            ],
        }
    }
}

impl_widget!(stateful eq GcpWizard);

impl GcpState {
    /// `addPostFrameCallback`: the field of the stage is mounted by then.
    fn focus_after_frame(cx: &mut Cx<'_, Self>) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::Focus));
        }
    }

    fn emit(cx: &mut Cx<'_, Self>, intent: provider::Intent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Onboarding(Intent::SignIn(auth::Intent::Provider(intent))),
        );
    }

    fn fields(&self) -> Fields {
        Fields {
            project: self.project.text().to_string(),
            location: self.location.text().to_string(),
            credential_file: self.credential.text().to_string(),
        }
    }
}

impl State for GcpState {
    type Widget = GcpWizard;
    type Msg = Msg;

    fn init_state(&mut self, w: &GcpWizard, cx: &mut Cx<'_, Self>) {
        // The wizard's fields take the quit chords and the guard like the
        // step's own (a `q` is text here: `should_suppress_printable_quit`).
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let now: Rc<dyn Fn() -> Duration> =
            Rc::new(move || clock.as_ref().map_or(Duration::ZERO, |clock| clock.now()));
        Self::focus_after_frame(cx);
        let send = cx.local_sender();
        for node in &self.nodes {
            let send = send.clone();
            node.set_on_key_event(Some(guarded_keys(
                w.guard.clone(),
                now.clone(),
                Rc::new(|| true),
                Rc::new(move || send(Msg::Quit)),
            )));
        }
    }

    fn did_update_widget(&mut self, old: &GcpWizard, w: &GcpWizard, cx: &mut Cx<'_, Self>) {
        if old.gcp.stage != w.gcp.stage {
            Self::focus_after_frame(cx);
        }
    }

    fn dispose(&mut self) {
        self.project.dispose();
        self.location.dispose();
        self.credential.dispose();
    }

    fn update(&mut self, msg: Msg, w: &GcpWizard, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Submit => Self::emit(
                cx,
                provider::Intent::Gcp(provider::gcp::Intent::Next(self.fields())),
            ),
            Msg::Edited => {
                if w.gcp.error.is_some() {
                    Self::emit(cx, provider::Intent::Edited);
                }
            }
            Msg::Focus => {
                let node = match w.gcp.stage {
                    Stage::Project => Some(&self.nodes[0]),
                    Stage::Location => Some(&self.nodes[1]),
                    Stage::Credentials => Some(&self.nodes[2]),
                    Stage::Configuring(_) => Some(&self.nodes[3]),
                };
                if let Some(node) = node {
                    node.request_focus();
                }
            }
            Msg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::Exit));
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &GcpWizard, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let tokens = Tokens::of(cx);
        let look = Look {
            dim: Style::new().attrs(rtwk::style::Attrs::DIM),
            key: tokens.bold(),
            cyan: tokens.info(),
            green: tokens.success(),
            red: tokens.error(),
        };
        let (guidance, field) = match w.gcp.stage {
            Stage::Project => (
                strings.gcp_project_guidance,
                Some((
                    "project",
                    strings.gcp_project_label,
                    strings.gcp_project_placeholder,
                    &self.project,
                    &self.nodes[0],
                )),
            ),
            Stage::Location => (
                strings.gcp_location_guidance,
                Some((
                    "location",
                    strings.gcp_location_label,
                    strings.gcp_location_placeholder,
                    &self.location,
                    &self.nodes[1],
                )),
            ),
            Stage::Credentials => (
                strings.gcp_credentials_guidance,
                Some((
                    "credentials",
                    strings.gcp_credentials_label,
                    "",
                    &self.credential,
                    &self.nodes[2],
                )),
            ),
            Stage::Configuring(_) => (strings.gcp_configuring, None),
        };
        let [mark, title] = strings.gcp_title;
        let mut top = vec![
            vec![Span::new(mark), Span::new(title).bold()],
            Vec::new(),
            vec![Span::new(guidance)],
        ];
        if field.is_some() {
            top.push(Vec::new());
        }
        let idle: AnyWidget = Focus::new(SizedBox::shrink())
            .focus_node(self.nodes[3].clone())
            .into();
        let field_row: Option<AnyWidget> =
            field.map(|(key, label, placeholder, controller, node)| {
                let input = TextField::new()
                    .controller(controller.clone())
                    .focus_node(node.clone())
                    .autofocus(true)
                    .placeholder(placeholder)
                    .placeholder_style(look.dim.clone())
                    .style(look.cyan.clone())
                    .focused_style(look.cyan.clone())
                    .selection_style(Style::new())
                    .on_changed(cx.map(|_| Msg::Edited))
                    .on_submitted(cx.map(|_| Msg::Submit));
                // The field is its stage's: a new stage mounts a new field,
                // which takes the focus (`autofocus`).
                let row: AnyWidget = Row::new()
                    .child(Text::new(label).wrap(TextWrap::None))
                    .child(Expanded::new(1, input))
                    .into();
                row.with_key(key)
            });
        let adc = if w.gcp.stage == Stage::Credentials {
            vec![
                Vec::new(),
                vec![Span::styled(strings.gcp_adc_hint, look.dim.clone())],
                vec![Span::styled(strings.gcp_adc_command, look.cyan.clone())],
            ]
        } else {
            Vec::new()
        };
        let mut footer = Vec::new();
        if !matches!(w.gcp.stage, Stage::Configuring(_)) {
            footer.push(hint(
                strings,
                &look,
                if w.gcp.stage == Stage::Credentials {
                    strings.to_set_up
                } else {
                    strings.to_continue
                },
            ));
        }
        if let Some(error) = &w.gcp.error {
            footer.push(Vec::new());
            let message = match error {
                Error::EnterProject => strings.gcp_project_required.to_owned(),
                Error::Server(message) => message.clone(),
            };
            footer.push(vec![Span::styled(message, look.red)]);
        }
        LayoutBuilder::new(move |cx, constraints| {
            let width = width_of(constraints.max_width);
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(sized(cx, top.clone(), width));
            column = column.child(field_row.clone().unwrap_or_else(|| idle.clone()));
            if !adc.is_empty() {
                column = column.child(sized(cx, adc.clone(), width));
            }
            frame(
                cx,
                column.into(),
                footer.clone(),
                width,
                constraints.max_height,
            )
        })
        .into()
    }
}
