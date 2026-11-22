//! Local clipboard text reads (oracle `clipboard_paste/text.rs`). WSL reads
//! the Windows clipboard through one long-lived PowerShell reader; it never
//! falls back to a separate Linux clipboard. Over SSH the clipboard is not
//! this machine's, so there is nothing to read.
//!
//! In Flutter this is `Clipboard.getData(Clipboard.kTextPlain)`: a future the
//! caller awaits ([`TextReader::read_within`]); the blocking read runs off the
//! async workers and gives up at its deadline.

use std::time::Duration;
use std::time::Instant;

use codex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;

use super::super::Environment;

#[cfg(target_os = "linux")]
type Request = (Instant, std::sync::mpsc::Sender<Result<String, String>>);

/// Reads the text of the system clipboard. Cheap to clone: the WSL reader
/// thread, when there is one, is shared.
#[derive(Clone)]
pub struct TextReader {
    ssh: bool,
    #[cfg(target_os = "linux")]
    windows: Option<Result<std::sync::mpsc::SyncSender<Request>, String>>,
}

impl TextReader {
    pub fn new(environment: &Environment) -> Self {
        #[cfg(target_os = "linux")]
        let windows = (environment.wsl && !environment.ssh).then(spawn_windows_reader);
        Self {
            ssh: environment.ssh,
            #[cfg(target_os = "linux")]
            windows,
        }
    }

    /// Read the text, off the async workers, within `timeout`.
    pub async fn read_within(&self, timeout: Duration) -> Result<String, String> {
        let reader = self.clone();
        let deadline = Instant::now() + timeout;
        tokio::task::spawn_blocking(move || reader.read(deadline))
            .await
            .map_err(|error| format!("the clipboard read did not finish: {error}"))?
    }

    /// The blocking read: an empty clipboard is the empty text.
    pub fn read(&self, deadline: Instant) -> Result<String, String> {
        if self.ssh {
            return Err("clipboard text is unavailable over SSH".into());
        }
        #[cfg(target_os = "linux")]
        if let Some(windows) = &self.windows {
            let (response, result) = std::sync::mpsc::channel();
            windows
                .as_ref()
                .map_err(Clone::clone)?
                .try_send((deadline, response))
                .map_err(|_| "Windows clipboard reader is busy or unavailable")?;
            return result
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "clipboard read timed out")?;
        }
        #[cfg(not(target_os = "android"))]
        {
            native_result(
                arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()),
                deadline,
            )
        }
        #[cfg(target_os = "android")]
        {
            validate(String::new(), deadline)
        }
    }
}

/// WSL process creation itself can hang. Never do it on the caller's thread,
/// including on retries; at most one Windows reader can be starting or
/// running.
#[cfg(target_os = "linux")]
fn spawn_windows_reader() -> Result<std::sync::mpsc::SyncSender<Request>, String> {
    let (requests, reads) = std::sync::mpsc::sync_channel::<Request>(/*bound*/ 1);
    std::thread::Builder::new()
        .name("wsl-clipboard-reader".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let mut child = runtime
                .as_ref()
                .ok()
                .and_then(|runtime| start_windows_reader(runtime).ok());
            while let Ok((deadline, response)) = reads.recv() {
                if Instant::now() >= deadline {
                    continue;
                }
                let result = match &runtime {
                    Ok(runtime) => {
                        if child.is_none() {
                            child = start_windows_reader(runtime).ok();
                        }
                        match child.as_mut() {
                            Some(child) => runtime.block_on(read_command(child, deadline)),
                            None => Err("Windows clipboard reader is unavailable".into()),
                        }
                    }
                    Err(_) => Err("could not start clipboard reader".into()),
                };
                // Never mistake a partial response for the next paste.
                if result.is_err() {
                    child.take();
                }
                let _ = response.send(result);
            }
        })
        .map(|_| requests)
        .map_err(|_| "could not start clipboard reader".to_string())
}

#[cfg(not(target_os = "android"))]
fn native_result(
    result: Result<String, arboard::Error>,
    deadline: Instant,
) -> Result<String, String> {
    let text = match result {
        Ok(text) => text,
        Err(arboard::Error::ContentNotAvailable) => String::new(),
        Err(_) => return Err("clipboard text is unavailable".into()),
    };
    validate(text, deadline)
}

fn validate(text: String, deadline: Instant) -> Result<String, String> {
    if Instant::now() >= deadline {
        Err("clipboard read timed out".into())
    } else if text.chars().count() > MAX_USER_INPUT_TEXT_CHARS {
        Err("clipboard text exceeds the message size limit".into())
    } else {
        Ok(text)
    }
}

