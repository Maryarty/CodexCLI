//! Voice preferences are server-owned; audio preferences belong to the client machine.

use std::path::Path;
use std::sync::Arc;

use codex_app_server_client::{AppServerRequestHandle, TypedRequestError};
use codex_app_server_protocol as wire;
use codex_config::config_toml::RealtimeAudioToml;
use codex_core::config::Config;
use codex_core::config::edit::{ConfigEdit, ConfigEditsBuilder};
use codex_install_context::InstallContext;
use codex_realtime_webrtc::{AudioDeviceKind, VoiceHost};
use codex_tui_core::protocol::config::config_write;
use codex_tui_core::screens::voice::settings::{
    Answer, Audio, AudioOutcome, Catalog, Change, Device, DeviceKind, Operation, Problem,
    RealtimeVoice, Report, Request, catalog_request,
};
use codex_utils_absolute_path::AbsolutePathBuf;

use codex_tui_rtwk::services::agents::workspace::blocking;
use codex_tui_rtwk::services::backend::rpc::Ids;
use tokio::sync::{Mutex, OwnedMutexGuard};

pub(super) async fn run_local(
    request: Request,
    config: Option<&Config>,
    gate: &Arc<Mutex<()>>,
) -> Option<Report> {
    let result = match &request.operation {
        Operation::ReadCatalog | Operation::Persist(Change::Voice(_)) => return None,
        Operation::ReadAudio => {
            let _guard = gate.lock().await;
            read_local(config)
                .await
                .map(Answer::Audio)
                .map_err(Problem::ReadAudio)
        }
        Operation::ListDevices(kind) => list_devices(*kind).await.map(|devices| Answer::Devices {
            kind: *kind,
            devices,
        }),
        Operation::Persist(change) => {
            save_audio(config, change, Arc::clone(gate).lock_owned().await).await
        }
    };
    Some(Report { request, result })
}

pub(crate) async fn run_server(
    request: Request,
    handle: &AppServerRequestHandle,
    ids: &Ids,
) -> Option<Report> {
    let server = Server { handle, ids };
    let result = match request.operation {
        Operation::ReadCatalog => catalog(&server, request.origin.cwd.clone())
            .await
            .map(Answer::Catalog),
        Operation::Persist(Change::Voice(voice)) => {
            save_voice(&server, request.origin.cwd.clone(), voice).await
        }
        _ => return None,
    };
    Some(Report { request, result })
}

struct Server<'a> {
    handle: &'a AppServerRequestHandle,
    ids: &'a Ids,
}

async fn voices(
    link: &Server<'_>,
) -> Result<codex_protocol::protocol::RealtimeVoicesList, TypedRequestError> {
    link.handle
        .request_typed::<wire::ThreadRealtimeListVoicesResponse>(catalog_request(link.ids.next()))
        .await
        .map(|reply| reply.voices)
}

