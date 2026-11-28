//! The issued workspace of a voice settings request, and its last verified facts.

use std::collections::BTreeMap;
use std::num::NonZeroU16;
use std::rc::Rc;

pub use codex_protocol::protocol::RealtimeVoice;

use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::Address;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub thread: ThreadId,
    pub address: Address,
    pub cwd: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    Input,
    Output,
}

impl DeviceKind {
    fn index(self) -> usize {
        usize::from(matches!(self, Self::Output))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub name: String,
    pub channels: u16,
    pub is_default: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Audio {
    pub microphone: Option<String>,
    pub speaker: Option<String>,
    pub channels: Option<Vec<NonZeroU16>>,
}

impl Audio {
    pub fn device(&self, kind: DeviceKind) -> Option<&str> {
        match kind {
            DeviceKind::Input => self.microphone.as_deref(),
            DeviceKind::Output => self.speaker.as_deref(),
        }
    }

    pub fn selection(&self, kind: DeviceKind, device: &Device) -> bool {
        self.device(kind)
            .map_or(device.is_default, |name| name == device.name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    pub voices: Vec<RealtimeVoice>,
    pub current: Option<RealtimeVoice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Voice(RealtimeVoice),
    Device {
        kind: DeviceKind,
        name: Option<String>,
    },
    InputChannels {
        microphone: Option<String>,
        channels: Option<Vec<NonZeroU16>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    ReadCatalog,
    ReadAudio,
    ListDevices(DeviceKind),
    Persist(Change),
}

impl Operation {
    pub fn local(&self) -> bool {
        !matches!(self, Self::ReadCatalog | Self::Persist(Change::Voice(_)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Read {
        thread: ThreadId,
        operation: Operation,
    },
    Persist {
        origin: Origin,
        change: Change,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub ticket: Ticket,
    pub origin: Origin,
    pub operation: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub request: Request,
    pub result: Result<Answer, Problem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Catalog(Catalog),
    Audio(Audio),
    Devices {
        kind: DeviceKind,
        devices: Vec<Device>,
    },
    VoiceSaved {
        catalog: Catalog,
        warning: Option<String>,
    },
    AudioSaved {
        audio: Audio,
        outcome: AudioOutcome,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioOutcome {
    Applied,
    Overridden,
    MicrophoneChannelOverridden,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    ConnectionLost,
    Catalog(String),
    ReadVoice(String),
    ReadAudio(String),
    ListDevices,
    WriteVoice(String),
    ReadSavedVoice(String),
    WriteAudio(String),
    ReadSavedAudio(String),
    InputChanged,
    UnmatchedAnswer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feedback {
    VoiceSaved(RealtimeVoice),
    VoiceOverridden(String),
    AudioSaved(AudioOutcome),
    Failed(Problem),
}

impl Feedback {
    pub fn successful(&self) -> bool {
        matches!(
            self,
            Self::VoiceSaved(_) | Self::AudioSaved(AudioOutcome::Applied)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub origin: Origin,
    pub catalog: Option<Catalog>,
    pub audio: Option<Audio>,
    devices: [Option<Vec<Device>>; 2],
    pub feedback: Option<Feedback>,
}

impl Snapshot {
    pub fn devices(&self, kind: DeviceKind) -> Option<&[Device]> {
        self.devices[kind.index()].as_deref()
    }

    fn supports(&self, change: &Change) -> bool {
        match change {
            Change::Voice(voice) => self
                .catalog
                .as_ref()
                .is_some_and(|catalog| catalog.voices.contains(voice)),
            Change::Device { kind, name } => self.devices(*kind).is_some_and(|devices| {
                name.as_ref().is_none_or(|name| {
                    devices.iter().filter(|device| &device.name == name).count() == 1
                })
            }),
            Change::InputChannels {
                microphone,
                channels,
            } => self.audio.as_ref().is_some_and(|audio| {
                audio.microphone == *microphone
                    && self.devices(DeviceKind::Input).is_some_and(|devices| {
                        let mut selected = devices
                            .iter()
                            .filter(|device| audio.selection(DeviceKind::Input, device));
                        let Some(device) = selected.next() else {
                            return false;
                        };
                        selected.next().is_none()
                            && device.channels >= 3
                            && channels.as_ref().is_none_or(|channels| {
                                !channels.is_empty()
                                    && channels
                                        .iter()
                                        .all(|channel| channel.get() <= device.channels)
                                    && channels.is_sorted_by(|left, right| left < right)
                            })
                    })
            }),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    snapshot: Option<Rc<Snapshot>>,
    read: Option<Request>,
    write: Option<Request>,
    confirmed: Rc<VoiceByWorkspace>,
}

type VoiceByWorkspace = BTreeMap<(BackendId, Option<String>), (Address, Option<RealtimeVoice>)>;

impl Model {
    pub fn snapshot(&self, thread: &ThreadId) -> Option<&Rc<Snapshot>> {
        self.snapshot
            .as_ref()
            .filter(|snapshot| snapshot.origin.thread == *thread)
    }

    pub fn pending(&self, thread: &ThreadId) -> bool {
        [&self.read, &self.write].iter().any(|request| {
            request
                .as_ref()
                .is_some_and(|request| request.origin.thread == *thread)
        })
    }

    pub(crate) fn voice(&self, address: Address, cwd: Option<&String>) -> Option<RealtimeVoice> {
        self.confirmed
            .get(&(address.backend, cwd.cloned()))
            .and_then(|(seen, voice)| (*seen == address).then_some(*voice).flatten())
    }

    pub(crate) fn begin(
        &mut self,
        ticket: Ticket,
        origin: Origin,
        intent: Intent,
    ) -> Option<Request> {
        let operation = match intent {
            Intent::Read { thread, operation }
                if thread == origin.thread && !matches!(&operation, Operation::Persist(_)) =>
            {
                operation
            }
            Intent::Persist {
                origin: asked,
                change,
            } if asked == origin
                && self.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.origin == origin && snapshot.supports(&change)
                }) =>
            {
                Operation::Persist(change)
            }
            _ => return None,
        };
        if self.write.is_some()
            || self
                .read
                .as_ref()
                .is_some_and(|reader| reader.origin == origin && reader.operation == operation)
        {
            return None;
        }
        let request = Request {
            ticket,
            origin,
            operation,
        };
        if self
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.origin != request.origin)
        {
            self.snapshot = Some(Rc::new(Snapshot {
                origin: request.origin.clone(),
                catalog: None,
                audio: None,
                devices: [None, None],
                feedback: None,
            }));
        }
        if let Some(snapshot) = &mut self.snapshot {
            Rc::make_mut(snapshot).feedback = None;
        }
        if matches!(request.operation, Operation::Persist(_)) {
            self.read = None;
            self.write = Some(request.clone());
        } else {
            self.read = Some(request.clone());
        }
        Some(request)
    }

    pub(crate) fn issued(&self, request: &Request) -> bool {
        self.read.as_ref() == Some(request) || self.write.as_ref() == Some(request)
    }

    pub(crate) fn finish(&mut self, report: Report) -> Option<Feedback> {
        let Report { request, result } = report;
        if self.read.as_ref() == Some(&request) {
            self.read = None;
        } else if self.write.as_ref() == Some(&request) {
            self.write = None;
        } else {
            return None;
        }
        let snapshot = self
            .snapshot
            .as_mut()
            .filter(|seen| seen.origin == request.origin)?;
        let snapshot = Rc::make_mut(snapshot);
        let feedback = match (request.operation, result) {
            (Operation::ReadCatalog, Ok(Answer::Catalog(catalog))) => {
                Rc::make_mut(&mut self.confirmed).insert(
                    (request.origin.address.backend, request.origin.cwd),
                    (request.origin.address, catalog.current),
                );
                snapshot.catalog = Some(catalog);
                return None;
            }
            (Operation::ReadAudio, Ok(Answer::Audio(audio))) => {
                snapshot.audio = Some(audio);
                return None;
            }
            (Operation::ListDevices(asked), Ok(Answer::Devices { kind, devices }))
                if asked == kind =>
            {
                snapshot.devices[kind.index()] = Some(devices);
                return None;
            }
            (
                Operation::Persist(Change::Voice(voice)),
                Ok(Answer::VoiceSaved { catalog, warning }),
            ) => {
                Rc::make_mut(&mut self.confirmed).insert(
                    (request.origin.address.backend, request.origin.cwd),
                    (request.origin.address, catalog.current),
                );
                let applied = catalog.current == Some(voice);
                snapshot.catalog = Some(catalog);
                if applied {
                    Feedback::VoiceSaved(voice)
                } else {
                    Feedback::VoiceOverridden(warning.unwrap_or_default())
                }
            }
            (
                Operation::Persist(Change::Device { .. } | Change::InputChannels { .. }),
                Ok(Answer::AudioSaved { audio, outcome }),
            ) => {
                snapshot.audio = Some(audio);
                Feedback::AudioSaved(outcome)
            }
            (_, Err(problem)) => Feedback::Failed(problem),
            (_, Ok(_)) => Feedback::Failed(Problem::UnmatchedAnswer),
        };
        snapshot.feedback = Some(feedback.clone());
        Some(feedback)
    }

    pub(crate) fn retire(&mut self, backend: BackendId) {
        Rc::make_mut(&mut self.confirmed).retain(|(owner, _), _| *owner != backend);
        if self
            .read
            .as_ref()
            .is_some_and(|r| r.origin.address.backend == backend)
        {
            self.read = None;
        }
        if self
            .write
            .as_ref()
            .is_some_and(|r| r.origin.address.backend == backend)
        {
            self.write = None;
        }
        if let Some(view) = self
            .snapshot
            .as_mut()
            .filter(|s| s.origin.address.backend == backend)
        {
            let view = Rc::make_mut(view);
            view.catalog = None;
            view.audio = None;
            view.devices = [None, None];
            view.feedback = Some(Feedback::Failed(Problem::ConnectionLost));
        }
    }
}
