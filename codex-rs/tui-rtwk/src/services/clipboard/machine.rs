//! The machine behind the routes: arboard's native clipboard, PowerShell on
//! WSL, tmux, X11 PRIMARY (oracle `clipboard_copy.rs` `arboard_copy`,
//! `clipboard_copy/primary.rs`). The worker thread makes every call here.

use std::time::Instant;

use super::Environment;
use super::Lease;
use super::Outcome;
use super::Routes;
use super::tmux;
use super::wsl;

/// The platform's clipboards.
#[derive(Default)]
pub struct Machine {
    /// The native clipboard `prepare` opened, or why it could not be opened:
    /// the failure is the native route's, not the whole copy's, so a terminal
    /// route still runs.
    #[cfg(not(target_os = "android"))]
    prepared: Option<Result<arboard::Clipboard, String>>,
}

/// What the routes read from the process: the variables, and the WSL probe.
pub fn environment() -> Environment {
    Environment::from_lookup(
        |name| std::env::var_os(name).map(|value| value.to_string_lossy().into_owned()),
        is_wsl_session(),
    )
}

/// Whether the process runs under WSL (oracle `is_probably_wsl`): the kernel
/// says so, or a WSL variable is set (custom kernels).
#[cfg(target_os = "linux")]
pub fn is_wsl_session() -> bool {
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let version = version.to_lowercase();
        if version.contains("microsoft") || version.contains("wsl") {
            return true;
        }
    }
    std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some()
}

#[cfg(not(target_os = "linux"))]
pub fn is_wsl_session() -> bool {
    false
}

#[cfg(not(target_os = "android"))]
impl Routes for Machine {
    fn prepare(&mut self, environment: Environment) -> Result<(), String> {
        // Bound WSL native setup: even initialization can block forever, so WSL
        // opens it later, within its own budget.
        self.prepared = (!environment.wsl).then(|| {
            arboard::Clipboard::new().map_err(|error| format!("clipboard unavailable: {error}"))
        });
        Ok(())
    }

    fn native(&mut self, text: &str, html: Option<&str>) -> Result<Option<Lease>, String> {
        let clipboard = match self.prepared.take() {
            Some(prepared) => prepared?,
            None => wsl::native_clipboard()?,
        };
        arboard_copy(clipboard, text, html)
    }

    fn windows(&mut self, text: &str, deadline: Instant) -> Result<(), String> {
        wsl::copy(text, deadline)
    }

    fn tmux(&mut self, text: &str) -> Result<(), String> {
        tmux::copy(text)
    }

    #[cfg(target_os = "linux")]
    fn primary(
        &mut self,
        text: &str,
        begin_delivery: &dyn Fn() -> Result<(), String>,
    ) -> Result<Outcome, String> {
        use arboard::SetExtLinux;
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        begin_delivery()?;
        clipboard
            .set()
            .clipboard(arboard::LinuxClipboardKind::Primary)
            .text(text)
            .map_err(|error| format!("X11 primary selection: {error}"))?;
        Ok(Outcome::Copied(Some(Lease::new(clipboard))))
    }

    #[cfg(not(target_os = "linux"))]
    fn primary(
        &mut self,
        _text: &str,
        _begin_delivery: &dyn Fn() -> Result<(), String>,
    ) -> Result<Outcome, String> {
        Err("X11 primary selection unavailable".into())
    }
}

#[cfg(target_os = "android")]
impl Routes for Machine {
    fn native(&mut self, _text: &str, _html: Option<&str>) -> Result<Option<Lease>, String> {
        Err("native clipboard unavailable on Android".to_string())
    }

    fn windows(&mut self, text: &str, deadline: Instant) -> Result<(), String> {
        wsl::copy(text, deadline)
    }

    fn tmux(&mut self, text: &str) -> Result<(), String> {
        tmux::copy(text)
    }

    fn primary(
        &mut self,
        _text: &str,
        _begin_delivery: &dyn Fn() -> Result<(), String>,
    ) -> Result<Outcome, String> {
        Err("X11 primary selection unavailable".into())
    }
}

/// Write to the native clipboard, the HTML flavour first where there is one.
#[cfg(not(target_os = "android"))]
fn arboard_copy(
    mut clipboard: arboard::Clipboard,
    text: &str,
    html: Option<&str>,
) -> Result<Option<Lease>, String> {
    match html {
        Some(html) => clipboard
            .set_html(html, Some(text))
            .or_else(|_| clipboard.set_text(text)),
        None => clipboard.set_text(text),
    }
    .map_err(|error| format!("failed to set clipboard text: {error}"))?;
    // Linux clipboard owners must stay alive until the user pastes.
    #[cfg(target_os = "linux")]
    {
        Ok(Some(Lease::new(clipboard)))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(None)
    }
}
