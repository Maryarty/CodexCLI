//! Opening things outside the terminal (oracle `app/history_ui.rs`):
//! `Command::OpenUrl` opens a page in the user's browser; `/app` opens the
//! thread in the Desktop app through its `codex://threads/{id}` link.
//!
//! The words of the failure are the UI's string tables'; these functions
//! return the pieces (the url and the error) and the Desktop launch's own
//! errors, as the oracle formats them from.

use super::startup::Browser;

/// A browser that did not open: the page and the system's reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenFailure {
    pub url: String,
    pub error: String,
}

/// Open `url` with `browser` (`webbrowser::open`), off the async workers: the
/// open may start a process and wait.
pub async fn open_url(browser: Browser, url: String) -> Result<(), OpenFailure> {
    let opened = url.clone();
    match tokio::task::spawn_blocking(move || browser(&opened)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(OpenFailure {
            url,
            error: error.to_string(),
        }),
        Err(error) => Err(OpenFailure {
            url,
            error: format!("the browser open did not finish: {error}"),
        }),
    }
}

/// The link the Desktop app registers for a thread.
pub fn desktop_thread_url(thread: impl std::fmt::Display) -> String {
    format!("codex://threads/{thread}")
}

/// Open the thread in the Desktop app, off the async workers. The error is
/// the launch's own text (the UI adds "Failed to open this session in the
/// Desktop app: {error}. Install or launch the Desktop app and try again.").
pub async fn open_desktop_thread(thread: String) -> Result<(), String> {
    let url = desktop_thread_url(thread);
    tokio::task::spawn_blocking(move || launch_desktop(&url))
        .await
        .map_err(|error| format!("the Desktop app launch did not finish: {error}"))?
}

#[cfg(target_os = "macos")]
fn launch_desktop(url: &str) -> Result<(), String> {
    use std::process::Command;

    let status = Command::new("open")
        .arg(url)
        .status()
        .map_err(|err| format!("failed to invoke `open`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`open {url}` exited with {status}"))
    }
}

#[cfg(target_os = "windows")]
fn launch_desktop(url: &str) -> Result<(), String> {
    use std::process::Command;

    let script = windows_launch_script(url);
    let output = Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(&script)
        .output()
        .map_err(|err| format!("failed to launch the Desktop app through PowerShell: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!(
            "failed to launch the Desktop app through PowerShell with {}",
            output.status
        ))
    } else {
        Err(stderr)
    }
}

#[cfg(target_os = "windows")]
fn windows_launch_script(url: &str) -> String {
    let url = powershell_single_quoted(url);
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$url = {url}

$package = Get-AppxPackage -Name OpenAI.Codex -ErrorAction SilentlyContinue
if ($null -eq $package) {{
    Write-Error 'Desktop app package is not installed'
    exit 1
}}

$manifest = Get-AppxPackageManifest -Package $package.PackageFullName
$application = $manifest.Package.Applications.Application |
    Where-Object {{
        @($_.Extensions.Extension) | Where-Object {{
            $_.Category -eq 'windows.protocol' -and $_.Protocol.Name -eq 'codex'
        }}
    }} |
    Select-Object -First 1
if ($null -eq $application -or [string]::IsNullOrWhiteSpace($application.Executable)) {{
    Write-Error 'Desktop app package does not declare a codex protocol executable'
    exit 1
}}

# Launch the package-declared protocol executable rather than an internal Electron shim.
# Windows can deny direct starts of internal executables under WindowsApps.
$exe = Join-Path $package.InstallLocation $application.Executable
$appDir = Split-Path -Parent $exe
$app = Join-Path $appDir 'resources\app.asar'
if (-not (Test-Path $exe)) {{
    Write-Error "Desktop app executable not found at $exe"
    exit 1
}}
if (-not (Test-Path $app)) {{
    Write-Error "Desktop app bundle not found at $app"
    exit 1
}}

Start-Process -FilePath $exe -WorkingDirectory $appDir -ArgumentList @('resources\app.asar', $url)
"#
    )
}

#[cfg(target_os = "windows")]
fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_desktop(_url: &str) -> Result<(), String> {
    Err("The Desktop app is only available on macOS and Windows".to_string())
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Arc;
    use std::sync::Mutex;

    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn a_page_is_opened_with_the_browser_off_the_async_workers() {
        let opened = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&opened);
        let browser: Browser = Arc::new(move |url| {
            seen.lock().unwrap().push(url.to_owned());
            Ok(())
        });

        let result = open_url(browser, "https://chatgpt.com/settings/usage".to_owned()).await;

        assert_eq!(result, Ok(()));
        assert_eq!(
            opened.lock().unwrap().as_slice(),
            ["https://chatgpt.com/settings/usage"]
        );
    }

    #[tokio::test]
    async fn a_browser_that_fails_gives_back_the_page_and_the_reason() {
        let browser: Browser = Arc::new(|_| Err(io::Error::other("no browser")));

        let result = open_url(browser, "https://example.com".to_owned()).await;

        assert_eq!(
            result,
            Err(OpenFailure {
                url: "https://example.com".to_owned(),
                error: "no browser".to_owned()
            })
        );
    }

    #[test]
    fn the_desktop_link_is_the_codex_scheme_with_the_thread() {
        assert_eq!(
            desktop_thread_url("019c5a1e-0000-7000-8000-000000000001"),
            "codex://threads/019c5a1e-0000-7000-8000-000000000001"
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[tokio::test]
    async fn the_desktop_app_is_only_launched_on_macos_and_windows() {
        assert_eq!(
            open_desktop_thread("t".to_owned()).await,
            Err("The Desktop app is only available on macOS and Windows".to_owned())
        );
    }
}
