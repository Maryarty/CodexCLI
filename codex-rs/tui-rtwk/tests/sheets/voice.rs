//! Screen tests of the `voice` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/voice/`.

use codex_tui_core::app::AppEv;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::screens::voice::settings::{
    Answer, Audio, AudioOutcome, Catalog, Change, Device, DeviceKind, Operation, Problem,
    RealtimeVoice as Voice, Report, Request,
};
use codex_tui_core::screens::{ScreenEv, chat};

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
