//! `--oss` / `--local-provider`: the local (open-source) provider of the
//! launch (oracle `startup_orchestration.rs`, `oss_selection.rs`).
//!
//! The choice is `--local-provider`, else the config's `oss_provider`, else
//! the one local server that answers (`detect_oss_provider`); both or none
//! answering asks the user in the provider panel (`pick`), and a pick made
//! there is kept as `oss_provider` (`persist`). The launch then
//! carries it as the `model_provider` session flag, the provider's default
//! model and raw reasoning, and makes the provider ready.

use std::path::Path;
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_config::TomlValue;
use codex_config::config_toml::ConfigToml;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_http_client::HttpClientBuilder;
use codex_model_provider_info::DEFAULT_LMSTUDIO_PORT;
use codex_model_provider_info::DEFAULT_OLLAMA_PORT;
use codex_model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::effects::ConfigEdit;
use codex_tui_core::protocol::config::config_write;
use codex_tui_core::protocol::config::config_written;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::Ask;
use codex_tui_core::screens::onboarding::Ev;
use codex_tui_core::screens::startup::Provider;
use codex_tui_core::screens::startup::Status;
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_utils_absolute_path::AbsolutePathBuf;
use tokio::sync::mpsc::UnboundedReceiver;

/// What the oracle's `select_oss_provider` shows when the user is asked.
pub const CANCELLED: &str = "OSS provider selection was cancelled by user";

/// The provider the flag or the config names (`resolve_oss_provider`).
pub fn resolve(explicit: Option<&str>, config: &ConfigToml) -> Option<String> {
    codex_core::config::resolve_oss_provider(explicit, config)
}

/// `detect_oss_provider`'s rule: the one provider that is running is chosen.
pub fn single_running(lmstudio: bool, ollama: bool) -> Option<&'static str> {
    match (lmstudio, ollama) {
        (true, false) => Some(LMSTUDIO_OSS_PROVIDER_ID),
        (false, true) => Some(OLLAMA_OSS_PROVIDER_ID),
        _ => None,
    }
}

