//! The startup page's file and process effects (R12: IO lives in services):
//! the update cache (`version.json`: what is offered, what was dismissed)
//! (the update run itself is the host's, `host/src/update.rs`). The host
//! reads the config and the install method and calls these. Oracle:
//! `updates_cache.rs`, `update_action.rs`, the CLI's `run_update_action`.

use std::path::Path;
use std::sync::Arc;

use chrono::DateTime;
use chrono::Utc;
use codex_tui_core::app::effects::DaemonSource as Source;
use codex_tui_core::protocol::onboarding::VersionCache;

/// Opens a URL in the user's browser (`webbrowser::open`); blocking.
pub type Browser = Arc<dyn Fn(&str) -> std::io::Result<()> + Send + Sync>;

/// `show_chatgpt_login_in_browser`: opens `url` with `browser`. The open may
/// start a process and wait, so it runs off the async workers (R1: the
/// blocking pool is the services', not the host's). A failure is returned
/// for the caller to log, never answered to the page.
pub async fn open_in_browser(browser: Browser, url: String) -> Result<(), String> {
    match tokio::task::spawn_blocking(move || browser(&url)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("failed to open the login URL: {error}")),
        Err(error) => Err(format!("the browser open did not finish: {error}")),
    }
}

/// Whether a browser can be opened (`webbrowser::Browser::is_available`);
/// blocking, so it runs off the async workers like the open (R1). A check
/// that did not finish counts as no browser.
pub async fn browser_available(check: fn() -> bool) -> bool {
    tokio::task::spawn_blocking(check).await.unwrap_or(false)
}

/// The update cache at `file`: the latest release and the one dismissed.
pub fn read_version_cache(file: &Path) -> Option<VersionCache> {
    let info: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    Some(VersionCache {
        latest: info["latest_version"].as_str()?.to_owned(),
        dismissed: info["dismissed_version"].as_str().map(str::to_owned),
    })
}

/// `updates_cache.rs::dismiss_version`: the cache keeps what it had, with
/// `version` dismissed.
pub fn dismiss_version(file: &Path, version: &str) -> std::io::Result<()> {
    let mut info = std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| {
            serde_json::json!({
                "latest_version": version,
                "last_checked_at": "1970-01-01T00:00:00Z",
            })
        });
    info["dismissed_version"] = serde_json::Value::String(version.to_owned());
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, format!("{info}\n"))
}

/// `updates.rs::get_upgrade_version`: the cache is refreshed in the
/// background when it is missing, unreadable, or was checked more than this
/// many hours ago.
pub const VERSION_REFRESH_AFTER_HOURS: i64 = 20;

/// Whether the cache at `file` needs a refresh at `now` (`read_version_info`
/// fails, or `last_checked_at < now - 20 h`).
pub fn version_cache_is_stale(file: &Path, now: DateTime<Utc>) -> bool {
    let checked = std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(|info| info["latest_version"].is_string())
        .and_then(|info| {
            info["last_checked_at"]
                .as_str()?
                .parse::<DateTime<Utc>>()
                .ok()
        });
    checked
        .is_none_or(|checked| checked < now - chrono::Duration::hours(VERSION_REFRESH_AFTER_HOURS))
}

/// `update_versions.rs::extract_version_from_latest_tag`: the release tag of
/// the GitHub release is `rust-v<version>`.
pub fn latest_from_release_tag(tag: &str) -> Result<String, String> {
    tag.strip_prefix("rust-v")
        .map(str::to_owned)
        .ok_or_else(|| format!("Failed to parse latest tag name '{tag}'"))
}

/// `npm_registry.rs::ensure_version_ready`: the npm package's `latest`
/// dist-tag is the GitHub release and that version has its tarball and
/// integrity.
pub fn ensure_npm_version_ready(package: &serde_json::Value, version: &str) -> Result<(), String> {
    let version = version.trim();
    match package["dist-tags"]["latest"].as_str() {
        Some(latest) if latest == version => {}
        Some(latest) => {
            return Err(format!(
                "npm latest dist-tag points to {latest}, expected GitHub release {version}"
            ));
        }
        None => return Err("npm package is missing latest dist-tag".to_owned()),
    }
    let info = package["versions"]
        .get(version)
        .ok_or_else(|| format!("npm package version {version} is missing"))?;
    let Some(dist) = info.get("dist").filter(|dist| !dist.is_null()) else {
        return Err(format!(
            "npm package version {version} is missing dist metadata"
        ));
    };
    let has = |key: &str| dist[key].as_str().is_some_and(|text| !text.is_empty());
    if !has("tarball") {
        return Err(format!(
            "npm package version {version} is missing dist.tarball"
        ));
    }
    if !has("integrity") {
        return Err(format!(
            "npm package version {version} is missing dist.integrity"
        ));
    }
    Ok(())
}

