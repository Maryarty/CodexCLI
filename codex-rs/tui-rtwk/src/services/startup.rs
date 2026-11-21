//! The startup page's file and process effects (R12: IO lives in services):
//! the update cache (`version.json`: what is offered, what was dismissed)
//! (the update run itself is the host's, `host/src/update.rs`). The host
//! reads the config and the install method and calls these. Oracle:
//! `updates_cache.rs`, `update_action.rs`, the CLI's `run_update_action`.

use std::path::Path;

use chrono::DateTime;
use chrono::Utc;
use codex_tui_core::protocol::onboarding::VersionCache;

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

/// `update_action.rs::UpdateAction` without the daemon's (the local daemon
/// update is not part of this prompt).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateAction {
    NpmGlobalLatest,
    BunGlobalLatest,
    VitePlusGlobalLatest,
    PnpmGlobalLatest,
    BrewUpgrade,
    StandaloneUnix,
    StandaloneWindows,
}

impl UpdateAction {
    /// `UpdateAction::command_args`.
    pub fn command_args(self) -> (&'static str, &'static [&'static str]) {
        match self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_npm_command_reads_as_the_prompt_shows_it() {
        assert_eq!(
            UpdateAction::NpmGlobalLatest.command_str(),
            "npm install -g @openai/codex"
        );
        assert_eq!(
            UpdateAction::BrewUpgrade.command_str(),
            "brew upgrade --cask codex"
        );
    }

    #[test]
    fn a_dismissal_keeps_the_cache_and_marks_the_version() {
        let dir = std::env::temp_dir().join(format!("rtwk-version-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let file = dir.join("version.json");
        std::fs::write(
            &file,
            r#"{"latest_version":"9.9.9","last_checked_at":"2026-01-01T00:00:00Z"}"#,
        )
        .expect("the cache");
        dismiss_version(&file, "9.9.9").expect("dismissed");
        assert_eq!(
            read_version_cache(&file),
            Some(VersionCache {
                latest: "9.9.9".into(),
                dismissed: Some("9.9.9".into()),
            })
        );
        let text = std::fs::read_to_string(&file).expect("the cache");
        assert!(text.contains("2026-01-01T00:00:00Z"), "{text}");
        // No cache yet: one is written with the version dismissed.
        let fresh = dir.join("none").join("version.json");
        dismiss_version(&fresh, "1.2.3").expect("dismissed");
        assert_eq!(
            read_version_cache(&fresh).and_then(|cache| cache.dismissed),
            Some("1.2.3".into())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rtwk-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        dir
    }

    fn at(text: &str) -> DateTime<Utc> {
        text.parse().expect("a time")
    }

    // `get_upgrade_version`: `None => true`, else `last_checked_at < now - 20 h`.
    #[test]
    fn a_cache_is_stale_when_missing_unreadable_or_older_than_twenty_hours() {
        let dir = temp("stale");
        let file = dir.join("version.json");
        let now = at("2026-11-21T20:00:00Z");
        assert!(version_cache_is_stale(&file, now), "missing");
        std::fs::write(&file, "not json").expect("the cache");
        assert!(version_cache_is_stale(&file, now), "unreadable");
        std::fs::write(&file, r#"{"latest_version":"1.0.0"}"#).expect("the cache");
        assert!(version_cache_is_stale(&file, now), "no check time");
        let checked =
            |time: &str| format!(r#"{{"latest_version":"1.0.0","last_checked_at":"{time}"}}"#);
        std::fs::write(&file, checked("2026-11-21T00:00:01Z")).expect("the cache");
        assert!(!version_cache_is_stale(&file, now), "19 h 59 min old");
        std::fs::write(&file, checked("2026-11-20T23:59:59Z")).expect("the cache");
        assert!(version_cache_is_stale(&file, now), "20 h 1 s old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // `check_for_update`: the written cache keeps what was dismissed and
    // carries the check time as RFC 3339; it creates the folder.
    #[test]
    fn a_refresh_keeps_the_dismissed_version_and_stamps_the_check() {
        let dir = temp("refresh");
        let file = dir.join("deep").join("version.json");
        let now = at("2026-11-21T20:00:00Z");
        write_version_cache(&file, "2.0.0", now).expect("written");
        assert_eq!(
            read_version_cache(&file),
            Some(VersionCache {
                latest: "2.0.0".into(),
                dismissed: None,
            })
        );
        dismiss_version(&file, "2.0.0").expect("dismissed");
        let later = at("2026-11-22T20:00:00Z");
        write_version_cache(&file, "2.1.0", later).expect("written");
        assert_eq!(
            read_version_cache(&file),
            Some(VersionCache {
                latest: "2.1.0".into(),
                dismissed: Some("2.0.0".into()),
            })
        );
        assert!(!version_cache_is_stale(&file, later));
        let text = std::fs::read_to_string(&file).expect("the cache");
        assert!(
            text.ends_with('\n') && text.contains("2026-11-22T20:00:00Z"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // update_versions.rs tests `extracts_version_from_latest_tag`,
    // `latest_tag_without_prefix_is_invalid`.
    #[test]
    fn extracts_version_from_latest_tag() {
        assert_eq!(
            latest_from_release_tag("rust-v1.5.0"),
            Ok("1.5.0".to_owned())
        );
    }

    #[test]
    fn latest_tag_without_prefix_is_invalid() {
        assert!(latest_from_release_tag("v1.5.0").is_err());
    }

    fn version_json(version: &str) -> serde_json::Value {
        serde_json::json!({
            "dist": {
                "integrity": format!("sha512-{version}"),
                "tarball": format!("https://registry.npmjs.org/@openai/codex/-/codex-{version}.tgz"),
            }
        })
    }

    fn package(github_latest: &str, npm_latest: &str) -> serde_json::Value {
        serde_json::json!({
            "dist-tags": { "latest": npm_latest },
            "versions": { github_latest: version_json(github_latest) },
        })
    }

    // npm_registry.rs tests `ready_version_requires_latest_dist_tag_and_root_dist`,
    // `ready_version_rejects_stale_latest_dist_tag`,
    // `ready_version_rejects_missing_root_dist`.
    #[test]
    fn ready_version_requires_latest_dist_tag_and_root_dist() {
        assert_eq!(
            ensure_npm_version_ready(&package("1.2.3", "1.2.3"), "1.2.3"),
            Ok(())
        );
    }

    #[test]
    fn ready_version_rejects_stale_latest_dist_tag() {
        let error = ensure_npm_version_ready(&package("1.2.3", "1.2.2"), "1.2.3")
            .expect_err("npm latest dist-tag must match GitHub latest");
        assert!(error.contains("latest dist-tag"), "{error}");
    }

    #[test]
    fn ready_version_rejects_missing_root_dist() {
        let package = serde_json::json!({
            "dist-tags": { "latest": "1.2.3" },
            "versions": { "1.2.3": {} },
        });
        let error = ensure_npm_version_ready(&package, "1.2.3")
            .expect_err("root package must have dist metadata");
        assert!(error.contains("missing dist metadata"), "{error}");
    }

    #[test]
    fn ready_version_rejects_missing_tarball_and_integrity() {
        let missing = |dist: serde_json::Value| {
            serde_json::json!({
                "dist-tags": { "latest": "1.2.3" },
                "versions": { "1.2.3": { "dist": dist } },
            })
        };
        let error =
            ensure_npm_version_ready(&missing(serde_json::json!({"integrity": "x"})), "1.2.3")
                .expect_err("a tarball is needed");
        assert!(error.contains("missing dist.tarball"), "{error}");
        let error =
            ensure_npm_version_ready(&missing(serde_json::json!({"tarball": "x"})), "1.2.3")
                .expect_err("an integrity is needed");
        assert!(error.contains("missing dist.integrity"), "{error}");
        let error = ensure_npm_version_ready(&serde_json::json!({"versions": {}}), "1.2.3")
            .expect_err("a latest dist-tag is needed");
        assert!(error.contains("missing latest dist-tag"), "{error}");
    }
}
