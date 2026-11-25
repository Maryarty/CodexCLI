//! `--oss` / `--local-provider`: the local (open-source) provider of the
//! launch (oracle `startup_orchestration.rs`, `oss_selection.rs`).
//!
//! The choice is `--local-provider`, else the config's `oss_provider`, else
//! the one local server that answers (`detect_oss_provider`). The launch then
//! carries it as the `model_provider` session flag, the provider's default
//! model and raw reasoning, and makes the provider ready.

use std::path::Path;
use std::time::Duration;

use codex_config::TomlValue;
use codex_config::config_toml::ConfigToml;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_http_client::HttpClientBuilder;
use codex_model_provider_info::DEFAULT_LMSTUDIO_PORT;
use codex_model_provider_info::DEFAULT_OLLAMA_PORT;
use codex_model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use codex_utils_absolute_path::AbsolutePathBuf;

/// Both servers answer, or neither does: the oracle asks the user in a panel
/// (`select_oss_provider`); this launch has no panel before its config, so it
/// ends and names the flag.
pub const NEEDS_SELECTION: &str = "No single local provider is running: pass --local-provider lmstudio or --local-provider ollama.";

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

/// `detect_oss_provider` over two ports.
pub async fn detect_at(lmstudio_port: u16, ollama_port: u16) -> Option<&'static str> {
    let (lmstudio, ollama) = tokio::join!(running(lmstudio_port), running(ollama_port));
    single_running(lmstudio, ollama)
}

/// The provider `--oss` runs with: the flag, the config's, or the one running
/// server; an error text when none is decided.
pub async fn choose(
    explicit: Option<&str>,
    codex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
) -> Result<String, String> {
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
    match provider {
        Some(provider) => Ok(provider),
        None => detect_at(DEFAULT_LMSTUDIO_PORT, DEFAULT_OLLAMA_PORT)
            .await
            .map(str::to_owned)
            .ok_or_else(|| NEEDS_SELECTION.to_owned()),
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
        assert_eq!(chosen.as_deref(), Ok("ollama"));
    }

    #[tokio::test]
    async fn the_config_provider_is_read_from_the_layers() {
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::File::create(dir.path().join("config.toml"))
            .and_then(|mut file| file.write_all(b"oss_provider = \"lmstudio\"\n"))
            .expect("a config");
        let chosen = choose(None, dir.path(), None, Vec::new()).await;
        assert_eq!(chosen.as_deref(), Ok("lmstudio"));
    }
}
