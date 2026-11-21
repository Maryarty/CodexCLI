//! The startup page's file and process effects (R12: IO lives in services):
//! the update cache (`version.json`: what is offered, what was dismissed)
//! and the update run once the terminal is restored. The host reads the
//! config and the install method and calls these. Oracle:
//! `updates_cache.rs`, `update_action.rs`, the CLI's `run_update_action`.

use std::path::Path;

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

/// The CLI's `run_update_action` (non-Windows): after the terminal is
/// restored, the update runs from `/` so no project config steers it.
pub fn run_update(action: UpdateAction) -> Result<(), String> {
    println!();
    let command = action.command_str();
    println!("Updating Codex via `{command}`...");
    let (program, args) = action.command_args();
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir("/")
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("`{command}` failed with status {status}"));
    }
    println!("\n🎉 Update ran successfully! Please restart Codex.");
    Ok(())
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
}
