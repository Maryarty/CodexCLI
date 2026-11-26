//! Which app server `codex-rtwk` talks to, as the oracle decides it
//! (`tui/src/lib.rs::app_server_target_for_launch`, `resolve_remote_addr`,
//! `cli/src/main.rs::resolve_remote_endpoint`): `--remote ADDR` an explicit
//! server, `--no-daemon` the embedded one, the command center (`agents`) the
//! local daemon, started when it is not running, and anything else the
//! embedded server.

use anyhow::Context as _;
use codex_app_server_client::RemoteAppServerEndpoint;
use codex_utils_absolute_path::AbsolutePathBuf;
use url::Url;

/// The server the app talks to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The in-process app server.
    Embedded,
    /// The local daemon (`codex app-server daemon`), started if needed.
    Daemon,
    /// A server at `endpoint`; `daemon`: it is the local daemon's socket.
    Remote {
        endpoint: RemoteAppServerEndpoint,
        daemon: bool,
    },
}

/// What the command line says about the connection.
pub struct Flags<'a> {
    pub no_daemon: bool,
    pub agents: bool,
    pub remote: Option<&'a str>,
    pub remote_auth_token_env: Option<&'a str>,
    /// `-c key=value`, raw.
    pub config_overrides: &'a [String],
}

/// What keeps the command center off the shared daemon, as the oracle words
/// it (`daemon_startup.rs::config_exclusion`): any `-c` override but the few
/// the daemon can take along. A malformed override is the config's error
/// to report, not an exclusion.
pub fn config_exclusion(raw: &[String]) -> Option<&'static str> {
    let overrides = codex_utils_cli::CliConfigOverrides {
        raw_overrides: raw.to_vec(),
    }
    .parse_overrides()
    .ok()?;
    let allowed = overrides.iter().all(|(key, value)| match key.as_str() {
        "suppress_unstable_features_warning" | "tui.fullscreen_transcript" => value.is_bool(),
        "tui" => value.as_table().is_some_and(|tui| {
            tui.len() == 1
                && tui
                    .get("fullscreen_transcript")
                    .is_some_and(codex_config::TomlValue::is_bool)
        }),
        "features" => value.as_table().is_some_and(|features| {
            !features.is_empty() && features.values().all(codex_config::TomlValue::is_bool)
        }),
        _ => {
            key.strip_prefix("features.")
                .is_some_and(|name| !name.contains('.'))
                && value.is_bool()
        }
    });
    (!allowed).then_some(OVERRIDES_EXCLUSION)
}

const OVERRIDES_EXCLUSION: &str =
    "command-line configuration overrides (-c, --enable, --disable, or --search)";

/// `startup_orchestration.rs` (`daemon_startup_warning`): the notice that the
/// launch runs without the shared background server, when it would have used it.
pub fn startup_warning(flags: &Flags<'_>) -> Option<String> {
    let uses_daemon = flags.agents && !flags.no_daemon && flags.remote.is_none();
    let reason = config_exclusion(flags.config_overrides).filter(|_| uses_daemon)?;
    Some(exclusion_warning(reason))
}

/// The one sentence of that notice, for whichever launch builds it.
pub fn exclusion_warning(reason: &str) -> String {
    format!("Running without the shared background server: {reason} requires embedded mode.")
}

/// What the launch asked of the connection, as `daemon_startup.rs::exclusion` reads it.
/// The oracle's other exclusions (`--profile`, `--strict-config`,
/// `--dangerously-bypass-hook-trust`, workload identity, a custom configuration
/// loader) belong to flags this port's command line does not take, so they cannot occur.
pub struct Launch<'a> {
    pub no_daemon: bool,
    pub agents: bool,
    pub oss: bool,
    /// An explicit `--remote` endpoint.
    pub remote: bool,
    /// `-c key=value`, raw.
    pub config_overrides: &'a [String],
    /// `CODEX_EXEC_SERVER_URL` is set.
    pub exec_server_url: bool,
}

