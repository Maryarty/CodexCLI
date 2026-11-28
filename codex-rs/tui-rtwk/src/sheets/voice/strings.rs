//! The strings of `voice` (S25): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;
use std::num::NonZeroU16;

use crate::widgets::picker::list::{Role, Span};
use codex_tui_core::screens::voice::settings::{AudioOutcome, DeviceKind, Feedback, Problem};

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    pub voice: &'static str,
    pub voice_prefix: &'static str,
    pub active_marker: &'static str,
    pub quiet_marker: &'static str,
    pub control_gap: &'static str,
    pub connecting: &'static str,
    pub listening: &'static str,
    pub muted: &'static str,
    pub mute: &'static str,
    pub unmute: &'static str,
    pub mute_command: &'static str,
    pub stop_command: &'static str,
    pub microphone: &'static str,
    pub speaker: &'static str,
    pub settings: &'static str,
    pub set_sound: &'static str,
    pub select_voice_action: &'static str,
    pub select_voice: &'static str,
    pub sound_devices: &'static str,
    pub input_device: &'static str,
    pub output_device: &'static str,
    pub input_channels: &'static str,
    pub system_default: &'static str,
    pub duplicate_device: &'static str,
    pub back: &'static str,
    pub applies_to_next: &'static str,
    pub mix_channels: &'static str,
    pub at_least_one: &'static str,
    pub channel_hint: &'static str,
    pub pending: &'static str,
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "voice and realtime strip",
    // bottom_pane/voice_strip.rs
    voice: "voice",
    voice_prefix: "voice ",
    active_marker: "●",
    quiet_marker: "◌",
    control_gap: "   ",
    connecting: " connecting",
    listening: " listen",
    muted: " muted",
    mute: " mute",
    unmute: " unmute",
    mute_command: "/voice mute",
    stop_command: "/voice stop",
    microphone: "  mic ",
    speaker: "  codex ",
    // chatwidget/realtime_settings.rs
    settings: "Voice settings",
    set_sound: "Set sound devices",
    select_voice_action: "Choose a voice",
    select_voice: "Select voice",
    sound_devices: "Sound devices",
    input_device: "Input device",
    output_device: "Output device",
    input_channels: "Input channels",
    system_default: "System default",
    duplicate_device: "Identical device names; use System default.",
    back: "Back",
    applies_to_next: "Applies to your next voice conversation.",
    mix_channels: "Select the input channels to mix. Select at least one.",
    at_least_one: "Select at least one input channel.",
    channel_hint: "space toggle · enter save · esc cancel",
    pending: "Checking voice settings…",
};

pub(super) fn device_label(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Input => EN.input_device,
        DeviceKind::Output => EN.output_device,
    }
}

pub(super) fn microphone(name: &str) -> String {
    format!("Microphone: {name}")
}

pub(super) fn input(channel: NonZeroU16) -> String {
    format!("Input {channel}")
}

pub(super) fn describe_channels(channels: Option<&[NonZeroU16]>) -> String {
    channels.map_or_else(
        || "All channels (mixed)".into(),
        |channels| {
            format!(
                "Inputs {}",
                channels
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    )
}

pub(crate) fn feedback(feedback: &Feedback) -> Span {
    match feedback {
        Feedback::VoiceSaved(voice) => Span::new(
            format!("Voice set to {}. {}", voice.wire_name(), EN.applies_to_next),
            Role::Dim,
        ),
        Feedback::VoiceOverridden(reason) if reason.is_empty() => Span::new(
            "Voice preference was saved but is overridden by another configuration layer.",
            Role::Danger,
        ),
        Feedback::VoiceOverridden(reason) => Span::new(
            format!("Voice preference was saved but not applied: {reason}"),
            Role::Danger,
        ),
        Feedback::AudioSaved(AudioOutcome::Applied) => Span::new(
            format!("Audio setting saved. {}", EN.applies_to_next),
            Role::Dim,
        ),
        Feedback::AudioSaved(AudioOutcome::Overridden) => Span::new(
            "Audio setting was saved but is overridden by another configuration layer.",
            Role::Danger,
        ),
        Feedback::AudioSaved(AudioOutcome::MicrophoneChannelOverridden) => Span::new(
            "Input device saved, but the input channel is overridden by another configuration layer. Update that override before starting voice.",
            Role::Danger,
        ),
        Feedback::Failed(error) => Span::new(problem(error), Role::Danger),
    }
}

fn problem(problem: &Problem) -> String {
    match problem {
        Problem::ConnectionLost => "The voice settings connection was lost.".into(),
        Problem::Catalog(error) => format!("Failed to list voices: {error}"),
        Problem::ReadVoice(error) => format!("Failed to read voice settings: {error}"),
        Problem::ReadAudio(error) => format!("Failed to read audio settings: {error}"),
        Problem::ListDevices => {
            "Could not list audio devices. Check your audio device and voice package.".into()
        }
        Problem::WriteVoice(error) => format!("Failed to save voice: {error}"),
        Problem::ReadSavedVoice(error) => {
            format!("Voice preference was saved, but effective settings could not be read: {error}")
        }
        Problem::WriteAudio(error) => format!("Failed to save audio setting: {error}"),
        Problem::ReadSavedAudio(error) => {
            format!("Audio setting was saved, but effective settings could not be read: {error}")
        }
        Problem::InputChanged => {
            "The input device changed. Reopen the device picker to select its channels.".into()
        }
        Problem::UnmatchedAnswer => "The voice settings response could not be verified.".into(),
    }
}

impl Strings {
    pub fn lock_stop(&self, reason: codex_tui_core::screens::voice::LockStop) -> &'static str {
        use codex_tui_core::screens::voice::LockStop;
        match reason {
            LockStop::Locked => "Voice stopped because your session locked.",
            LockStop::MonitorFailed => {
                "Voice conversation stopped because session lock monitoring failed."
            }
        }
    }
}

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}
