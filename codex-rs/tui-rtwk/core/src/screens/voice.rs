//! The thread-owned lifecycle of a local media peer and its backend voice call.

use std::fmt;

use codex_app_server_protocol as wire;

use crate::app::{AppFx, Now};
use crate::commands::VoiceControl;
use crate::ids::{ThreadId, Ticket};
use crate::rpc::{Address, RpcError};
use crate::screens::Kind;
use crate::threads::{
    Failure, ThreadFx,
    voice_transcript::{Page, ThreadEvent},
};

pub const NAME: &str = "voice";
pub const ID: &str = "S25";
pub const KIND: Kind = Kind::Sheet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(_: &mut Model, ev: Ev, _: &mut AppFx) {
    match ev {}
}

pub fn apply_intent(_: &mut Model, intent: Intent, _: &mut AppFx) {
    match intent {}
}

/// SDP can contain private ICE addresses and credentials.
#[derive(Clone, PartialEq, Eq)]
pub struct Sdp(String);

impl Sdp {
    pub fn new(text: String) -> Self {
        Self(text)
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for Sdp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Sdp").field(&"[redacted]").finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owner {
    pub thread: ThreadId,
    pub address: Address,
    pub call: Ticket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioCommand {
    pub owner: Owner,
    pub action: AudioAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioAction {
    Prepare,
    Connect(Sdp),
    SetMuted(bool),
    Sample,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioReport {
    pub owner: Owner,
    pub result: AudioResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioResult {
    Prepared(Result<Sdp, String>),
    Connected(Result<(), String>),
    Muted {
        muted: bool,
        result: Result<(), String>,
    },
    Sampled(Result<Peaks, String>),
    Closed,
}

/// Peak levels are consumed as meter glyphs; do not emit them in effect logs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Peaks(pub [u16; 2]);

impl fmt::Debug for Peaks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Peaks").field(&"[redacted]").finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    Start,
    SetMuted(bool),
    Stop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    Transcript(ThreadEvent),
    Started { session: Option<String> },
    Answer(Sdp),
    Error(String),
    Closed,
    Audio(AudioReport),
    Sample,
    Retired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Start { session: String, offer: Sdp },
    Stop,
    ReadTimeline { cursor: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Started,
    Stopped,
    Timeline(Page),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Start(Owner),
    Stop(Owner),
    ReadTimeline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Negotiating {
    accepted: bool,
    started: bool,
    connected: bool,
    answering: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Preparing,
    Negotiating(Negotiating),
    Active,
    Stopping {
        start_pending: bool,
        stop_pending: bool,
        accepted: bool,
        closed: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Call {
    owner: Owner,
    session: String,
    phase: Phase,
    muted: bool,
    pending_mute: Option<bool>,
    microphone: [u8; 6],
    speaker: [u8; 6],
    next_sample: Option<Now>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    call: Option<Call>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    pub connecting: bool,
    pub muted: bool,
    pub microphone: [u8; 6],
    pub speaker: [u8; 6],
}

impl ThreadPart {
    pub fn display(&self) -> Option<Display> {
        let call = self.call.as_ref()?;
        match call.phase {
            Phase::Preparing | Phase::Negotiating(_) | Phase::Active => Some(Display {
                connecting: call.phase != Phase::Active,
                muted: call.muted,
                microphone: call.microphone,
                speaker: call.speaker,
            }),
            Phase::Stopping { .. } => None,
        }
    }

    pub fn can_mute(&self) -> bool {
        self.call
            .as_ref()
            .is_some_and(|call| call.phase == Phase::Active)
    }

    pub fn control(&self, control: VoiceControl) -> Option<ThreadIntent> {
        match control {
            VoiceControl::Toggle if self.call.is_none() => Some(ThreadIntent::Start),
            VoiceControl::Toggle | VoiceControl::Stop if self.call.is_some() => {
                Some(ThreadIntent::Stop)
            }
            VoiceControl::Mute => self.call.as_ref().and_then(|call| {
                (call.phase == Phase::Active && call.pending_mute.is_none())
                    .then_some(ThreadIntent::SetMuted(!call.muted))
            }),
            VoiceControl::Toggle | VoiceControl::Stop => None,
        }
    }

    pub(crate) fn caption_session(&self, address: Address) -> Option<&str> {
        let call = self
            .call
            .as_ref()
            .filter(|call| call.owner.address == address)?;
        (!matches!(call.phase, Phase::Preparing)).then_some(call.session.as_str())
    }

    pub(crate) fn has_call(&self) -> bool {
        self.call.is_some()
    }

    pub(crate) fn sample_at(&self) -> Option<Now> {
        self.call.as_ref()?.next_sample
    }

    pub(crate) fn owns(&self, owner: &Owner) -> bool {
        self.call.as_ref().is_some_and(|call| &call.owner == owner)
    }

    pub(crate) fn retire(&mut self, fx: &mut ThreadFx<'_>) {
        if let Some(call) = self.call.take() {
            audio(&call, AudioAction::Close, fx);
        }
    }
}

pub(crate) fn apply_intent_on_thread(
    part: &mut ThreadPart,
    intent: ThreadIntent,
    fx: &mut ThreadFx<'_>,
) {
    match intent {
        ThreadIntent::Start if part.call.is_none() => {
            let (owner, session) = fx.voice_owner();
            let call = Call {
                owner,
                session,
                phase: Phase::Preparing,
                muted: false,
                pending_mute: None,
                microphone: [0; 6],
                speaker: [0; 6],
                next_sample: None,
            };
            audio(&call, AudioAction::Prepare, fx);
            part.call = Some(call);
        }
        ThreadIntent::Stop => stop(part, fx),
        ThreadIntent::SetMuted(muted) => {
            if let Some(call) = part.call.as_mut()
                && call.phase == Phase::Active
                && call.pending_mute.is_none()
                && call.muted != muted
            {
                call.pending_mute = Some(muted);
                audio(call, AudioAction::SetMuted(muted), fx);
            }
        }
        ThreadIntent::Start => {}
    }
}

fn audio(call: &Call, action: AudioAction, fx: &mut ThreadFx<'_>) {
    fx.voice_audio(AudioCommand {
        owner: call.owner.clone(),
        action,
    });
}

fn stop(part: &mut ThreadPart, fx: &mut ThreadFx<'_>) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    match call.phase {
        Phase::Preparing => part.retire(fx),
        Phase::Negotiating(_) | Phase::Active => {
            audio(call, AudioAction::Close, fx);
            let start_pending = matches!(
                call.phase,
                Phase::Negotiating(Negotiating {
                    accepted: false,
                    ..
                })
            );
            call.phase = Phase::Stopping {
                start_pending,
                stop_pending: !start_pending,
                accepted: false,
                closed: false,
            };
            call.next_sample = None;
            if !start_pending {
                fx.request(Ctx::Stop(call.owner.clone()), Command::Stop);
            }
        }
        Phase::Stopping {
            start_pending: false,
            stop_pending: false,
            accepted: false,
            closed: false,
        } => {
            call.phase = Phase::Stopping {
                start_pending: false,
                stop_pending: true,
                accepted: false,
                closed: false,
            };
            fx.request(Ctx::Stop(call.owner.clone()), Command::Stop);
        }
        Phase::Stopping { .. } => {}
    }
}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    match ev {
        ThreadEv::Started { session } => {
            if let Phase::Negotiating(ref mut setup) = call.phase
                && session.as_deref() == Some(call.session.as_str())
            {
                setup.started = true;
            }
        }
        ThreadEv::Answer(answer) => {
            if let Phase::Negotiating(ref mut setup) = call.phase
                && setup.started
                && !setup.answering
            {
                setup.answering = true;
                audio(call, AudioAction::Connect(answer), fx);
            }
        }
        ThreadEv::Closed => match &mut call.phase {
            Phase::Negotiating(_) | Phase::Active => part.retire(fx),
            Phase::Stopping {
                start_pending,
                stop_pending,
                closed,
                ..
            } => {
                if !*start_pending && !*stop_pending {
                    part.retire(fx);
                } else {
                    *closed = true;
                }
            }
            Phase::Preparing => {}
        },
        ThreadEv::Error(error) if matches!(call.phase, Phase::Negotiating(_) | Phase::Active) => {
            fx.voice_failure(Failure::Message(error));
            stop(part, fx);
        }
        ThreadEv::Error(_) => {}
        ThreadEv::Audio(report) if call.owner == report.owner => {
            answer_audio(part, report.result, fx)
        }
        ThreadEv::Audio(_) => {}
        ThreadEv::Sample if call.phase == Phase::Active => {
            let now = fx.facts().now();
            if call.next_sample.is_some_and(|at| now.reached(at)) {
                call.next_sample = None;
                audio(call, AudioAction::Sample, fx);
            }
        }
        ThreadEv::Sample => {}
        ThreadEv::Retired => part.retire(fx),
        ThreadEv::Transcript(_) => {} // Applied by the thread transcript.
    }
    activate(part, fx.facts().now());
}

fn answer_audio(part: &mut ThreadPart, result: AudioResult, fx: &mut ThreadFx<'_>) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    match (call.phase, result) {
        (Phase::Preparing, AudioResult::Prepared(Ok(offer))) => {
            call.phase = Phase::Negotiating(Negotiating::default());
            fx.request(
                Ctx::Start(call.owner.clone()),
                Command::Start {
                    session: call.session.clone(),
                    offer,
                },
            );
        }
        (Phase::Negotiating(mut setup), AudioResult::Connected(Ok(()))) if setup.answering => {
            setup.connected = true;
            call.phase = Phase::Negotiating(setup);
        }
        (
            Phase::Active,
            AudioResult::Muted {
                muted,
                result: Ok(()),
            },
        ) if call.pending_mute == Some(muted) => {
            call.pending_mute = None;
            call.muted = muted;
            if muted {
                call.microphone = [0; 6];
            }
        }
        (Phase::Active, AudioResult::Sampled(Ok(Peaks([microphone, speaker]))))
            if call.next_sample.is_none() =>
        {
            call.microphone.rotate_left(1);
            call.speaker.rotate_left(1);
            call.microphone[5] = if call.muted { 0 } else { intensity(microphone) };
            call.speaker[5] = intensity(speaker);
            call.next_sample = Some(fx.facts().now().plus_millis(1000));
        }
        (Phase::Preparing, AudioResult::Prepared(Err(error))) => {
            fx.voice_failure(Failure::Message(error));
            part.retire(fx);
        }
        (Phase::Negotiating(_), AudioResult::Connected(Err(error)))
        | (
            Phase::Active,
            AudioResult::Muted {
                result: Err(error), ..
            }
            | AudioResult::Sampled(Err(error)),
        ) => {
            fx.voice_failure(Failure::Message(error));
            stop(part, fx);
        }
        _ => {}
    }
}

fn intensity(peak: u16) -> u8 {
    // The peer reports a signed PCM peak as its magnitude (oracle audio_meter_intensity).
    (u32::from(peak.min(32768)) * 255 / 32768) as u8
}

fn activate(part: &mut ThreadPart, now: Now) {
    if let Some(call) = part.call.as_mut()
        && matches!(
            call.phase,
            Phase::Negotiating(Negotiating {
                accepted: true,
                started: true,
                connected: true,
                ..
            })
        )
    {
        call.phase = Phase::Active;
        call.next_sample = Some(now.plus_millis(1000));
    }
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    match (ctx, result, &mut call.phase) {
        (Ctx::Start(owner), Ok(Reply::Started), Phase::Negotiating(setup))
            if owner == call.owner =>
        {
            setup.accepted = true
        }
        (
            Ctx::Start(owner),
            Ok(Reply::Started),
            Phase::Stopping {
                start_pending,
                stop_pending,
                closed,
                ..
            },
        ) if owner == call.owner && *start_pending => {
            *start_pending = false;
            if *closed {
                part.retire(fx);
            } else {
                *stop_pending = true;
                fx.request(Ctx::Stop(owner), Command::Stop);
            }
        }
        (
            Ctx::Stop(owner),
            Ok(Reply::Stopped),
            Phase::Stopping {
                stop_pending,
                accepted,
                closed,
                ..
            },
        ) if owner == call.owner && *stop_pending => {
            *stop_pending = false;
            if *closed {
                part.retire(fx);
            } else {
                *accepted = true;
            }
        }
        (
            Ctx::Stop(owner),
            Err(error),
            Phase::Stopping {
                stop_pending,
                closed,
                ..
            },
        ) if owner == call.owner && *stop_pending => {
            *stop_pending = false;
            fx.voice_failure(Failure::from(error));
            if *closed {
                part.retire(fx);
            }
        }
        (Ctx::Start(owner), Err(error), _) if owner == call.owner => {
            fx.voice_failure(Failure::from(error));
            part.retire(fx);
        }
        _ => {}
    }
    activate(part, fx.facts().now());
}

fn ask(command: &Command, thread: &ThreadId) -> crate::threads::ThreadAsk {
    use crate::threads::ThreadAsk;
    match command {
        Command::Start { session, offer } => ThreadAsk::RealtimeStart {
            thread: thread.clone(),
            session: session.clone(),
            offer: offer.clone(),
        },
        Command::Stop => ThreadAsk::RealtimeStop {
            thread: thread.clone(),
        },
        Command::ReadTimeline { cursor } => ThreadAsk::RealtimeTimeline {
            thread: thread.clone(),
            cursor: cursor.clone(),
        },
    }
}

pub fn wire(command: &Command, thread: &ThreadId, id: wire::RequestId) -> wire::ClientRequest {
    ask(command, thread).wire(id)
}

pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    use crate::threads::ThreadReply;
    match (command, ask(command, &ThreadId::default()).decode(result)?) {
        (Command::Start { .. }, ThreadReply::Done) => Ok(Reply::Started),
        (Command::Stop, ThreadReply::Done) => Ok(Reply::Stopped),
        (Command::ReadTimeline { .. }, ThreadReply::RealtimeTimeline(page)) => {
            Ok(Reply::Timeline(page))
        }
        _ => Err(RpcError::Mismatch),
    }
}

pub(crate) fn project(notification: &wire::ServerNotification) -> Option<(ThreadId, ThreadEv)> {
    use wire::ServerNotification as Note;
    let (thread, ev) = match notification {
        Note::ThreadRealtimeStarted(note) => (
            &note.thread_id,
            ThreadEv::Started {
                session: note.realtime_session_id.clone(),
            },
        ),
        Note::ThreadRealtimeSdp(note) => (
            &note.thread_id,
            ThreadEv::Answer(Sdp::new(note.sdp.clone())),
        ),
        Note::ThreadRealtimeClosed(note) => (&note.thread_id, ThreadEv::Closed),
        Note::ThreadRealtimeError(note) => (&note.thread_id, ThreadEv::Error(note.message.clone())),
        _ => return None,
    };
    Some((ThreadId::new(thread), ev))
}

pub fn seed(_: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    Err(crate::screens::SeedError::NoSeeds)
}
