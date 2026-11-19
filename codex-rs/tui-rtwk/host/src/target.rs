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
pub fn config_exclusion(_raw: &[String]) -> Option<&'static str> {
    None
}

/// `startup_orchestration.rs` (`daemon_startup_warning`): the notice that the
/// launch runs without the shared background server, when it would have used it.
pub fn startup_warning(_flags: &Flags<'_>) -> Option<String> {
    None
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
        let Some(remote) = flags.remote else {
            if flags.remote_auth_token_env.is_some() {
                anyhow::bail!("`--remote-auth-token-env` requires `--remote`.");
            }
            return Ok(if flags.agents && !flags.no_daemon {
                Mode::Daemon
            } else {
                Mode::Embedded
            });
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
        assert_eq!(startup_warning(&chat), None, "the chat never used the daemon");
    }
}