async fn effective_voice(
    link: &Server<'_>,
    cwd: Option<String>,
    default: RealtimeVoice,
) -> anyhow::Result<Option<Option<RealtimeVoice>>> {
    let config = match link
        .handle
        .request_typed::<wire::ConfigReadResponse>(wire::ClientRequest::ConfigRead {
            request_id: link.ids.next(),
            params: wire::ConfigReadParams {
                include_layers: false,
                cwd,
            },
        })
        .await
    {
        Ok(reply) => reply.config,
        Err(TypedRequestError::Server { source, .. }) if source.code == -32601 => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let voice = config
        .additional
        .get("realtime")
        .and_then(|realtime| realtime.get("voice"));
    let effective = match voice {
        None | Some(serde_json::Value::Null) => Some(default),
        Some(value @ serde_json::Value::String(_)) => serde_json::from_value(value.clone()).ok(),
        Some(value) => Some(serde_json::from_value(value.clone())?),
    };
    Ok(Some(effective))
}

async fn catalog(link: &Server<'_>, cwd: Option<String>) -> Result<Catalog, Problem> {
    let voices = voices(link)
        .await
        .map_err(|error| Problem::Catalog(error.to_string()))?;
    let current = effective_voice(link, cwd, voices.default_v1)
        .await
        .map_err(|error| Problem::ReadVoice(error.to_string()))?
        .flatten();
    Ok(Catalog {
        voices: voices.v1,
        current,
    })
}

async fn save_voice(
    link: &Server<'_>,
    cwd: Option<String>,
    voice: RealtimeVoice,
) -> Result<Answer, Problem> {
    let request = config_write(
        &[codex_tui_core::app::effects::ConfigEdit {
            key_path: "realtime.voice".into(),
            value: serde_json::json!(voice),
        }],
        link.ids.next(),
    );
    let saved = link
        .handle
        .request_typed::<wire::ConfigWriteResponse>(request)
        .await
        .map_err(|error| Problem::WriteVoice(error.to_string()))?;
    let voices = voices(link)
        .await
        .map_err(|error| Problem::ReadSavedVoice(error.to_string()))?;
    let current = match effective_voice(link, cwd, voices.default_v1).await {
        Ok(Some(current)) => current,
        Ok(None) if saved.status != wire::WriteStatus::OkOverridden => Some(voice),
        Ok(None) => {
            return Err(Problem::ReadSavedVoice(
                "the server cannot read the overriding configuration".into(),
            ));
        }
        Err(error) => return Err(Problem::ReadSavedVoice(error.to_string())),
    };
    let warning = saved.overridden_metadata.map(|meta| meta.message);
    Ok(Answer::VoiceSaved {
        catalog: Catalog {
            voices: voices.v1,
            current,
        },
        warning,
    })
}

async fn list_devices(kind: DeviceKind) -> Result<Vec<Device>, Problem> {
    let package = InstallContext::current()
        .package_layout
        .as_ref()
        .ok_or(Problem::ListDevices)?;
    let mut host = VoiceHost::connect(package, codex_build_info::BuildInfo::get().build_commit())
        .await
        .map_err(|_| Problem::ListDevices)?;
    let native = match kind {
        DeviceKind::Input => AudioDeviceKind::Input,
        DeviceKind::Output => AudioDeviceKind::Output,
    };
    let result = host.list_devices(native).await;
    host.close().await.map_err(|_| Problem::ListDevices)?;
    Ok(result
        .map_err(|_| Problem::ListDevices)?
        .into_iter()
        .map(|device| Device {
            name: device.name,
            channels: device.channels,
            is_default: device.is_default,
        })
        .collect())
}

fn config_path(config: &Config) -> AbsolutePathBuf {
    config
        .config_layer_stack
        .get_user_config_file()
        .cloned()
        .unwrap_or_else(|| config.codex_home.join(codex_config::CONFIG_TOML_FILE))
}

async fn read_user(path: &Path) -> anyhow::Result<toml::Value> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(toml::from_str::<toml::Table>(std::str::from_utf8(&bytes)?)?.into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(toml::Table::new().into()),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn read_local(config: Option<&Config>) -> Result<Audio, String> {
    let Some(config) = config else {
        return Ok(Audio::default());
    };
    let path = config_path(config);
    let user = read_user(path.as_path())
        .await
        .map_err(|error| error.to_string())?;
    let stack = config
        .config_layer_stack
        .with_user_config(&path, user)
        .map_err(|error| error.to_string())?;
    let effective = stack.effective_config();
    let native = match effective.get("audio").cloned() {
        None => RealtimeAudioToml::default(),
        Some(audio) => audio
            .try_into::<RealtimeAudioToml>()
            .map_err(|error| error.to_string())?,
    };
    Ok(Audio {
        microphone: native.microphone,
        speaker: native.speaker,
        channels: native
            .microphone_channel
            .map(|channels| channels.as_slice().to_vec()),
    })
}

fn edit(field: &str, value: Option<toml_edit::Item>) -> ConfigEdit {
    let segments = vec!["audio".into(), field.into()];
    match value {
        Some(value) => ConfigEdit::SetPath { segments, value },
        None => ConfigEdit::ClearPath { segments },
    }
}

async fn save_audio(
    config: Option<&Config>,
    change: &Change,
    guard: OwnedMutexGuard<()>,
) -> Result<Answer, Problem> {
    let config = config.ok_or_else(|| Problem::WriteAudio("local config is unavailable".into()))?;
    let mut edits = Vec::new();
    match change {
        Change::Device { kind, name } => {
            let field = match kind {
                DeviceKind::Input => "microphone",
                DeviceKind::Output => "speaker",
            };
            edits.push(edit(field, name.as_deref().map(toml_edit::value)));
            if *kind == DeviceKind::Input {
                edits.push(edit("microphone_channel", None));
            }
        }
        Change::InputChannels {
            microphone,
            channels,
        } => {
            let before = read_local(Some(config)).await.map_err(Problem::ReadAudio)?;
            if before.microphone != *microphone {
                return Err(Problem::InputChanged);
            }
            let value = channels
                .as_ref()
                .map(|selection| match selection.as_slice() {
                    [one] => toml_edit::value(i64::from(one.get())),
                    many => toml_edit::value(
                        many.iter()
                            .map(|channel| i64::from(channel.get()))
                            .collect::<toml_edit::Array>(),
                    ),
                });
            edits.push(edit("microphone_channel", value));
        }
        Change::Voice(_) => return Err(Problem::UnmatchedAnswer),
    }
    let builder = ConfigEditsBuilder::for_config(config).with_edits(edits);
    // If the connection goes away while writing, the blocking edit may finish independently.
    // Its guard stays with it, so a later read cannot see the old setting as the result.
    let (written, _guard) = blocking(move || (builder.apply_blocking(), guard))
        .await
        .ok_or_else(|| Problem::WriteAudio("local config update was interrupted".into()))?;
    written.map_err(|err| Problem::WriteAudio(err.to_string()))?;
    let audio = read_local(Some(config))
        .await
        .map_err(Problem::ReadSavedAudio)?;
    let outcome = match change {
        Change::Device {
            kind: DeviceKind::Input,
            ..
        } if audio.channels.is_some() => AudioOutcome::MicrophoneChannelOverridden,
        Change::Device { kind, name } if name.as_deref() != audio.device(*kind) => {
            AudioOutcome::Overridden
        }
        Change::InputChannels {
            microphone,
            channels,
        } if audio.microphone != *microphone || audio.channels != *channels => {
            AudioOutcome::Overridden
        }
        _ => AudioOutcome::Applied,
    };
    Ok(Answer::AudioSaved { audio, outcome })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::config::ConfigBuilder;
    use std::num::NonZeroU16;

    #[tokio::test]
    async fn audio_selection_is_durable_and_changing_input_clears_its_channels() {
        let home = tempfile::tempdir().expect("codex home");
        let path = home.path().join(codex_config::CONFIG_TOML_FILE);
        let gate = Arc::new(Mutex::new(()));
        let config = ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .expect("isolated config");
        let selected = Change::Device {
            kind: DeviceKind::Input,
            name: Some("USB Interface".into()),
        };
        assert!(matches!(
            save_audio(
                Some(&config),
                &selected,
                Arc::clone(&gate).lock_owned().await
            )
            .await,
            Ok(Answer::AudioSaved {
                outcome: AudioOutcome::Applied,
                ..
            })
        ));
        let channels = Some([1, 3].into_iter().filter_map(NonZeroU16::new).collect());
        let subset = Change::InputChannels {
            microphone: Some("USB Interface".into()),
            channels: channels.clone(),
        };
        assert!(matches!(
            save_audio(Some(&config), &subset, Arc::clone(&gate).lock_owned().await).await,
            Ok(Answer::AudioSaved {
                outcome: AudioOutcome::Applied,
                ..
            })
        ));
        assert_eq!(
            read_local(Some(&config))
                .await
                .expect("effective selection")
                .channels,
            channels
        );
        let persisted = read_user(&path).await.expect("persisted config");
        assert_eq!(
            persisted["audio"]["microphone_channel"]
                .as_array()
                .expect("two channels")
                .len(),
            2
        );
        let fallback = Change::Device {
            kind: DeviceKind::Input,
            name: None,
        };
        assert!(matches!(
            save_audio(
                Some(&config),
                &fallback,
                Arc::clone(&gate).lock_owned().await
            )
            .await,
            Ok(Answer::AudioSaved {
                outcome: AudioOutcome::Applied,
                ..
            })
        ));
        assert_eq!(
            read_local(Some(&config)).await.expect("effective default"),
            Audio::default()
        );
        assert_eq!(
            save_audio(Some(&config), &subset, Arc::clone(&gate).lock_owned().await).await,
            Err(Problem::InputChanged)
        );
    }
}
