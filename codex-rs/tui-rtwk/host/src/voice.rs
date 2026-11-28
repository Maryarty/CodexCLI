//! Ownership of the local packaged audio peer. All commands name one exact call.

use std::future::Future;
use std::sync::{Mutex, MutexGuard};

use codex_core::config::Config;
use codex_realtime_webrtc::{
    AudioDeviceSelection, RealtimeWebrtcSession, RealtimeWebrtcSessionHandle,
};
use codex_tui_core::ids::Ticket;
use codex_tui_core::screens::voice::{
    AudioAction, AudioCommand, AudioReport, AudioResult, LockStop, Owner, Peaks, Sdp,
};
use codex_tui_rtwk::services::agents::workspace::blocking;
use futures::FutureExt;
use futures::future::{AbortHandle, AbortRegistration, Abortable};
use tokio::sync::watch;

mod session_lock;

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
    prepare: Option<AbortRegistration>,
    abort_monitor: AbortHandle,
    monitor: Option<AbortRegistration>,
    ready: watch::Sender<bool>,
    handle: Option<RealtimeWebrtcSessionHandle>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.abort_monitor.abort();
        self.abort.abort();
        if let Some(handle) = &self.handle {
            handle.close();
        }
    }
}

impl Slot {
    fn reserve(&mut self, owner: &Owner) -> Option<&mut Entry> {
        if self
            .retired_through
            .is_some_and(|retired| retired >= owner.call)
        {
            return None;
        }
        let entry = self.entry.get_or_insert_with(|| {
            let (abort, prepare) = AbortHandle::new_pair();
            let (abort_monitor, monitor) = AbortHandle::new_pair();
            Entry {
                owner: owner.clone(),
                abort,
                prepare: Some(prepare),
                abort_monitor,
                monitor: Some(monitor),
                ready: watch::channel(false).0,
                handle: None,
            }
        });
        (&entry.owner == owner).then_some(entry)
    }

    fn stop(&mut self, owner: &Owner) -> bool {
        if self
            .entry
            .as_ref()
            .is_some_and(|entry| &entry.owner == owner)
        {
            self.retired_through = self.retired_through.max(Some(owner.call));
            self.entry = None;
            true
        } else {
            false
        }
    }
}

struct Monitor<'a> {
    audio: &'a Audio,
    owner: &'a Owner,
}

impl Drop for Monitor<'_> {
    fn drop(&mut self) {
        self.audio.lock().stop(self.owner);
    }
}