/// What a launch without an explicit server does about the shared one.
#[derive(Debug, PartialEq, Eq)]
pub enum AutoStart {
    /// Start it when it is not running and attach (required).
    Daemon,
    /// Stay embedded. `exclusion` is what keeps it off the daemon, named in the
    /// startup warning only when auto-start was on. `reusable`: nothing excludes
    /// the daemon, so one that already listens may be attached (`daemon_auto_start`
    /// off).
    Embedded {
        exclusion: Option<&'static str>,
        reusable: bool,
    },
}

/// `startup_orchestration.rs`: `auto_start_daemon` (the feature, or the command
/// center; an implicit connection; no `--no-daemon`) and the exclusions that
/// turn it back to embedded. `feature_on`: `daemon_auto_start` in the config.
pub fn auto_start(launch: &Launch<'_>, feature_on: bool) -> AutoStart {
    let supported = cfg!(any(unix, windows));
    let auto = supported && (feature_on || launch.agents) && !launch.remote && !launch.no_daemon;
    let excluded = if launch.no_daemon {
        Some("--no-daemon")
    } else if launch.oss {
        Some("--oss")
    } else if launch.exec_server_url {
        Some("executor selection (CODEX_EXEC_SERVER_URL)")
    } else {
        config_exclusion(launch.config_overrides)
    };
    if auto && excluded.is_none() {
        return AutoStart::Daemon;
    }
    AutoStart::Embedded {
        exclusion: excluded.filter(|_| auto),
        reusable: supported && !launch.remote && excluded.is_none(),
    }
}

/// `lib.rs::maybe_probe_default_daemon_socket`: whether a daemon already listens
/// on `socket` (a connect that answers within 50 ms). Unix sockets only.
pub async fn probe_daemon(socket: Option<AbsolutePathBuf>) -> bool {
    let Some(socket) = socket else {
        return false;
    };
    probe_socket(socket.into_path_buf()).await
}

#[cfg(unix)]
async fn probe_socket(path: std::path::PathBuf) -> bool {
    matches!(
        tokio::time::timeout(
            std::time::Duration::from_millis(50),
            tokio::net::UnixStream::connect(path)
        )
        .await,
        Ok(Ok(_))
    )
}

#[cfg(not(unix))]
async fn probe_socket(_: std::path::PathBuf) -> bool {
    false
}

/// The default daemon's socket (`<CODEX_HOME>/...`), when the home resolves.
pub fn default_daemon_socket() -> Option<AbsolutePathBuf> {
    default_socket()
}

impl Mode {
    /// The mode for `flags`; `var` reads an environment variable.
    pub fn resolve(
        flags: &Flags<'_>,
        var: impl FnOnce(&str) -> Result<String, std::env::VarError>,
    ) -> anyhow::Result<Mode> {
        if flags.no_daemon && flags.remote.is_some() {
            anyhow::bail!("--no-daemon cannot be used with --remote.");
        }
        if flags.agents && flags.remote.is_some() && sets_writable_roots(flags.config_overrides) {
            anyhow::bail!(
                "`codex agents` cannot apply local provider or additional-directory overrides to a remote server"
            );
        }
        let Some(remote) = flags.remote else {
            if flags.remote_auth_token_env.is_some() {
                anyhow::bail!("`--remote-auth-token-env` requires `--remote`.");
            }
            return Ok(
                if flags.agents
                    && !flags.no_daemon
                    && config_exclusion(flags.config_overrides).is_none()
                {
                    Mode::Daemon
                } else {
                    Mode::Embedded
                },
            );
        };
        let mut endpoint = resolve_remote_addr(remote)?;
        if let Some(name) = flags.remote_auth_token_env {
            let supports = remote_addr_supports_auth_token(&endpoint);
            let RemoteAppServerEndpoint::WebSocket { auth_token, .. } = &mut endpoint else {
                anyhow::bail!(
                    "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote."
                );
            };
            if !supports {
                anyhow::bail!(
                    "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote."
                );
            }
            *auth_token = Some(read_token(name, var)?);
        }
        let daemon = matches!(&endpoint, RemoteAppServerEndpoint::UnixSocket { socket_path }
            if default_socket().as_ref() == Some(socket_path));
        Ok(Mode::Remote { endpoint, daemon })
    }
}

