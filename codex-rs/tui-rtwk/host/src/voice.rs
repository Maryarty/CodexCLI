//! Ownership of the local packaged audio peer. All commands name one exact call.

use std::sync::{Mutex, MutexGuard};

use codex_core::config::Config;
use codex_realtime_webrtc::{
    AudioDeviceSelection, RealtimeWebrtcSession, RealtimeWebrtcSessionHandle,
};
use codex_tui_core::ids::Ticket;
use codex_tui_core::screens::voice::{
    AudioAction, AudioCommand, AudioReport, AudioResult, Owner, Peaks, Sdp,
};
use codex_tui_rtwk::services::agents::workspace::blocking;
use futures::future::AbortHandle;

#[derive(Default)]
pub(crate) struct Audio(Mutex<Slot>);

#[derive(Default)]
struct Slot {
    // A stop may execute before a queued prepare. It retires that exact ticket before it can open.
    retired_through: Option<Ticket>,
    entry: Option<Entry>,
}

struct Entry {
    owner: Owner,
    abort: AbortHandle,
    handle: Option<RealtimeWebrtcSessionHandle>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.abort.abort();
        if let Some(handle) = &self.handle {
            handle.close();
        }
    }
}

impl Audio {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) async fn run(&self, command: AudioCommand, config: Option<&Config>) -> AudioReport {
        let AudioCommand { owner, action } = command;
        let result = match action {
            AudioAction::Prepare => AudioResult::Prepared(self.prepare(&owner, config).await),
            AudioAction::Connect(answer) => {
                AudioResult::Connected(self.connect(&owner, answer).await)
            }
            AudioAction::SetMuted(muted) => AudioResult::Muted {
                muted,
                result: self.handle(&owner).and_then(|handle| {
                    handle
                        .set_microphone_muted(muted)
                        .map_err(|error| error.to_string())
                }),
            },
            AudioAction::Sample => AudioResult::Sampled(self.handle(&owner).and_then(|handle| {
                if let Some((_, message)) = handle.take_error() {
                    Err(message)
                } else {
                    Ok(Peaks([
                        handle.take_microphone_peak(),
                        handle.take_speaker_peak(),
                    ]))
                }
            })),
            AudioAction::Close => {
                self.close(&owner);
                AudioResult::Closed
            }
        };
        AudioReport { owner, result }
    }

    async fn prepare(&self, owner: &Owner, config: Option<&Config>) -> Result<Sdp, String> {
        if !RealtimeWebrtcSession::is_supported() {
            return Err("Voice requires a Codex package with a compatible audio helper.".into());
        }
        let (abort, registration) = AbortHandle::new_pair();
        {
            let mut slot = self.lock();
            if slot
                .retired_through
                .is_some_and(|retired| retired >= owner.call)
            {
                return Err("The voice call was cancelled.".into());
            }
            if slot.entry.is_some() {
                return Err("A local voice call already owns the audio devices.".into());
            }
            slot.entry = Some(Entry {
                owner: owner.clone(),
                abort,
                handle: None,
            });
        }
        let selection = config
            .map(|config| AudioDeviceSelection {
                microphone: config.realtime_audio.microphone.clone(),
                speaker: config.realtime_audio.speaker.clone(),
                channel: config
                    .realtime_audio
                    .microphone_channel
                    .as_ref()
                    .map(|channels| channels.as_slice().to_vec()),
            })
            .unwrap_or_default();
        let offer = blocking(move || RealtimeWebrtcSession::start(registration, selection))
            .await
            .ok_or_else(|| "The audio helper failed to prepare.".to_owned())
            .and_then(|result| result.map_err(|error| error.to_string()));
        match offer {
            Ok(offer) => {
                let mut slot = self.lock();
                if let Some(entry) = slot.entry.as_mut().filter(|entry| &entry.owner == owner) {
                    entry.handle = Some(offer.handle);
                    Ok(Sdp::new(offer.offer_sdp))
                } else {
                    offer.handle.close();
                    Err("The voice call was cancelled.".into())
                }
            }
            Err(error) => {
                self.close(owner);
                Err(error)
            }
        }
    }

    async fn connect(&self, owner: &Owner, answer: Sdp) -> Result<(), String> {
        let handle = self.handle(owner)?;
        blocking(move || handle.apply_answer_sdp(answer.into_inner()))
            .await
            .ok_or_else(|| "The audio helper failed to connect.".to_owned())?
            .map_err(|error| error.to_string())
    }

    fn handle(&self, owner: &Owner) -> Result<RealtimeWebrtcSessionHandle, String> {
        self.lock()
            .entry
            .as_ref()
            .filter(|entry| &entry.owner == owner)
            .and_then(|entry| entry.handle.clone())
            .ok_or_else(|| "The local audio connection is no longer available.".into())
    }

    fn close(&self, owner: &Owner) {
        let mut slot = self.lock();
        slot.retired_through = slot.retired_through.max(Some(owner.call));
        if slot
            .entry
            .as_ref()
            .is_some_and(|entry| &entry.owner == owner)
        {
            slot.entry = None;
        }
    }
}