impl Audio {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) async fn run(
        &self,
        command: AudioCommand,
        config: Option<&Config>,
    ) -> Option<AudioReport> {
        let AudioCommand { owner, action } = command;
        let result = match action {
            AudioAction::ObserveLock => {
                if !RealtimeWebrtcSession::is_supported() {
                    return None;
                }
                AudioResult::LockStopped(self.observe(&owner, session_lock::observe()).await?)
            }
            AudioAction::Prepare => AudioResult::Prepared(self.prepare(&owner, config).await?),
            AudioAction::Connect(answer) => {
                AudioResult::Connected(self.connect(&owner, answer).await?)
            }
            AudioAction::SetMuted(muted) => AudioResult::Muted {
                muted,
                result: self
                    .handle(&owner)?
                    .set_microphone_muted(muted)
                    .map_err(|error| error.to_string()),
            },
            AudioAction::Sample => {
                let handle = self.handle(&owner)?;
                AudioResult::Sampled({
                    if let Some((_, message)) = handle.take_error() {
                        Err(message)
                    } else {
                        Ok(Peaks([
                            handle.take_microphone_peak(),
                            handle.take_speaker_peak(),
                        ]))
                    }
                })
            }
            AudioAction::Close => {
                self.close(&owner);
                AudioResult::Closed
            }
        };
        Some(AudioReport { owner, result })
    }

    async fn observe(
        &self,
        owner: &Owner,
        setup: impl Future<Output = anyhow::Result<session_lock::Activation>>,
    ) -> Option<LockStop> {
        let (registration, ready) = {
            let mut slot = self.lock();
            let entry = slot.reserve(owner)?;
            (entry.monitor.take()?, entry.ready.clone())
        };
        let _monitor = Monitor { audio: self, owner };
        let result = Abortable::new(
            async move {
                let mut activation = setup.await?;
                if let Some(locked) = activation.as_mut().now_or_never() {
                    return locked;
                }
                ready.send_replace(true);
                activation.await
            },
            registration,
        )
        .await
        .ok()?;
        let reason = match result {
            Ok(()) => LockStop::Locked,
            Err(error) => {
                tracing::debug!(%error, "session lock observer failed");
                LockStop::MonitorFailed
            }
        };
        self.lock().stop(owner).then_some(reason)
    }

    async fn prepare(&self, owner: &Owner, config: Option<&Config>) -> Option<Result<Sdp, String>> {
        if !RealtimeWebrtcSession::is_supported() {
            return Some(Err(
                "Voice requires a Codex package with a compatible audio helper.".into(),
            ));
        }
        let (registration, mut ready) = {
            let mut slot = self.lock();
            if slot
                .retired_through
                .is_some_and(|retired| retired >= owner.call)
            {
                return None;
            }
            let Some(entry) = slot.reserve(owner) else {
                return Some(Err(
                    "A local voice call already owns the audio devices.".into()
                ));
            };
            (entry.prepare.take()?, entry.ready.subscribe())
        };
        ready.wait_for(|ready| *ready).await.ok()?;
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
        let mut slot = self.lock();
        let Some(entry) = slot.entry.as_mut().filter(|entry| &entry.owner == owner) else {
            if let Ok(offer) = offer {
                offer.handle.close();
            }
            return None;
        };
        Some(match offer {
            Ok(offer) => {
                entry.handle = Some(offer.handle);
                Ok(Sdp::new(offer.offer_sdp))
            }
            Err(error) => {
                slot.stop(owner);
                Err(error)
            }
        })
    }

    async fn connect(&self, owner: &Owner, answer: Sdp) -> Option<Result<(), String>> {
        let handle = self.handle(owner)?;
        let connected = blocking(move || handle.apply_answer_sdp(answer.into_inner()))
            .await
            .ok_or_else(|| "The audio helper failed to connect.".to_owned())
            .and_then(|result| result.map_err(|error| error.to_string()));
        self.handle(owner)?;
        Some(connected)
    }

    fn handle(&self, owner: &Owner) -> Option<RealtimeWebrtcSessionHandle> {
        self.lock()
            .entry
            .as_ref()
            .filter(|entry| &entry.owner == owner)
            .and_then(|entry| entry.handle.clone())
    }

    fn close(&self, owner: &Owner) {
        let mut slot = self.lock();
        slot.retired_through = slot.retired_through.max(Some(owner.call));
        slot.stop(owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::app::effects::Ask;
    use codex_tui_core::app::{AppEv, AppFx, Intent, apply_app};
    use codex_tui_core::fixtures::script::build;
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::screens::{ScreenThreadIntent, voice::ThreadIntent};

    fn owners() -> [Owner; 2] {
        let mut app = build(r#"{"step":"seed","seed":"session","thread":"thread"}"#)
            .expect("session")
            .app;
        let event = |intent| {
            AppEv::Intent(Intent::ThreadScreen {
                thread: ThreadId::new("thread"),
                intent: ScreenThreadIntent::Voice(intent),
            })
        };
        std::array::from_fn(|_| {
            let mut fx = AppFx::default();
            apply_app(&mut app, event(ThreadIntent::Start), &mut fx);
            apply_app(&mut app, event(ThreadIntent::Stop), &mut AppFx::default());
            fx.commands()
                .iter()
                .find_map(|command| match command.host_ask() {
                    Some(Ask::Audio(call)) if call.action == AudioAction::ObserveLock => {
                        Some(call.owner.clone())
                    }
                    _ => None,
                })
                .expect("voice owner")
        })
    }

    #[tokio::test]
    async fn lock_snapshot_prevents_media_and_retires_the_ticket() {
        let audio = Audio::default();
        let [first, _] = owners();
        let mut ready = audio
            .lock()
            .reserve(&first)
            .expect("reserved")
            .ready
            .subscribe();
        let activation: session_lock::Activation = Box::pin(async { Ok(()) });
        assert_eq!(
            audio.observe(&first, async { Ok(activation) }).await,
            Some(LockStop::Locked)
        );
        assert!(!*ready.borrow_and_update());
        assert!(ready.has_changed().is_err());
        assert!(audio.lock().reserve(&first).is_none());
    }

    #[tokio::test]
    async fn stale_observer_cannot_stop_the_replacement_and_drop_stops_current() {
        let audio = Audio::default();
        let [first, next] = owners();
        let (lock, event) = tokio::sync::oneshot::channel();
        let activation: session_lock::Activation = Box::pin(async move { Ok(event.await?) });
        let mut old = Box::pin(audio.observe(&first, async { Ok(activation) }));
        assert!(old.as_mut().now_or_never().is_none());
        assert!(
            *audio
                .lock()
                .reserve(&first)
                .expect("first call")
                .ready
                .borrow()
        );

        audio.close(&first);
        let activation: session_lock::Activation = Box::pin(std::future::pending());
        let mut current = Box::pin(audio.observe(&next, async { Ok(activation) }));
        assert!(current.as_mut().now_or_never().is_none());
        lock.send(()).expect("late lock signal");
        assert_eq!(old.await, None);
        assert!(audio.lock().reserve(&next).is_some());
        drop(current);
        assert!(audio.lock().reserve(&next).is_none());
    }
}
