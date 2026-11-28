//! Screen tests of the `voice` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/voice/`.

use codex_tui_core::app::AppEv;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{BackendEv, Command};
use codex_tui_core::protocol::reply;
use codex_tui_core::screens::voice::settings::{
    Answer, Audio, AudioOutcome, Catalog, Change, Device, DeviceKind, Operation, Problem,
    RealtimeVoice as Voice, Report, Request,
};
use codex_tui_core::screens::voice::{AudioAction, AudioReport, AudioResult, ConnectFailure, Sdp};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, chat, voice};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use crate::common::{Harness, slot, start};

#[test]
fn voice_slot_is_registered_and_empty() {
    slot("voice");
}

fn voice_settings() -> Harness {
    let mut h = start();
    crate::chat::chat_with_history(&mut h);
    h.inject([AppEv::Screen(ScreenEv::Chat(chat::Ev::VoiceAvailable(
        true,
    )))]);
    h.t.type_text("/voice settings");
    h.press("enter");
    h.wait_for("the voice settings sheet", |screen| {
        screen.contains("Choose a voice")
    });
    h
}

fn request(h: &mut Harness, accepts: impl Fn(&Operation) -> bool) -> Request {
    let find = |h: &Harness| {
        h.fake
            .commands()
            .iter()
            .rev()
            .find_map(|cmd| match cmd.host_ask() {
                Some(Ask::VoiceSettings(request)) if accepts(&request.operation) => {
                    Some(request.clone())
                }
                _ => None,
            })
    };
    h.wait_until("the voice settings request", |h| find(h).is_some());
    let Some(request) = find(h) else {
        panic!("the voice settings request was not recorded");
    };
    request
}

fn answer(h: &mut Harness, request: Request, result: Result<Answer, Problem>) {
    h.inject([AppEv::Effect {
        request: 0,
        done: Done::VoiceSettings(Box::new(Report { request, result })),
    }]);
}

#[test]
fn voice_settings_displays_the_live_catalog_and_waits_for_an_effective_write() {
    let mut h = voice_settings();
    h.press("down");
    h.press("enter");
    let read = request(&mut h, |op| *op == Operation::ReadCatalog);
    let catalog = |current| Catalog {
        voices: vec![Voice::Juniper, Voice::Maple],
        current: Some(current),
    };
    answer(&mut h, read, Ok(Answer::Catalog(catalog(Voice::Juniper))));
    h.wait_for("the actual saved voice", |screen| {
        screen.contains("juniper (current)")
    });
    h.press("down");
    h.press("enter");
    let save = request(&mut h, |op| {
        *op == Operation::Persist(Change::Voice(Voice::Maple))
    });
    assert!(
        !h.t.screen().contains("Voice set to maple"),
        "{}",
        h.t.screen()
    );
    answer(
        &mut h,
        save,
        Ok(Answer::VoiceSaved {
            catalog: catalog(Voice::Maple),
            warning: None,
        }),
    );
    h.wait_for("acknowledged voice selection", |screen| {
        screen.contains("Voice set to maple")
            && screen.contains("• Hello")
            && !screen.contains("Select voice")
    });
}

#[test]
fn voice_settings_can_save_a_subset_of_input_channels() {
    let mut h = voice_settings();
    h.press("enter");
    let read = request(&mut h, |op| *op == Operation::ReadAudio);
    answer(&mut h, read, Ok(Answer::Audio(Audio::default())));
    h.wait_for("the settings for both devices", |screen| {
        screen.contains("Output device")
    });
    h.press("enter");
    let input = request(&mut h, |op| {
        *op == Operation::ListDevices(DeviceKind::Input)
    });
    let device = |name: &str, channels, is_default| Device {
        name: name.into(),
        channels,
        is_default,
    };
    answer(
        &mut h,
        input,
        Ok(Answer::Devices {
            kind: DeviceKind::Input,
            devices: vec![
                device("USB Interface", 4, true),
                device("Duplicate", 2, false),
                device("Duplicate", 2, false),
            ],
        }),
    );
    h.wait_for("multichannel input and an ambiguous device", |screen| {
        screen.contains("Input channels") && screen.contains("Identical device names")
    });
    h.press("down");
    h.press("enter");
    h.wait_for("the channel editor", |screen| {
        screen.contains("Microphone: USB Interface") && screen.contains("Input 4")
    });
    h.press("space");
    h.press("enter");
    let saved = request(
        &mut h,
        |op| matches!(op, Operation::Persist(Change::InputChannels { microphone: None, channels: Some(channels) }) if channels.iter().map(|n| n.get()).eq([2, 3, 4])),
    );
    let Operation::Persist(Change::InputChannels { channels, .. }) = &saved.operation else {
        unreachable!()
    };
    let effective = Audio {
        channels: channels.clone(),
        ..Audio::default()
    };
    answer(
        &mut h,
        saved,
        Ok(Answer::AudioSaved {
            audio: effective,
            outcome: AudioOutcome::Applied,
        }),
    );
    h.wait_for("the acknowledged audio save", |screen| {
        screen.contains("Audio setting saved") && !screen.contains("Microphone: USB Interface")
    });
}

#[test]
fn voice_settings_show_device_failure_and_back_reopens_the_sound_menu() {
    let mut h = voice_settings();
    h.press("enter");
    let read = request(&mut h, |op| *op == Operation::ReadAudio);
    answer(&mut h, read, Ok(Answer::Audio(Audio::default())));
    h.wait_for("the settings for both devices", |screen| {
        screen.contains("Output device")
    });
    h.press("down");
    h.press("enter");
    let output = request(&mut h, |op| {
        *op == Operation::ListDevices(DeviceKind::Output)
    });
    answer(&mut h, output, Err(Problem::ListDevices));
    h.wait_for("the typed device refusal", |screen| {
        screen.contains("Could not list audio devices")
    });
    h.press("escape");
    h.wait_for("the devices parent", |screen| {
        screen.contains("Sound devices")
    });
}

