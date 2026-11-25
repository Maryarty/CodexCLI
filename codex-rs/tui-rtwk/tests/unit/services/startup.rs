use chrono::DateTime;
use chrono::Utc;
use codex_tui_core::protocol::onboarding::VersionCache;
use codex_tui_rtwk::services::startup::*;
use std::path::Path;
use std::sync::Arc;

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
    let error = ensure_npm_version_ready(&missing(serde_json::json!({"integrity": "x"})), "1.2.3")
        .expect_err("a tarball is needed");
    assert!(error.contains("missing dist.tarball"), "{error}");
    let error = ensure_npm_version_ready(&missing(serde_json::json!({"tarball": "x"})), "1.2.3")
        .expect_err("an integrity is needed");
    assert!(error.contains("missing dist.integrity"), "{error}");
    let error = ensure_npm_version_ready(&serde_json::json!({"versions": {}}), "1.2.3")
        .expect_err("a latest dist-tag is needed");
    assert!(error.contains("missing latest dist-tag"), "{error}");
}

// ------------------------------------------------ oracle `update_action.rs` tests

#[test]
fn standalone_windows_update_command_reruns_latest_installer() {
    assert_eq!(
        UpdateAction::StandaloneWindows.command_args(),
        (
            "powershell",
            &[
                "-ExecutionPolicy",
                "Bypass",
                "-c",
                "$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 | iex"
            ][..],
        )
    );
}

#[cfg(unix)]
#[test]
fn standalone_unix_update_fails_when_installer_download_fails() {
    let (command, args) = UpdateAction::StandaloneUnix.command_args();
    let offline_command = args[1].replace("https://chatgpt.com", "file:///dev/null");
    assert_ne!(offline_command, args[1], "installer URL must be replaced");
    let output = std::process::Command::new(command)
        .args([args[0], &offline_command])
        .env_clear()
        .env("HOME", "/dev/null")
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("run standalone Unix update command");

    assert!(!output.status.success(), "{output:?}");
}

// `DaemonUpdateSource::command_args`: what the daemon update runs, and how the notice reads it.
#[test]
fn a_daemon_update_runs_the_app_server_daemon_command() {
    use codex_tui_core::app::effects::DaemonSource as Source;
    assert_eq!(
        UpdateAction::Daemon(Source::PublicStable).command_args(),
        ("codex", &["app-server", "daemon", "update"][..])
    );
    assert_eq!(
        UpdateAction::Daemon(Source::ThisCli).command_args(),
        (
            "codex",
            &["app-server", "daemon", "update", "--from-cli", "--yes"][..]
        )
    );
    assert_eq!(
        UpdateAction::Daemon(Source::ThisCli).command_str(),
        "codex app-server daemon update --from-cli --yes"
    );
}

// ------------------------------------------------ oracle `updates_cache_tests.rs`

#[test]
fn dismiss_version_creates_cache_file_when_missing() {
    let dir = temp("dismiss-missing");
    let file = dir.join("version.json");
    dismiss_version(&file, "999.0.0").expect("dismiss version");
    let info: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("the cache")).expect("json");
    assert_eq!(
        at(info["last_checked_at"].as_str().expect("a time")),
        DateTime::<Utc>::UNIX_EPOCH
    );
    assert_eq!(
        (
            info["latest_version"].as_str(),
            info["dismissed_version"].as_str()
        ),
        (Some("999.0.0"), Some("999.0.0"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------ oracle `daemon_update_tests.rs`

// The CLI's `run_update_action` runs a package update from `/`, outside the project: the
// package manager sees none of its config.
#[cfg(unix)]
#[test]
fn package_update_runs_outside_the_project() {
    use std::os::unix::fs::PermissionsExt;
    let bin = temp("package-update");
    let pnpm = bin.join("pnpm");
    std::fs::write(&pnpm, "#!/bin/sh\npwd -P > \"$0.cwd\"\n").expect("the script");
    std::fs::set_permissions(&pnpm, std::fs::Permissions::from_mode(0o755)).expect("executable");
    run_update_process("pnpm add -g @openai/codex", &pnpm, &[], Path::new("/"))
        .expect("the update ran");
    let cwd = std::fs::read_to_string(bin.join("pnpm.cwd")).expect("the script ran");
    assert_eq!(cwd.trim_end(), "/");
    let _ = std::fs::remove_dir_all(&bin);
}