/// Whether a `-c` override sets the workspace-write roots, a local
/// additional directory a remote server cannot take (`cli/src/main.rs:1121`).
fn sets_writable_roots(raw: &[String]) -> bool {
    codex_utils_cli::CliConfigOverrides {
        raw_overrides: raw.to_vec(),
    }
    .parse_overrides()
    .is_ok_and(|overrides| {
        overrides.iter().any(|(key, value)| {
            key == "sandbox_workspace_write.writable_roots"
                || (key == "sandbox_workspace_write" && value.get("writable_roots").is_some())
        })
    })
}

/// The refusal of a command center on the embedded server when a terminal is
/// missing (`cli/src/main.rs:2595`, `AppExitInfo::fatal`).
pub fn terminal_refusal(
    flags: &Flags<'_>,
    mode: &Mode,
    stdin_tty: bool,
    stdout_tty: bool,
) -> Option<&'static str> {
    if !flags.agents || *mode != Mode::Embedded {
        return None;
    }
    if !stdin_tty {
        return Some("stdin is not a terminal");
    }
    (!stdout_tty).then_some("stdout is not a terminal")
}

fn default_socket() -> Option<AbsolutePathBuf> {
    let home = codex_core::config::find_codex_home().ok()?;
    codex_app_server_client::app_server_control_socket_path(&home).ok()
}

/// `cli/src/main.rs::read_remote_auth_token_from_env_var_with`.
fn read_token(
    name: &str,
    var: impl FnOnce(&str) -> Result<String, std::env::VarError>,
) -> anyhow::Result<String> {
    let token = var(name)
        .map_err(|_| anyhow::anyhow!("environment variable `{name}` is not set"))?
        .trim()
        .to_string();
    anyhow::ensure!(!token.is_empty(), "environment variable `{name}` is empty");
    Ok(token)
}

const INVALID: &str = "expected `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`";

/// `tui/src/lib.rs::resolve_remote_addr`.
pub fn resolve_remote_addr(addr: &str) -> anyhow::Result<RemoteAppServerEndpoint> {
    if let Some(path) = addr.strip_prefix("unix://") {
        let socket_path = if path.is_empty() {
            let home =
                codex_core::config::find_codex_home().context("failed to resolve CODEX_HOME")?;
            codex_app_server_client::app_server_control_socket_path(&home)?
        } else {
            AbsolutePathBuf::relative_to_current_dir(path)?
        };
        return Ok(RemoteAppServerEndpoint::UnixSocket { socket_path });
    }
    let invalid = || anyhow::anyhow!("invalid remote address `{addr}`; {INVALID}");
    let parsed = Url::parse(addr).map_err(|_| invalid())?;
    if matches!(parsed.scheme(), "ws" | "wss")
        && parsed.host_str().is_some()
        && has_explicit_port(addr, &parsed)
        && parsed.path() == "/"
        && parsed.query().is_none()
        && parsed.fragment().is_none()
    {
        return Ok(RemoteAppServerEndpoint::WebSocket {
            websocket_url: parsed.to_string(),
            auth_token: None,
        });
    }
    Err(invalid())
}

/// `tui/src/lib.rs::remote_addr_has_explicit_port`.
fn has_explicit_port(addr: &str, parsed: &Url) -> bool {
    let Some(host) = parsed.host_str() else {
        return false;
    };
    if parsed.port().is_some() {
        return true;
    }
    let Some((_, rest)) = addr.split_once("://") else {
        return false;
    };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let host_and_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host_and_port)| host_and_port);
    let default_port = match parsed.scheme() {
        "ws" => 80,
        "wss" => 443,
        _ => return false,
    };
    let expected_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    host_and_port == format!("{expected_host}:{default_port}")
}