/// `check_port_status`: the server on `http://localhost:<port>` answers with
/// a success. These probes bypass proxy discovery: the targets are loopback.
/// A client that cannot be built (an unreadable custom CA bundle: the oracle
/// goes on with the system roots, through a deprecated builder) probes
/// nothing: the server counts as not running.
pub async fn running(port: u16) -> bool {
    let Ok(client) = HttpClientBuilder::new().build_direct() else {
        return false;
    };
    match client
        .get(format!("http://localhost:{port}"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Which of the two servers answer: (LM Studio, Ollama).
pub async fn probe(lmstudio_port: u16, ollama_port: u16) -> (bool, bool) {
    tokio::join!(running(lmstudio_port), running(ollama_port))
}

/// Where the launch's provider stands.
#[derive(Debug, PartialEq, Eq)]
pub enum Choice {
    /// Decided: the flag, the config's, or the one running server.
    Provider(String),
    /// Both servers answer, or neither does: the user is asked (`select_oss_provider`).
    Pick { lmstudio: bool, ollama: bool },
}

/// The provider `--oss` runs with: the flag, the config's, or the one running
/// server; the two servers' states when none is decided.
pub async fn choose(
    explicit: Option<&str>,
    codex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
) -> Result<Choice, String> {
    let provider = match explicit {
        Some(provider) => Some(provider.to_owned()),
        None => {
            let loaded = codex_core::config::load_config_toml_with_layer_stack(
                codex_home,
                cwd,
                cli_overrides,
                codex_core::config::ConfigLoadOptions::default(),
            )
            .await
            .map_err(|error| format!("loading the config: {error}"))?;
            resolve(None, &loaded.config_toml)
        }
    };
    if let Some(provider) = provider {
        return Ok(Choice::Provider(provider));
    }
    let (lmstudio, ollama) = probe(DEFAULT_LMSTUDIO_PORT, DEFAULT_OLLAMA_PORT).await;
    Ok(match single_running(lmstudio, ollama) {
        Some(provider) => Choice::Provider(provider.to_owned()),
        None => Choice::Pick { lmstudio, ollama },
    })
}

/// The status the panel draws for a server (`check_port_status`: every send error is
/// `NotRunning`, never `Unknown`).
fn status(running: bool) -> Status {
    if running {
        Status::Running
    } else {
        Status::NotRunning
    }
}

/// `select_oss_provider`: the panel opens first among the startup prompts and the user's pick
/// is awaited (`Ask::OssChoice`). The launch task owns the app's commands and reads none of
/// them before it connects, so the pick is the first one that can come: the app asks the host
/// for nothing else before `Connected`. Ctrl-C is the oracle's error, and so is an app that
/// ended without answering.
pub async fn pick<E: FnMut(AppEv)>(
    lmstudio: bool,
    ollama: bool,
    mut emit: E,
    commands: &mut UnboundedReceiver<Command>,
) -> Result<String, String> {
    emit(AppEv::Screen(ScreenEv::Onboarding(Ev::OssPick {
        lmstudio: status(lmstudio),
        ollama: status(ollama),
    })));
    // Another command cannot come before `Connected`: it is dropped.
    while let Some(command) = commands.recv().await {
        if let Command::Startup { ask, .. } = &command
            && let Some(answer) = answer(ask)
        {
            return answer;
        }
    }
    Err(CANCELLED.to_owned())
}

/// What an ask of the page means for the pick: the provider's id, the cancel's error, or none
/// for an ask that is not the panel's.
fn answer(ask: &Ask) -> Option<Result<String, String>> {
    let Ask::OssChoice { provider } = ask else {
        return None;
    };
    Some(match provider {
        Some(Provider::LmStudio) => Ok(LMSTUDIO_OSS_PROVIDER_ID.to_owned()),
        Some(Provider::Ollama) => Ok(OLLAMA_OSS_PROVIDER_ID.to_owned()),
        None => Err(CANCELLED.to_owned()),
    })
}

/// A provider picked in the panel is kept as `oss_provider` (`lib.rs`: `write_config_batch`
/// once the app server session is up). A failure is only logged.
pub async fn persist<F, Fut>(provider: &str, ids: &Ids, send: F)
where
    F: Fn(ClientRequest) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, ServerError>>,
{
    let edit = ConfigEdit {
        key_path: "oss_provider".to_owned(),
        value: serde_json::json!(provider),
    };
    let written = match send(config_write(&[edit], ids.next())).await {
        Ok(result) => config_written(result),
        Err(error) => Err(error.message),
    };
    if let Err(error) = written {
        tracing::warn!(%error, provider, "Failed to persist selected OSS provider preference");
    }
}

/// What the choice puts on the launch (`startup_orchestration.rs`): the
/// `model_provider` kv override (kept by every config reload), the model
/// (`-m`, else the provider's default) and raw agent reasoning. Returns the
/// model the threads start with.
pub fn apply(
    provider: &str,
    model: Option<String>,
    kv_overrides: &mut Vec<(String, TomlValue)>,
    overrides: &mut ConfigOverrides,
) -> Option<String> {
    kv_overrides.push((
        "model_provider".to_owned(),
        TomlValue::String(provider.to_owned()),
    ));
    let model = model.or_else(|| {
        codex_utils_oss::get_default_model_for_oss_provider(provider).map(str::to_owned)
    });
    overrides.model.clone_from(&model);
    overrides.model_provider = Some(provider.to_owned());
    overrides.show_raw_agent_reasoning = Some(true);
    model
}

/// `ensure_oss_provider_ready`: the provider's models are there and its
/// service answers. The oracle's pull reports progress through a reporter
/// that is not `Send`, so the work runs on the blocking pool (the services'
/// `blocking`), driven there to its end; the launch awaits it.
pub async fn ensure_ready(provider: String, config: std::sync::Arc<Config>) -> Result<(), String> {
    let runtime = tokio::runtime::Handle::current();
    codex_tui_rtwk::services::agents::workspace::blocking(move || {
        runtime
            .block_on(codex_utils_oss::ensure_oss_provider_ready(
                &provider, &config,
            ))
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap_or_else(|| Err("the local provider setup ended without an answer".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;

    use super::*;

    /// `detect_oss_provider` over two ports.
    async fn detect_at(lmstudio_port: u16, ollama_port: u16) -> Option<&'static str> {
        let (lmstudio, ollama) = probe(lmstudio_port, ollama_port).await;
        single_running(lmstudio, ollama)
    }

    #[test]
    fn oss_provider_is_the_flag_then_the_config() {
        let mut config = ConfigToml::default();
        assert_eq!(resolve(None, &config), None);
        config.oss_provider = Some("ollama".to_owned());
        assert_eq!(resolve(None, &config).as_deref(), Some("ollama"));
        assert_eq!(
            resolve(Some("lmstudio"), &config).as_deref(),
            Some("lmstudio")
        );
    }

    #[test]
    fn oss_sets_provider_override_default_model_and_raw_reasoning() {
        let (mut kv, mut overrides) = (Vec::new(), ConfigOverrides::default());
        let model = apply("ollama", None, &mut kv, &mut overrides);
        assert_eq!(
            model.as_deref(),
            Some(codex_ollama::DEFAULT_OSS_MODEL),
            "the provider's default model"
        );
        assert_eq!(overrides.model, model);
        assert_eq!(overrides.model_provider.as_deref(), Some("ollama"));
        assert_eq!(overrides.show_raw_agent_reasoning, Some(true));
        assert_eq!(
            kv,
            [(
                "model_provider".to_owned(),
                TomlValue::String("ollama".to_owned())
            )]
        );
        // `-m` wins over the default.
        let (mut kv, mut overrides) = (Vec::new(), ConfigOverrides::default());
        let model = apply("lmstudio", Some("mine".to_owned()), &mut kv, &mut overrides);
        assert_eq!(model.as_deref(), Some("mine"));
    }

    /// A loopback listener, and the port it holds.
    async fn listener() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let port = listener.local_addr().expect("an address").port();
        (listener, port)
    }

    /// A server answering every request with `status`, until dropped.
    async fn serve(listener: TcpListener, status: &'static str) {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer).await;
            let reply =
                format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(reply.as_bytes()).await;
        }
    }

    /// A port nobody listens on.
    async fn closed() -> u16 {
        listener().await.1
    }

    #[tokio::test]
    async fn a_single_running_server_is_auto_selected() {
        let (up, port) = listener().await;
        let lmstudio = tokio::select! {
            got = async { detect_at(port, closed().await).await } => got,
            () = serve(up, "200 OK") => unreachable!("the server runs on"),
        };
        assert_eq!(lmstudio, Some(LMSTUDIO_OSS_PROVIDER_ID));
        let (up, port) = listener().await;
        let ollama = tokio::select! {
            got = async { detect_at(closed().await, port).await } => got,
            () = serve(up, "200 OK") => unreachable!("the server runs on"),
        };
        assert_eq!(ollama, Some(OLLAMA_OSS_PROVIDER_ID));
    }

    #[tokio::test]
    async fn both_or_none_running_selects_nothing() {
        let ((a, port_a), (b, port_b)) = (listener().await, listener().await);
        let both = tokio::select! {
            got = detect_at(port_a, port_b) => got,
            () = serve(a, "200 OK") => unreachable!("the server runs on"),
            () = serve(b, "200 OK") => unreachable!("the server runs on"),
        };
        assert_eq!(both, None);
        assert_eq!(detect_at(closed().await, closed().await).await, None);
        // Both, or none, running: the panel asks, with the states it draws.
        let ((a, port_a), (b, port_b)) = (listener().await, listener().await);
        let states = tokio::select! {
            got = probe(port_a, port_b) => got,
            () = serve(a, "200 OK") => unreachable!("the server runs on"),
            () = serve(b, "200 OK") => unreachable!("the server runs on"),
        };
        assert_eq!(states, (true, true));
        assert_eq!(probe(closed().await, closed().await).await, (false, false));
        // A server that answers with a failure is not running.
        let (down, port) = listener().await;
        let failing = tokio::select! {
            got = async { detect_at(port, closed().await).await } => got,
            () = serve(down, "503 Service Unavailable") => unreachable!("the server runs on"),
        };
        assert_eq!(failing, None);
    }

    #[tokio::test]
    async fn an_explicit_provider_needs_no_config_or_probe() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let chosen = choose(Some("ollama"), dir.path(), None, Vec::new()).await;
        assert_eq!(chosen, Ok(Choice::Provider("ollama".to_owned())));
    }

    #[tokio::test]
    async fn the_config_provider_is_read_from_the_layers() {
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::File::create(dir.path().join("config.toml"))
            .and_then(|mut file| file.write_all(b"oss_provider = \"lmstudio\"\n"))
            .expect("a config");
        let chosen = choose(None, dir.path(), None, Vec::new()).await;
        assert_eq!(chosen, Ok(Choice::Provider("lmstudio".to_owned())));
    }

    #[tokio::test]
    async fn pick_emits_the_panel_and_a_closed_channel_ends_it() {
        let (tx, mut commands) = tokio::sync::mpsc::unbounded_channel();
        drop(tx);
        let mut emitted = Vec::new();
        let picked = pick(true, false, |ev| emitted.push(ev), &mut commands).await;
        assert_eq!(picked, Err(CANCELLED.to_owned()));
        let [AppEv::Screen(ScreenEv::Onboarding(panel))] = emitted.as_slice() else {
            panic!("one event: {}", emitted.len());
        };
        assert_eq!(
            panel,
            &Ev::OssPick {
                lmstudio: Status::Running,
                ollama: Status::NotRunning,
            }
        );
    }

    #[test]
    fn the_panels_answer_is_the_provider_or_the_oracles_error() {
        let ask = |provider| Ask::OssChoice { provider };
        assert_eq!(
            answer(&ask(Some(Provider::LmStudio))),
            Some(Ok(LMSTUDIO_OSS_PROVIDER_ID.to_owned()))
        );
        assert_eq!(
            answer(&ask(Some(Provider::Ollama))),
            Some(Ok(OLLAMA_OSS_PROVIDER_ID.to_owned()))
        );
        assert_eq!(
            answer(&ask(None)),
            Some(Err(
                "OSS provider selection was cancelled by user".to_owned()
            ))
        );
        assert_eq!(answer(&Ask::AcknowledgeGovCloud), None);
    }

    #[tokio::test]
    async fn a_picked_provider_is_written_as_oss_provider_and_a_failure_is_only_logged() {
        let sent = std::sync::Mutex::new(Vec::new());
        let send = |request: ClientRequest| {
            sent.lock().expect("lock").push(request);
            async { Ok(serde_json::json!({})) }
        };
        persist("ollama", &Ids::default(), send).await;
        let sent = sent.into_inner().expect("lock");
        let [ClientRequest::ConfigBatchWrite { params, .. }] = sent.as_slice() else {
            panic!("one config write: {sent:?}");
        };
        assert_eq!(params.edits.len(), 1);
        assert_eq!(params.edits[0].key_path, "oss_provider");
        assert_eq!(params.edits[0].value, serde_json::json!("ollama"));
        // A refused write only warns.
        persist("ollama", &Ids::default(), |_| async {
            Err(ServerError {
                code: None,
                message: "no".to_owned(),
                data: None,
            })
        })
        .await;
    }
}
