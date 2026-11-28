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
    voice_transcript::{History, Legacy, Page, ThreadEvent},
};

pub mod settings;
pub use settings::Intent;

pub const NAME: &str = "voice";
pub const ID: &str = "S25";
pub const KIND: Kind = Kind::Sheet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    ordered_legacy_publisher: Option<Address>,
    pub settings: settings::Model,
}

impl Model {
    pub(crate) fn legacy_captions_from(&self, address: Address) -> bool {
        self.ordered_legacy_publisher == Some(address)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    OrderedLegacyPublisher(Address),
}

pub fn apply(model: &mut Model, ev: Ev, _: &mut AppFx) {
    match ev {
        Ev::OrderedLegacyPublisher(address) => model.ordered_legacy_publisher = Some(address),
    }
}

// The app captures the active thread's physical address before issuing a settings request.
pub fn apply_intent(_: &mut Model, _: Intent, _: &mut AppFx) {}

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
    pub attempt: Attempt,
}

impl Owner {
    pub fn same_call(&self, other: &Self) -> bool {
        self.thread == other.thread && self.address == other.address && self.call == other.call
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attempt {
    Initial,
    Retry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioCommand {
    pub owner: Owner,
    pub action: AudioAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioAction {
    ObserveLock,
    Prepare,
    PrepareRetry { muted: bool },
    Connect(Sdp),
    SetMuted(bool),
    Sample,
    ReleasePeer,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioReport {
    pub owner: Owner,
    pub result: AudioResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioResult {
    LockStopped(LockStop),
    Prepared(Result<Sdp, String>),
    Connected(Result<(), ConnectFailure>),
    Muted {
        muted: bool,
        result: Result<(), String>,
    },
    Sampled(Result<Peaks, String>),
    PeerReleased,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectFailure {
    TimedOut(String),
    Other(String),
}

impl ConnectFailure {
    fn message(self) -> String {
        match self {
            Self::TimedOut(message) | Self::Other(message) => message,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockStop {
    Locked,
    MonitorFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Requested,
    TransportClosed,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryReason {
    Timeout,
    TransportClosed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Settings(settings::Feedback),
    Retry(RetryReason),
    StartupTransportClosed,
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
    Closed(CloseReason),
    Audio(AudioReport),
    LocalHostLost,
    Sample,
    Retired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Start {
        session: String,
        offer: Sdp,
        voice: Option<settings::RealtimeVoice>,
    },
    Stop,
    ReadTimeline {
        cursor: Option<String>,
    },
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
    connected: bool,
    answering: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Preparing,
    Negotiating(Negotiating),
    Active,
    Stopping(Cleanup),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cleanup {
    backend: BackendRelease,
    terminal: Terminal,
    next: AfterCleanup,
}

/// The thread-only terminal notification and the exact request receipt can arrive in either order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendRelease {
    AwaitStart { closed: bool },
    AwaitStop { closed: bool },
    AwaitClosed,
    StopRefused,
    Released,
}

impl BackendRelease {
    fn closed(self) -> bool {
        matches!(
            self,
            Self::AwaitStart { closed: true } | Self::AwaitStop { closed: true } | Self::Released
        )
    }

    fn backend_closed(self) -> Self {
        match self {
            Self::AwaitStart { .. } => Self::AwaitStart { closed: true },
            Self::AwaitStop { .. } => Self::AwaitStop { closed: true },
            _ => Self::Released,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Terminal {
    Any,
    Requested,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AfterCleanup {
    Stop,
    Retry { peer_released: bool },
}

impl Phase {
    fn backend_closed(self) -> bool {
        matches!(self, Self::Stopping(cleanup) if cleanup.backend.closed())
    }

    fn accepts_close(self, reason: CloseReason) -> bool {
        match self {
            Self::Preparing => false,
            Self::Stopping(cleanup) if cleanup.terminal == Terminal::Requested => {
                reason == CloseReason::Requested
            }
            _ => true,
        }
    }

    fn waiting_to_retry(self) -> bool {
        matches!(
            self,
            Self::Stopping(Cleanup {
                next: AfterCleanup::Retry { .. },
                ..
            })
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Captions {
    Canonical,
    Legacy(Legacy),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Call {
    owner: Owner,
    session: String,
    voice: Option<settings::RealtimeVoice>,
    phase: Phase,
    started: bool,
    captions: Captions,
    muted: bool,
    pending_mute: Option<bool>,
    microphone: [u8; 6],
    speaker: [u8; 6],
    next_sample: Option<Now>,
}

impl Call {
    fn accepts_audio(&self, report: &AudioReport) -> bool {
        self.owner == report.owner
            || (matches!(report.result, AudioResult::LockStopped(_))
                && self.owner.same_call(&report.owner))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    call: Option<Call>,
    history: History,
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
            Phase::Stopping(_) => None,
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

    pub(crate) fn history(&self) -> History {
        self.history
    }

    pub(crate) fn set_history(&mut self, history: History) {
        self.history = history;
    }

    pub(crate) fn uses_legacy_captions(&self) -> bool {
        self.call
            .as_ref()
            .is_some_and(|call| matches!(call.captions, Captions::Legacy(_)))
    }

    pub(crate) fn captions_open(&self) -> bool {
        self.call
            .as_ref()
            .is_some_and(|call| call.phase != Phase::Preparing && !call.phase.backend_closed())
    }

    pub(crate) fn caption_session(&self, address: Address) -> Option<&str> {
        let call = self.call.as_ref()?;
        (call.owner.address == address
            && matches!(call.captions, Captions::Canonical)
            && call.phase != Phase::Preparing
            && !call.phase.backend_closed())
        .then_some(call.session.as_str())
    }

    pub(crate) fn legacy_captions(&mut self, address: Address) -> Option<(&str, &mut Legacy)> {
        let call = self.call.as_mut()?;
        if call.owner.address != address
            || !call.started
            || call.phase == Phase::Preparing
            || call.phase.backend_closed()
        {
            return None;
        }
        let Captions::Legacy(legacy) = &mut call.captions else {
            return None;
        };
        legacy.ordered().then_some((call.session.as_str(), legacy))
    }

    pub(crate) fn lose_legacy_order(&mut self) -> bool {
        if let Some(call) = self.call.as_mut()
            && let Captions::Legacy(legacy) = &mut call.captions
        {
            legacy.lose_order();
            return true;
        }
        false
    }

    pub(crate) fn has_call(&self) -> bool {
        self.call.is_some()
    }

    pub(crate) fn owner(&self) -> Option<&Owner> {
        self.call.as_ref().map(|call| &call.owner)
    }

    pub(crate) fn sample_at(&self) -> Option<Now> {
        self.call.as_ref()?.next_sample
    }

    pub(crate) fn owns(&self, owner: &Owner) -> bool {
        self.call.as_ref().is_some_and(|call| &call.owner == owner)
    }

    pub(crate) fn accepts_audio(&self, report: &AudioReport) -> bool {
        self.call
            .as_ref()
            .is_some_and(|call| call.accepts_audio(report))
    }

    pub(crate) fn accepts_close(&self, reason: CloseReason) -> bool {
        self.call
            .as_ref()
            .is_some_and(|call| call.phase.accepts_close(reason))
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
                voice: fx.realtime_voice(),
                phase: Phase::Preparing,
                started: false,
                captions: if part.history == History::Legacy && fx.ordered_legacy_voice() {
                    Captions::Legacy(Legacy::new())
                } else {
                    Captions::Canonical
                },
                muted: false,
                pending_mute: None,
                microphone: [0; 6],
                speaker: [0; 6],
                next_sample: None,
            };
            audio(&call, AudioAction::ObserveLock, fx);
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
    if call.owner.address != fx.address() {
        part.retire(fx);
        return;
    }
    match call.phase {
        Phase::Preparing => part.retire(fx),
        Phase::Negotiating(_) | Phase::Active => begin_cleanup(call, None, fx),
        Phase::Stopping(ref mut cleanup) => {
            if matches!(cleanup.next, AfterCleanup::Retry { .. }) {
                cleanup.next = AfterCleanup::Stop;
                // Keep the backend barrier, even if the user cancels after local peer release.
                if cleanup.backend != BackendRelease::Released {
                    audio(call, AudioAction::Close, fx);
                }
            }
            if let Phase::Stopping(cleanup) = &mut call.phase
                && cleanup.backend == BackendRelease::StopRefused
            {
                cleanup.backend = BackendRelease::AwaitStop { closed: false };
                fx.request(Ctx::Stop(call.owner.clone()), Command::Stop);
            }
        }
    }
    advance_cleanup(part, fx);
}

fn begin_cleanup(call: &mut Call, retry: Option<RetryReason>, fx: &mut ThreadFx<'_>) {
    let closed = retry == Some(RetryReason::TransportClosed);
    let start_pending = matches!(
        call.phase,
        Phase::Negotiating(Negotiating {
            accepted: false,
            ..
        })
    );
    let terminal = if retry == Some(RetryReason::Timeout) {
        Terminal::Requested
    } else {
        Terminal::Any
    };
    let next = if let Some(reason) = retry {
        audio(call, AudioAction::ReleasePeer, fx);
        fx.voice_note(Note::Retry(reason));
        AfterCleanup::Retry {
            peer_released: false,
        }
    } else {
        audio(call, AudioAction::Close, fx);
        AfterCleanup::Stop
    };
    let backend = if start_pending {
        BackendRelease::AwaitStart { closed }
    } else if closed {
        BackendRelease::Released
    } else {
        fx.request(Ctx::Stop(call.owner.clone()), Command::Stop);
        BackendRelease::AwaitStop { closed: false }
    };
    call.phase = Phase::Stopping(Cleanup {
        backend,
        terminal,
        next,
    });
    call.next_sample = None;
}

fn advance_cleanup(part: &mut ThreadPart, fx: &mut ThreadFx<'_>) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    let Phase::Stopping(Cleanup {
        backend: BackendRelease::Released,
        next,
        ..
    }) = call.phase
    else {
        return;
    };
    match next {
        AfterCleanup::Stop => part.retire(fx),
        AfterCleanup::Retry {
            peer_released: true,
        } if call.owner.address == fx.address() => {
            call.owner.attempt = Attempt::Retry;
            call.session.push_str(":retry");
            call.phase = Phase::Preparing;
            call.started = false;
            if let Captions::Legacy(legacy) = &mut call.captions
                && legacy.ordered()
            {
                *legacy = Legacy::new();
            }
            audio(call, AudioAction::PrepareRetry { muted: call.muted }, fx);
        }
        AfterCleanup::Retry {
            peer_released: true,
        } => part.retire(fx),
        AfterCleanup::Retry {
            peer_released: false,
        } => {}
    }
}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    match ev {
        ThreadEv::Started { session } => {
            if matches!(call.phase, Phase::Negotiating(_))
                && session.as_deref() == Some(call.session.as_str())
            {
                call.started = true;
            }
        }
        ThreadEv::Answer(answer) => {
            if let Phase::Negotiating(ref mut setup) = call.phase
                && call.started
                && !setup.answering
            {
                setup.answering = true;
                audio(call, AudioAction::Connect(answer), fx);
            }
        }
        ThreadEv::Closed(reason) if call.phase.accepts_close(reason) => match &mut call.phase {
            Phase::Negotiating(_)
                if reason == CloseReason::TransportClosed
                    && call.owner.attempt == Attempt::Initial =>
            {
                begin_cleanup(call, Some(RetryReason::TransportClosed), fx);
            }
            Phase::Negotiating(_) | Phase::Active => {
                if call.phase != Phase::Active && reason == CloseReason::TransportClosed {
                    fx.voice_note(Note::StartupTransportClosed);
                }
                part.retire(fx);
            }
            Phase::Stopping(cleanup) => cleanup.backend = cleanup.backend.backend_closed(),
            Phase::Preparing => {}
        },
        ThreadEv::Closed(_) => {}
        ThreadEv::Error(error)
            if matches!(call.phase, Phase::Negotiating(_) | Phase::Active)
                || call.phase.waiting_to_retry() =>
        {
            fx.voice_failure(Failure::Message(error));
            stop(part, fx);
        }
        ThreadEv::Error(_) => {}
        ThreadEv::Audio(report) if call.accepts_audio(&report) => {
            answer_audio(part, report.result, fx)
        }
        ThreadEv::Audio(_) => {}
        ThreadEv::LocalHostLost
            if !matches!(call.phase, Phase::Stopping(_)) || call.phase.waiting_to_retry() =>
        {
            fx.voice_failure(Failure::VoiceHostLost);
            stop(part, fx);
        }
        ThreadEv::LocalHostLost => {}
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
    advance_cleanup(part, fx);
    activate(part, fx.facts().now());
}

fn answer_audio(part: &mut ThreadPart, result: AudioResult, fx: &mut ThreadFx<'_>) {
    let Some(call) = part.call.as_mut() else {
        return;
    };
    match (call.phase, result) {
        (phase, AudioResult::LockStopped(reason))
            if !matches!(phase, Phase::Stopping(_)) || phase.waiting_to_retry() =>
        {
            fx.voice_failure(Failure::VoiceLock(reason));
            stop(part, fx);
        }
        (Phase::Stopping(mut cleanup), AudioResult::PeerReleased)
            if matches!(cleanup.next, AfterCleanup::Retry { .. }) =>
        {
            cleanup.next = AfterCleanup::Retry {
                peer_released: true,
            };
            call.phase = Phase::Stopping(cleanup);
        }
        (Phase::Preparing, AudioResult::Prepared(Ok(offer))) => {
            call.phase = Phase::Negotiating(Negotiating::default());
            fx.request(
                Ctx::Start(call.owner.clone()),
                Command::Start {
                    session: call.session.clone(),
                    offer,
                    voice: call.voice,
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
        (Phase::Negotiating(setup), AudioResult::Connected(Err(ConnectFailure::TimedOut(_))))
            if setup.answering && call.owner.attempt == Attempt::Initial =>
        {
            begin_cleanup(call, Some(RetryReason::Timeout), fx);
        }
        (Phase::Negotiating(setup), AudioResult::Connected(Err(error))) if setup.answering => {
            fx.voice_failure(Failure::Message(error.message()));
            stop(part, fx);
        }
        (
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
        && call.started
        && matches!(
            call.phase,
            Phase::Negotiating(Negotiating {
                accepted: true,
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
        (Ctx::Start(owner), Ok(Reply::Started), Phase::Stopping(cleanup))
            if owner == call.owner =>
        {
            if let BackendRelease::AwaitStart { closed } = cleanup.backend {
                cleanup.backend = if closed {
                    BackendRelease::Released
                } else {
                    fx.request(Ctx::Stop(owner), Command::Stop);
                    BackendRelease::AwaitStop { closed: false }
                };
            }
        }
        (Ctx::Stop(owner), Ok(Reply::Stopped), Phase::Stopping(cleanup)) if owner == call.owner => {
            if let BackendRelease::AwaitStop { closed } = cleanup.backend {
                cleanup.backend = if closed {
                    BackendRelease::Released
                } else {
                    BackendRelease::AwaitClosed
                };
            }
        }
        (Ctx::Stop(owner), Err(error), Phase::Stopping(cleanup)) if owner == call.owner => {
            if let BackendRelease::AwaitStop { closed } = cleanup.backend {
                let cancelled_retry = matches!(cleanup.next, AfterCleanup::Retry { .. });
                cleanup.next = AfterCleanup::Stop;
                cleanup.backend = if closed {
                    BackendRelease::Released
                } else {
                    BackendRelease::StopRefused
                };
                if cancelled_retry && !closed {
                    audio(call, AudioAction::Close, fx);
                }
                fx.voice_failure(Failure::from(error));
            }
        }
        (Ctx::Start(owner), Err(error), _) if owner == call.owner => {
            fx.voice_failure(Failure::from(error));
            part.retire(fx);
        }
        _ => {}
    }
    advance_cleanup(part, fx);
    activate(part, fx.facts().now());
}

fn ask(command: &Command, thread: &ThreadId) -> crate::threads::ThreadAsk {
    use crate::threads::ThreadAsk;
    match command {
        Command::Start {
            session,
            offer,
            voice,
        } => ThreadAsk::RealtimeStart {
            thread: thread.clone(),
            session: session.clone(),
            offer: offer.clone(),
            voice: *voice,
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
        Note::ThreadRealtimeClosed(note) => (
            &note.thread_id,
            ThreadEv::Closed(match note.reason.as_deref() {
                Some("requested") => CloseReason::Requested,
                Some("transport_closed") => CloseReason::TransportClosed,
                _ => CloseReason::Other,
            }),
        ),
        Note::ThreadRealtimeError(note) => (&note.thread_id, ThreadEv::Error(note.message.clone())),
        _ => return None,
    };
    Some((ThreadId::new(thread), ev))
}

pub fn seed(_: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    Err(crate::screens::SeedError::NoSeeds)
}
