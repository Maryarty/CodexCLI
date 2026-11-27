use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::thread;

use super::*;

fn fixture() -> io::Result<(tempfile::TempDir, PathBuf, UnixListener)> {
    // Keep the socket below the smallest supported sockaddr_un limit.
    let home = tempfile::tempdir_in("/tmp")?;
    let ipc = home.path().join("ipc");
    fs::create_dir(&ipc)?;
    fs::set_permissions(ipc, fs::Permissions::from_mode(0o700))?;
    let socket = primary_ipc_socket_path(home.path());
    let listener = UnixListener::bind(&socket)?;
    Ok((home, socket, listener))
}

fn read_request(listener: UnixListener) -> (UnixStream, Value) {
    let (mut stream, _) = listener
        .accept()
        .unwrap_or_else(|error| panic!("IDE fixture accept: {error}"));
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap_or_else(|error| panic!("IDE fixture timeout: {error}"));
    let request = read_frame(&mut stream, Instant::now() + Duration::from_secs(1))
        .unwrap_or_else(|error| panic!("read IDE IPC request: {error}"));
    (stream, request)
}

fn respond(stream: &mut UnixStream, id: &Value, selection: &str) -> io::Result<()> {
    write_frame(
        stream,
        &json!({
            "type": "response",
            "requestId": id,
            "resultType": "success",
            "result": { "ideContext": {
                "activeFile": {
                    "label": "main.rs",
                    "path": "src/main.rs",
                    "selection": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 7 }
                    },
                    "activeSelectionContent": selection,
                },
                "openTabs": [{ "label": "Cargo.toml", "path": "Cargo.toml" }],
            } },
        }),
    )
}

#[tokio::test]
async fn ide_context_reads_the_actual_authenticated_socket_and_correlates_replies()
-> anyhow::Result<()> {
    let (home, _, listener) = fixture()?;
    let workspace = home.path().join("workspace");
    fs::create_dir(&workspace)?;
    let expected_workspace = workspace.clone();
    let provider = thread::spawn(move || -> anyhow::Result<()> {
        let (mut stream, request) = read_request(listener);
        assert_eq!(request["method"], "ide-context");
        assert_eq!(request["version"], 0);
        assert_eq!(
            request["params"]["workspaceRoot"],
            expected_workspace.to_string_lossy().as_ref()
        );
        assert!(request["requestId"].is_string());
        respond(&mut stream, &json!("other-request"), "foreign answer")?;
        respond(&mut stream, &request["requestId"], "fn main")?;
        Ok(())
    });
    let prompt = super::super::read_ide_context(workspace, home.path().to_owned())
        .await
        .unwrap_or_else(|failure| panic!("{}", failure.hint));
    let prompt = prompt.unwrap_or_else(|| panic!("no prompt from the IDE fixture"));
    assert!(prompt.contains("## Active file: src/main.rs"));
    assert!(prompt.contains("## Active selection of the file:\nfn main"));
    assert!(prompt.contains("- Cargo.toml: Cargo.toml"));
    assert!(!prompt.contains("foreign answer"));
    provider
        .join()
        .unwrap_or_else(|_| panic!("IDE fixture panicked"))?;
    Ok(())
}

#[test]
fn ide_context_rejects_an_unowned_peer_and_untrusted_socket_directory() -> anyhow::Result<()> {
    let uid = unsafe { libc::getuid() };
    assert_eq!(
        ensure_peer_uid_matches_current_user(uid.wrapping_add(1))
            .err()
            .map(|error| error.kind()),
        Some(io::ErrorKind::PermissionDenied)
    );
    let (home, socket, listener) = fixture()?;
    fs::set_permissions(home.path().join("ipc"), fs::Permissions::from_mode(0o777))?;
    let error = fetch_ide_context_from_unix_socket_paths(
        socket,
        Vec::new(),
        home.path(),
        Instant::now() + Duration::from_secs(1),
    );
    assert!(
        matches!(error, Err(IdeContextError::Connect(error)) if error.kind() == io::ErrorKind::PermissionDenied)
    );
    listener.set_nonblocking(true)?;
    assert!(matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    Ok(())
}

#[test]
fn ide_context_rejects_an_oversized_frame_before_reading_its_body() -> anyhow::Result<()> {
    let (home, socket, listener) = fixture()?;
    let provider = thread::spawn(move || -> io::Result<()> {
        let (mut stream, _) = read_request(listener);
        let oversized = u32::try_from(MAX_IPC_FRAME_BYTES + 1)
            .unwrap_or_else(|error| panic!("frame size: {error}"));
        stream.write_all(&oversized.to_le_bytes())
    });
    let result = fetch_ide_context_from_unix_socket_paths(
        socket,
        Vec::new(),
        home.path(),
        Instant::now() + Duration::from_secs(1),
    );
    assert!(matches!(result, Err(IdeContextError::ResponseTooLarge)));
    provider
        .join()
        .unwrap_or_else(|_| panic!("IDE fixture panicked"))?;
    Ok(())
}

#[test]
fn ide_context_deadline_includes_a_partial_frame() -> anyhow::Result<()> {
    let (home, socket, listener) = fixture()?;
    let (release, wait) = std::sync::mpsc::channel();
    let provider = thread::spawn(move || -> io::Result<()> {
        let (mut stream, _) = read_request(listener);
        stream.write_all(&[4, 0])?;
        let _ = wait.recv_timeout(Duration::from_secs(2));
        Ok(())
    });
    let result = fetch_ide_context_from_unix_socket_paths(
        socket,
        Vec::new(),
        home.path(),
        Instant::now() + Duration::from_millis(250),
    );
    assert!(
        matches!(result, Err(IdeContextError::Read(error)) if error.kind() == io::ErrorKind::TimedOut)
    );
    release.send(())?;
    provider
        .join()
        .unwrap_or_else(|_| panic!("IDE fixture panicked"))?;
    Ok(())
}

#[tokio::test]
async fn ide_context_never_contacts_the_extension_without_a_local_workspace() -> anyhow::Result<()>
{
    let (home, _, listener) = fixture()?;
    let result =
        super::super::read_ide_context(PathBuf::from("relative"), home.path().to_owned()).await;
    assert!(matches!(result, Err(error) if error.skip_hint == OPEN_IDE_HINT));
    listener.set_nonblocking(true)?;
    assert!(matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    Ok(())
}