#[cfg(target_os = "linux")]
fn start_windows_reader(
    runtime: &tokio::runtime::Runtime,
) -> Result<tokio::process::Child, String> {
    use std::process::Stdio;

    let _guard = runtime.enter();
    let executable = codex_utils_path::system_executable("powershell.exe")
        .ok_or("Windows clipboard reader is unavailable")?;
    let mut command = tokio::process::Command::new(executable);
    // Length-framed UTF-8 preserves every newline and whitespace, including empty text.
    // Neither the prompt nor clipboard contents are ever evaluated as PowerShell.
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        r#"
$ErrorActionPreference = 'Stop'
$out = [Console]::OpenStandardOutput()
$utf8 = [System.Text.UTF8Encoding]::new($false)
while ($null -ne [Console]::ReadLine()) {
    $bytes = $utf8.GetBytes([string](Get-Clipboard -Raw))
    $header = [BitConverter]::GetBytes([uint32]$bytes.Length)
    $out.Write($header, 0, $header.Length)
    $out.Write($bytes, 0, $bytes.Length)
    $out.Flush()
}
"#,
    ]);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(/*kill_on_drop*/ true)
        .spawn()
        .map_err(|_| "could not start Windows clipboard reader".into())
}

#[cfg(any(target_os = "linux", all(test, unix)))]
async fn read_command(
    child: &mut tokio::process::Child,
    deadline: Instant,
) -> Result<String, String> {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    let result = tokio::time::timeout_at(deadline.into(), async {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("clipboard reader has no input")?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|_| "could not start clipboard read")?;
        stdin
            .flush()
            .await
            .map_err(|_| "could not start clipboard read")?;
        let stdout = child
            .stdout
            .as_mut()
            .ok_or("clipboard reader has no output")?;
        let size = stdout
            .read_u32_le()
            .await
            .map_err(|_| "could not read clipboard text")? as usize;
        if size > MAX_USER_INPUT_TEXT_CHARS * 4 {
            return Err("clipboard text exceeds the message size limit");
        }
        let mut bytes = vec![0; size];
        stdout
            .read_exact(&mut bytes)
            .await
            .map_err(|_| "could not read clipboard text")?;
        String::from_utf8(bytes).map_err(|_| "clipboard text is not UTF-8")
    })
    .await
    .unwrap_or(Err("clipboard read timed out"));
    validate(result.map_err(str::to_owned)?, deadline)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn text_preserves_whitespace_and_rejects_late_or_oversized_reads() {
        let deadline = Instant::now() + Duration::from_secs(/*secs*/ 5);
        assert_eq!(
            validate(" café\r\n\t".into(), deadline),
            Ok(" café\r\n\t".into())
        );
        assert!(validate("late".into(), Instant::now()).is_err());
        assert!(validate("x".repeat(MAX_USER_INPUT_TEXT_CHARS + 1), deadline).is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn command_drains_large_output_and_bounds_waiting() {
        use std::process::Stdio;

        let mut child = tokio::process::Command::new("sh")
            .args([
                "-c",
                // Two distinct framed clipboard results, only in response to separate requests.
                r#"read -r request; printf '\001\000\002\000'; head -c 131072 /dev/zero | tr '\000' x; printf '\n'; read -r request; printf '\010\000\000\000 café\r\n'"#,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(/*kill_on_drop*/ true)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(/*secs*/ 5);
        assert_eq!(
            read_command(&mut child, deadline).await,
            Ok(format!("{}\n", "x".repeat(/*n*/ 131072)))
        );
        assert_eq!(
            read_command(&mut child, deadline).await,
            Ok(" café\r\n".into())
        );
        let mut child = tokio::process::Command::new("sh")
            .args(["-c", "exec sleep 10"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(/*kill_on_drop*/ true)
            .spawn()
            .unwrap();
        assert_eq!(
            read_command(
                &mut child,
                Instant::now() + Duration::from_millis(/*millis*/ 20)
            )
            .await,
            Err("clipboard read timed out".into())
        );
    }

    #[tokio::test]
    async fn over_ssh_there_is_no_clipboard_to_read() {
        let reader = TextReader::new(&Environment {
            ssh: true,
            ..Environment::default()
        });

        assert_eq!(
            reader.read_within(Duration::from_secs(1)).await,
            Err("clipboard text is unavailable over SSH".into())
        );
    }
}