fn voice_command(h: &mut Harness, accepts: impl Fn(&Command) -> bool) -> Command {
    let find = |h: &Harness| h.fake.commands().into_iter().rev().find(&accepts);
    h.wait_until("the recorded voice call command", |h| find(h).is_some());
    let Some(command) = find(h) else {
        panic!("voice call command missing")
    };
    command
}

fn voice_audio(h: &mut Harness, accepts: impl Fn(&AudioAction) -> bool) -> Command {
    voice_command(
        h,
        |command| matches!(command.host_ask(), Some(Ask::Audio(audio)) if accepts(&audio.action)),
    )
}

fn audio_receipt(command: &Command, result: AudioResult) -> AppEv {
    let Command::Effect {
        ask: Ask::Audio(audio),
        request,
        ..
    } = command
    else {
        panic!("audio command")
    };
    AppEv::Effect {
        request: *request,
        done: Done::Audio(AudioReport {
            owner: audio.owner.clone(),
            result,
        }),
    }
}

fn voice_notice(method: &str, params: serde_json::Value) -> AppEv {
    let Ok(notification) =
        serde_json::from_value(serde_json::json!({ "method": method, "params": params }))
    else {
        panic!("pinned voice notification")
    };
    AppEv::Backend(BackendEv::Notification(Box::new(notification)))
}

fn voice_ack(command: &Command) -> AppEv {
    let Some((ticket, reply)) = reply(command, Ok(serde_json::json!({}))) else {
        panic!("pinned voice RPC")
    };
    AppEv::Reply(ticket, reply)
}

fn answer_voice_offer(
    h: &mut Harness,
    prepared: &Command,
    prior_session: Option<&str>,
) -> (String, Command) {
    h.inject([audio_receipt(
        prepared,
        AudioResult::Prepared(Ok(Sdp::new("offer".into()))),
    )]);
    let start = voice_command(h, |command| {
        matches!(command,
        Command::Screen { command: ScreenCommand::Voice(voice::Command::Start { session, .. }), .. }
        if Some(session.as_str()) != prior_session)
    });
    let Command::Screen {
        command: ScreenCommand::Voice(voice::Command::Start { session, .. }),
        ..
    } = &start
    else {
        panic!("voice start")
    };
    h.inject([
        voice_ack(&start),
        voice_notice("thread/realtime/started", serde_json::json!({"threadId": FAKE_THREAD, "realtimeSessionId": session, "version": "v3"})),
        voice_notice("thread/realtime/sdp", serde_json::json!({"threadId": FAKE_THREAD, "sdp": "answer"})),
    ]);
    let Some(Ask::Audio(prepared)) = prepared.host_ask() else {
        panic!("preparing voice")
    };
    let connect = voice_command(h, |command| {
        matches!(command.host_ask(),
        Some(Ask::Audio(audio)) if audio.owner == prepared.owner && matches!(audio.action, AudioAction::Connect(_)))
    });
    (session.clone(), connect)
}

#[test]
fn voice_f8_visibly_retries_once_after_cleanup_and_connects_without_losing_the_draft() {
    for timed_out in [true, false] {
        let mut h = start();
        crate::chat::chat_with_history(&mut h);
        h.t.type_text("keep my draft");
        h.press("f8");
        let prepared = voice_audio(&mut h, |action| *action == AudioAction::Prepare);
        let (previous_session, connect) = answer_voice_offer(&mut h, &prepared, None);
        let (notice, event) = if timed_out {
            (
                "Voice connection timed out. Retrying once after cleanup.",
                audio_receipt(
                    &connect,
                    AudioResult::Connected(Err(ConnectFailure::TimedOut("timeout".into()))),
                ),
            )
        } else {
            (
                "Voice connection closed during startup. Retrying once.",
                voice_notice(
                    "thread/realtime/closed",
                    serde_json::json!({"threadId": FAKE_THREAD, "reason": "transport_closed"}),
                ),
            )
        };
        h.inject([event]);
        let release = voice_audio(&mut h, |action| *action == AudioAction::ReleasePeer);
        h.wait_for("the original retry notice", |screen| {
            screen.contains(notice)
        });
        h.inject([audio_receipt(&release, AudioResult::PeerReleased)]);
        let is_stop = |command: &Command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Voice(voice::Command::Stop),
                    ..
                }
            )
        };
        if timed_out {
            let stop = voice_command(&mut h, is_stop);
            h.inject([voice_notice(
                "thread/realtime/closed",
                serde_json::json!({"threadId": FAKE_THREAD, "reason": "requested"}),
            )]);
            h.settle();
            assert!(!h.fake.commands().iter().any(is_retry));
            h.inject([voice_ack(&stop)]);
        }
        let retry = voice_command(&mut h, is_retry);
        if !timed_out {
            assert!(!h.fake.commands().iter().any(is_stop));
        }
        let (next_session, connect) = answer_voice_offer(&mut h, &retry, Some(&previous_session));
        assert_ne!(next_session, previous_session);
        h.inject([audio_receipt(&connect, AudioResult::Connected(Ok(())))]);
        h.wait_for("the retry listening with the draft", |screen| {
            screen.contains("voice ● listen") && screen.contains("keep my draft")
        });
        assert_eq!(
            h.fake
                .commands()
                .iter()
                .filter(|command| is_retry(command))
                .count(),
            1
        );
    }
}

fn is_retry(command: &Command) -> bool {
    matches!(command.host_ask(), Some(Ask::Audio(audio)) if matches!(audio.action, AudioAction::PrepareRetry { .. }))
}