/// `tui/src/lib.rs::remote_addr_supports_auth_token`.
fn remote_addr_supports_auth_token(endpoint: &RemoteAppServerEndpoint) -> bool {
    match endpoint {
        RemoteAppServerEndpoint::WebSocket { websocket_url, .. } => Url::parse(websocket_url)
            .is_ok_and(|parsed| match (parsed.scheme(), parsed.host()) {
                ("wss", Some(_)) => true,
                ("ws", Some(url::Host::Domain(domain))) => domain.eq_ignore_ascii_case("localhost"),
                ("ws", Some(url::Host::Ipv4(addr))) => addr.is_loopback(),
                ("ws", Some(url::Host::Ipv6(addr))) => addr.is_loopback(),
                _ => false,
            }),
        RemoteAppServerEndpoint::UnixSocket { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags<'a>(remote: Option<&'a str>, token: Option<&'a str>) -> Flags<'a> {
        Flags {
            no_daemon: false,
            agents: true,
            remote,
            remote_auth_token_env: token,
            config_overrides: &[],
        }
    }

    const ANALYTICS_OFF: &[&str] = &["analytics.enabled=false"];

    fn raw(overrides: &[&str]) -> Vec<String> {
        overrides.iter().map(ToString::to_string).collect()
    }

    fn env(_: &str) -> Result<String, std::env::VarError> {
        Ok(" secret \n".to_string())
    }

    /// `resolve_remote_addr_accepts_websocket_url`.
    #[test]
    fn resolve_remote_addr_accepts_websocket_url() {
        assert_eq!(
            resolve_remote_addr("ws://127.0.0.1:4500").unwrap(),
            RemoteAppServerEndpoint::WebSocket {
                websocket_url: "ws://127.0.0.1:4500/".to_string(),
                auth_token: None,
            }
        );
    }

    /// `resolve_remote_addr_accepts_secure_websocket_url`.
    #[test]
    fn resolve_remote_addr_accepts_secure_websocket_url() {
        assert_eq!(
            resolve_remote_addr("wss://example.com:443").unwrap(),
            RemoteAppServerEndpoint::WebSocket {
                websocket_url: "wss://example.com/".to_string(),
                auth_token: None,
            }
        );
    }

    /// `resolve_remote_addr_accepts_absolute_socket_path`.
    #[test]
    fn resolve_remote_addr_accepts_absolute_socket_path() {
        assert_eq!(
            resolve_remote_addr("unix:///tmp/codex.sock").unwrap(),
            RemoteAppServerEndpoint::UnixSocket {
                socket_path: AbsolutePathBuf::from_absolute_path("/tmp/codex.sock").unwrap(),
            }
        );
    }

    /// `resolve_remote_addr_rejects_invalid_remote_addresses`.
    #[test]
    fn resolve_remote_addr_rejects_invalid_remote_addresses() {
        for addr in [
            "ws://127.0.0.1",
            "wss://example.com",
            "127.0.0.1:4500",
            "https://127.0.0.1:4500",
        ] {
            let err = resolve_remote_addr(addr).unwrap_err();
            assert!(err.to_string().contains(INVALID), "{addr}");
        }
    }

    /// `cli/src/main.rs:1121`: a remote command center takes no local
    /// additional-directory override.
    #[test]
    fn a_remote_center_refuses_local_writable_roots() {
        let roots = raw(&["sandbox_workspace_write.writable_roots=[\"/x\"]"]);
        let table = raw(&["sandbox_workspace_write={writable_roots=[\"/x\"]}"]);
        for overrides in [&roots, &table] {
            let flags = Flags {
                config_overrides: overrides,
                ..flags(Some("ws://127.0.0.1:4500"), None)
            };
            let err = Mode::resolve(&flags, env).unwrap_err();
            assert_eq!(
                err.to_string(),
                "`codex agents` cannot apply local provider or additional-directory overrides to a remote server"
            );
        }
        let fine = raw(&["analytics.enabled=false"]);
        let flags = Flags {
            config_overrides: &fine,
            ..flags(Some("ws://127.0.0.1:4500"), None)
        };
        assert!(Mode::resolve(&flags, env).is_ok());
    }

    /// `cli/src/main.rs:2595`: the embedded command center needs terminals.
    #[test]
    fn the_embedded_center_needs_a_terminal() {
        let embedded = Flags {
            no_daemon: true,
            ..flags(None, None)
        };
        let mode = Mode::Embedded;
        assert_eq!(
            terminal_refusal(&embedded, &mode, false, true),
            Some("stdin is not a terminal")
        );
        assert_eq!(
            terminal_refusal(&embedded, &mode, true, false),
            Some("stdout is not a terminal")
        );
        assert_eq!(terminal_refusal(&embedded, &mode, true, true), None);
        // The chat and the daemon are not asked.
        let chat = Flags {
            agents: false,
            ..flags(None, None)
        };
        assert_eq!(terminal_refusal(&chat, &mode, false, false), None);
        assert_eq!(
            terminal_refusal(&flags(None, None), &Mode::Daemon, false, false),
            None
        );
    }

    /// `startup_orchestration.rs:16`: the two flags exclude each other.
    #[test]
    fn no_daemon_with_remote_is_refused() {
        let flags = Flags {
            no_daemon: true,
            ..flags(Some("ws://127.0.0.1:4500"), None)
        };
        let err = Mode::resolve(&flags, env).unwrap_err();
        assert_eq!(err.to_string(), "--no-daemon cannot be used with --remote.");
    }

    /// `app_server_target_for_launch`: the command center uses the daemon,
    /// `--no-daemon` the embedded server, an explicit address that server.
    #[test]
    fn the_command_center_uses_the_daemon_unless_told_otherwise() {
        assert_eq!(
            Mode::resolve(&flags(None, None), env).unwrap(),
            Mode::Daemon
        );
        let embedded = Flags {
            no_daemon: true,
            ..flags(None, None)
        };
        assert_eq!(Mode::resolve(&embedded, env).unwrap(), Mode::Embedded);
        let chat = Flags {
            agents: false,
            ..flags(None, None)
        };
        assert_eq!(Mode::resolve(&chat, env).unwrap(), Mode::Embedded);
        let remote = Mode::resolve(&flags(Some("ws://127.0.0.1:4500"), None), env).unwrap();
        assert_eq!(
            remote,
            Mode::Remote {
                endpoint: RemoteAppServerEndpoint::WebSocket {
                    websocket_url: "ws://127.0.0.1:4500/".into(),
                    auth_token: None
                },
                daemon: false
            }
        );
    }

    /// `prefer_explicit_remote_endpoint`: the daemon's own socket named with
    /// `--remote unix://` is the local daemon (its notice points at `/daemon`).
    #[test]
    fn the_daemons_own_socket_is_the_local_daemon() {
        let Some(socket) = default_socket() else {
            return;
        };
        let addr = format!("unix://{}", socket.display());
        let mode = Mode::resolve(&flags(Some(&addr), None), env).unwrap();
        assert!(matches!(mode, Mode::Remote { daemon: true, .. }));
    }

    /// `cli/src/main.rs`: the token comes from the named variable, trimmed,
    /// and only for `wss://` and loopback `ws://`.
    #[test]
    fn the_auth_token_is_read_from_the_variable_for_secure_remotes_only() {
        let mode = Mode::resolve(&flags(Some("wss://example.com:443"), Some("T")), env).unwrap();
        assert!(matches!(mode, Mode::Remote {
            endpoint: RemoteAppServerEndpoint::WebSocket { auth_token: Some(token), .. }, ..
        } if token == "secret"));
        let err = Mode::resolve(&flags(Some("ws://example.com:80"), Some("T")), env).unwrap_err();
        assert_eq!(
            err.to_string(),
            "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote."
        );
        let err = Mode::resolve(&flags(None, Some("T")), env).unwrap_err();
        assert_eq!(
            err.to_string(),
            "`--remote-auth-token-env` requires `--remote`."
        );
        let missing = |_: &str| Err(std::env::VarError::NotPresent);
        let err =
            Mode::resolve(&flags(Some("wss://example.com:443"), Some("T")), missing).unwrap_err();
        assert_eq!(err.to_string(), "environment variable `T` is not set");
        let empty = |_: &str| Ok("  ".to_string());
        let err =
            Mode::resolve(&flags(Some("wss://example.com:443"), Some("T")), empty).unwrap_err();
        assert_eq!(err.to_string(), "environment variable `T` is empty");
    }

    /// `feature_overrides_allow_daemon_without_allowing_arbitrary_config`.
    #[test]
    fn feature_overrides_allow_daemon_without_allowing_arbitrary_config() {
        for (raw_override, eligible) in [
            ("features.app_server_websocket_compression=true", true),
            ("features.app_server_websocket_compression=false", true),
            ("features.worktrees='true'", false),
            ("features={worktrees=true,shell_tool=false}", true),
            ("suppress_unstable_features_warning=true", true),
            ("suppress_unstable_features_warning='true'", false),
            ("features.shell_tool=false", true),
            ("features.worktrees.enabled=true", false),
            ("tui.fullscreen_transcript=true", true),
            ("tui.fullscreen_transcript=false", true),
            ("tui.fullscreen_transcript=\"true\"", false),
            ("tui={fullscreen_transcript=true}", true),
            ("tui={fullscreen_transcript='true'}", false),
            ("tui={fullscreen_transcript=true,animations=false}", false),
            ("features={}", false),
            ("model='test'", false),
            ("analytics.enabled=false", false),
        ] {
            assert_eq!(
                config_exclusion(&raw(&[raw_override])).is_none(),
                eligible,
                "{raw_override}"
            );
        }
        assert_eq!(
            config_exclusion(&raw(ANALYTICS_OFF)),
            Some("command-line configuration overrides (-c, --enable, --disable, or --search)")
        );
    }

    /// The scenario `agents`: the oracle run as `codex -c analytics.enabled=false
    /// agents` takes the embedded server (its overrides are not the daemon's),
    /// and says so; ours does the same with the same command line, and a
    /// plain `agents` still uses the daemon.
    #[test]
    fn overrides_the_daemon_cannot_take_keep_the_command_center_embedded() {
        let overrides = raw(ANALYTICS_OFF);
        let with_override = Flags {
            config_overrides: &overrides,
            ..flags(None, None)
        };
        assert_eq!(Mode::resolve(&with_override, env).unwrap(), Mode::Embedded);
        assert_eq!(
            startup_warning(&with_override).as_deref(),
            Some(
                "Running without the shared background server: command-line configuration overrides (-c, --enable, --disable, or --search) requires embedded mode."
            )
        );
        assert_eq!(startup_warning(&flags(None, None)), None);
        let explicit = Flags {
            no_daemon: true,
            ..with_override
        };
        assert_eq!(startup_warning(&explicit), None, "--no-daemon asked for it");
        let chat = Flags {
            agents: false,
            ..with_override
        };
        assert_eq!(
            startup_warning(&chat),
            None,
            "the chat never used the daemon"
        );
    }

    fn launch<'a>(overrides: &'a [String]) -> Launch<'a> {
        Launch {
            no_daemon: false,
            agents: false,
            oss: false,
            remote: false,
            config_overrides: overrides,
            exec_server_url: false,
        }
    }

    const EMBEDDED: AutoStart = AutoStart::Embedded {
        exclusion: None,
        reusable: true,
    };

    /// `daemon_auto_start` is Stable and on by default: a plain chat starts the shared server.
    #[cfg(any(unix, windows))]
    #[test]
    fn chat_auto_starts_the_daemon_by_default() {
        assert_eq!(auto_start(&launch(&[]), true), AutoStart::Daemon);
        let features = raw(&[
            "features.shell_tool=false",
            "tui.fullscreen_transcript=true",
        ]);
        assert_eq!(auto_start(&launch(&features), true), AutoStart::Daemon);
    }

    /// A pin move that flips the feature's default is red here.
    #[test]
    fn daemon_auto_start_is_on_by_default() {
        assert!(codex_features::Feature::DaemonAutoStart.default_enabled());
    }

    /// `daemon_startup_tests.rs::daemon_eligibility_preserves_launch_options_and_explains_exclusions`
    /// (`--profile`, `--strict-config`, `--dangerously-bypass-hook-trust`, workload identity and
    /// a custom loader are flags this command line does not take).
    #[cfg(any(unix, windows))]
    #[test]
    fn daemon_eligibility_preserves_launch_options_and_explains_exclusions() {
        let excluded = |exclusion| AutoStart::Embedded {
            exclusion: Some(exclusion),
            reusable: false,
        };
        let none: Vec<String> = Vec::new();
        let oss = Launch {
            oss: true,
            ..launch(&none)
        };
        assert_eq!(auto_start(&oss, true), excluded("--oss"));
        let exec = Launch {
            exec_server_url: true,
            ..launch(&none)
        };
        assert_eq!(
            auto_start(&exec, true),
            excluded("executor selection (CODEX_EXEC_SERVER_URL)")
        );
        let search = raw(&["web_search=live"]);
        assert_eq!(
            auto_start(&launch(&search), true),
            excluded(OVERRIDES_EXCLUSION)
        );
        // `--oss` is named before the overrides.
        let both = Launch {
            oss: true,
            ..launch(&search)
        };
        assert_eq!(auto_start(&both, true), excluded("--oss"));
    }

    /// With the feature off nothing starts, and an exclusion is nothing to warn about; a
    /// launch that nothing excludes may still attach to a daemon that already runs.
    #[cfg(any(unix, windows))]
    #[test]
    fn feature_off_chat_stays_embedded_without_a_warning() {
        assert_eq!(auto_start(&launch(&[]), false), EMBEDDED);
        let search = raw(&["web_search=live"]);
        assert_eq!(
            auto_start(&launch(&search), false),
            AutoStart::Embedded {
                exclusion: None,
                reusable: false
            }
        );
        let oss = Launch {
            oss: true,
            ..launch(&[])
        };
        assert_eq!(
            auto_start(&oss, false),
            AutoStart::Embedded {
                exclusion: None,
                reusable: false
            }
        );
    }

    /// `an_excluded_chat_warns_only_when_auto_start_was_on`, and `--no-daemon` never does.
    #[cfg(any(unix, windows))]
    #[test]
    fn an_excluded_chat_warns_only_when_auto_start_was_on() {
        let search = raw(&["web_search=live"]);
        let AutoStart::Embedded { exclusion, .. } = auto_start(&launch(&search), true) else {
            panic!("embedded");
        };
        assert_eq!(
            exclusion.map(exclusion_warning).as_deref(),
            Some(
                "Running without the shared background server: command-line configuration overrides (-c, --enable, --disable, or --search) requires embedded mode."
            )
        );
        let no_daemon = Launch {
            no_daemon: true,
            ..launch(&search)
        };
        for feature_on in [true, false] {
            assert_eq!(
                auto_start(&no_daemon, feature_on),
                AutoStart::Embedded {
                    exclusion: None,
                    reusable: false
                }
            );
        }
    }

    #[test]
    fn an_explicit_remote_is_never_auto_started() {
        let remote = Launch {
            remote: true,
            ..launch(&[])
        };
        for feature_on in [true, false] {
            assert_eq!(
                auto_start(&remote, feature_on),
                AutoStart::Embedded {
                    exclusion: None,
                    reusable: false
                }
            );
        }
    }

    /// The command center asks for the daemon whatever the feature says.
    #[cfg(any(unix, windows))]
    #[test]
    fn agents_keeps_its_own_decision() {
        let agents = Launch {
            agents: true,
            ..launch(&[])
        };
        assert_eq!(auto_start(&agents, false), AutoStart::Daemon);
        assert_eq!(
            Mode::resolve(&flags(None, None), env).unwrap(),
            Mode::Daemon
        );
    }

    /// `maybe_probe_default_daemon_socket`: a listener answers, a missing socket does not.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_running_daemon_is_probed_and_a_missing_one_is_not() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("control.sock");
        let socket = AbsolutePathBuf::from_absolute_path_checked(&path).expect("absolute");
        assert!(!probe_daemon(Some(socket.clone())).await);
        assert!(!probe_daemon(None).await);
        let _listener = tokio::net::UnixListener::bind(&path).expect("a listener");
        assert!(probe_daemon(Some(socket)).await);
    }
}