/// `updates.rs::check_for_update`'s write: the cache with `latest` checked at
/// `now`, keeping a dismissed version.
pub fn write_version_cache(file: &Path, latest: &str, now: DateTime<Utc>) -> std::io::Result<()> {
    let dismissed = std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|info| info["dismissed_version"].as_str().map(str::to_owned));
    let line = serde_json::json!({
        "latest_version": latest,
        "last_checked_at": now.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
        "dismissed_version": dismissed,
    });
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, format!("{line}\n"))
}

/// Whether the cache at `file` is due a refresh now (the clock is read here:
/// R11).
pub fn version_cache_needs_refresh(file: &Path) -> bool {
    version_cache_is_stale(file, Utc::now())
}

/// `write_version_cache` for a check made now.
pub fn store_latest_version(file: &Path, latest: &str) -> std::io::Result<()> {
    write_version_cache(file, latest, Utc::now())
}

/// The CLI's update run, once the terminal is restored: `program` with
/// `args` in `cwd`, announced as `command`.
pub fn run_update_process(
    command: &str,
    program: &Path,
    args: &[String],
    cwd: &Path,
) -> Result<(), String> {
    println!();
    println!("Updating Codex via `{command}`...");
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("`{command}` failed with status {status}"));
    }
    println!("\n🎉 Update ran successfully! Please restart Codex.");
    Ok(())
}

/// `update_action.rs::UpdateAction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateAction {
    /// Replace the local daemon after restoring the terminal (`/daemon` "Update and exit").
    Daemon(Source),
    NpmGlobalLatest,
    BunGlobalLatest,
    VitePlusGlobalLatest,
    PnpmGlobalLatest,
    BrewUpgrade,
    StandaloneUnix,
    StandaloneWindows,
}

/// `DaemonUpdateSource::command_args`: what `codex` runs to update the daemon from `source`.
fn daemon_args(source: Source) -> &'static [&'static str] {
    match source {
        Source::PublicStable => &["app-server", "daemon", "update"],
        Source::ThisCli => &["app-server", "daemon", "update", "--from-cli", "--yes"],
    }
}

impl UpdateAction {
    /// `UpdateAction::command_args`.
    pub fn command_args(self) -> (&'static str, &'static [&'static str]) {
        match self {
            UpdateAction::Daemon(source) => ("codex", daemon_args(source)),
            UpdateAction::NpmGlobalLatest => ("npm", &["install", "-g", "@openai/codex"]),
            UpdateAction::BunGlobalLatest => ("bun", &["install", "-g", "@openai/codex"]),
            UpdateAction::VitePlusGlobalLatest => ("vp", &["install", "-g", "@openai/codex"]),
            UpdateAction::PnpmGlobalLatest => ("pnpm", &["add", "-g", "@openai/codex"]),
            UpdateAction::BrewUpgrade => ("brew", &["upgrade", "--cask", "codex"]),
            UpdateAction::StandaloneUnix => (
                "sh",
                &[
                    "-c",
                    r#"s=$(curl -fsSL https://chatgpt.com/codex/install.sh) && CODEX_NON_INTERACTIVE=1 eval "$s""#,
                ],
            ),
            UpdateAction::StandaloneWindows => (
                "powershell",
                &[
                    "-ExecutionPolicy",
                    "Bypass",
                    "-c",
                    "$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 | iex",
                ],
            ),
        }
    }

    /// `UpdateAction::command_str`: the command as the prompt shows it.
    pub fn command_str(self) -> String {
        let (command, args) = self.command_args();
        shlex::try_join(std::iter::once(command).chain(args.iter().copied()))
            .unwrap_or_else(|_| format!("{command} {}", args.join(" ")))
    }
}
