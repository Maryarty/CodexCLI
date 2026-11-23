//! The `$` picks file (oracle `bottom_pane/mention_usage.rs`): a bounded
//! window of the confirmed picker selections, kept under the Codex home as
//! `mention_usage.json`. The window itself is the core's
//! (`data::usage::MentionUsage`, updated at once); this is its
//! file: loaded when the app starts, one write per pick.
//!
//! Writes are serialized, and a bounded file-lock wait merges selections made
//! by separate processes without indefinitely delaying a pick. Malformed
//! history is treated as empty and replaced by the next persisted pick.

use std::collections::VecDeque;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use codex_tui_core::screens::popovers::Ev;
use codex_utils_path::write_atomically;

/// How many picks the window keeps.
pub const HISTORY_LIMIT: usize = 50;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const FILE_NAME: &str = "mention_usage.json";
const LOCK_TIMEOUT: Duration = Duration::from_millis(100);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

/// The file of one Codex home. `persist` is `history.persistence = save-all`:
/// without it nothing is read or written.
#[derive(Clone)]
pub struct MentionFile {
    path: PathBuf,
    persist: bool,
    /// One write at a time, in the order the picks were made.
    writes: Arc<tokio::sync::Mutex<()>>,
}

impl MentionFile {
    pub fn new(codex_home: &Path, persist: bool) -> Self {
        Self {
            path: codex_home.join(FILE_NAME),
            persist,
            writes: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// The window the file holds, oldest first: `Ev::MentionUsage`. A missing,
    /// oversized or unreadable file is an empty window.
    pub async fn load(&self) -> Ev {
        let path = self.path.clone();
        let recent = if self.persist {
            tokio::task::spawn_blocking(move || read_recent(&path))
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default()
        } else {
            VecDeque::new()
        };
        Ev::MentionUsage(recent.into())
    }

    /// Persist one confirmed pick. A failure is the caller's to log: the pick
    /// itself is already in the window.
    pub async fn record(&self, identity: String) -> Result<(), String> {
        if !self.persist {
            return Ok(());
        }
        let _turn = self.writes.lock().await;
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || persist_selection(&path, identity))
            .await
            .map_err(|error| format!("mention preference writer stopped: {error}"))?
            .map_err(|error| format!("could not save mention preferences: {error}"))
    }
}

fn append_recent(recent: &mut VecDeque<String>, identity: String) {
    if recent.len() == HISTORY_LIMIT {
        recent.pop_front();
    }
    recent.push_back(identity);
}

fn read_recent(path: &Path) -> io::Result<VecDeque<String>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(VecDeque::new()),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() > MAX_FILE_BYTES {
        return Err(io::Error::other("mention preferences exceed size limit"));
    }
    let mut recent: VecDeque<String> = match serde_json::from_reader(file) {
        Ok(recent) => recent,
        Err(error) if error.is_io() => return Err(error.into()),
        Err(_) => VecDeque::new(),
    };
    recent.drain(..recent.len().saturating_sub(HISTORY_LIMIT));
    Ok(recent)
}

fn persist_selection(path: &Path, identity: String) -> io::Result<()> {
    let lock_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path.with_extension("lock"))?;
    let deadline = Instant::now() + LOCK_TIMEOUT;
    loop {
        match lock_file.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(LOCK_RETRY_INTERVAL);
            }
            Err(error) => return Err(error.into()),
        }
    }
    let mut recent = read_recent(path)?;
    append_recent(&mut recent, identity);
    let contents = serde_json::to_string(&recent)?;
    if contents.len() as u64 > MAX_FILE_BYTES {
        return Err(io::Error::other("mention preferences exceed size limit"));
    }
    write_atomically(path, &contents)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn recent(file: &MentionFile) -> Vec<String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        match runtime.block_on(file.load()) {
            Ev::MentionUsage(recent) => recent,
            other => panic!("not the usage: {other:?}"),
        }
    }

    #[tokio::test]
    async fn picks_persist_in_order_across_loads() {
        let home = tempfile::tempdir().unwrap();
        let file = MentionFile::new(home.path(), true);

        file.record("skill://status".into()).await.unwrap();
        file.record("skill://status".into()).await.unwrap();
        file.record("app://other".into()).await.unwrap();

        // Another process (a new handle) reads the same window, oldest first.
        let Ev::MentionUsage(window) = MentionFile::new(home.path(), true).load().await else {
            panic!("not the usage");
        };
        assert_eq!(window, ["skill://status", "skill://status", "app://other"]);
    }

    #[tokio::test]
    async fn concurrent_picks_are_written_one_at_a_time_and_none_is_lost() {
        let home = tempfile::tempdir().unwrap();
        let file = MentionFile::new(home.path(), true);

        let (a, b, c) = tokio::join!(
            file.record("a".into()),
            file.record("b".into()),
            file.record("c".into())
        );
        a.unwrap();
        b.unwrap();
        c.unwrap();

        let Ev::MentionUsage(mut window) = file.load().await else {
            panic!("not the usage");
        };
        window.sort();
        assert_eq!(window, ["a", "b", "c"]);
    }

    #[tokio::test]
    async fn old_picks_expire_from_the_file() {
        let home = tempfile::tempdir().unwrap();
        let file = MentionFile::new(home.path(), true);
        file.record("skill://status".into()).await.unwrap();

        for _ in 0..HISTORY_LIMIT {
            file.record("skill://other".into()).await.unwrap();
        }

        let Ev::MentionUsage(window) = file.load().await else {
            panic!("not the usage");
        };
        assert_eq!(window, vec!["skill://other"; HISTORY_LIMIT]);
    }

    #[test]
    fn a_pick_replaces_malformed_history() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(FILE_NAME);
        std::fs::write(&path, "[\"truncated").unwrap();

        persist_selection(&path, "skill://status".to_owned()).unwrap();

        assert_eq!(
            recent(&MentionFile::new(home.path(), true)),
            ["skill://status"]
        );
    }

    #[test]
    fn a_contended_lock_does_not_prevent_later_picks() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(FILE_NAME);
        let lock = File::create(path.with_extension("lock")).unwrap();
        lock.lock().unwrap();

        let blocked = persist_selection(&path, "skill://blocked".to_owned());
        assert_eq!(
            blocked.expect_err("lock contention").kind(),
            io::ErrorKind::WouldBlock
        );
        drop(lock);
        persist_selection(&path, "skill://status".to_owned()).unwrap();

        assert_eq!(
            recent(&MentionFile::new(home.path(), true)),
            ["skill://status"]
        );
    }

    #[test]
    fn an_oversized_or_unreadable_file_is_an_empty_window() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(FILE_NAME);
        std::fs::write(&path, vec![b' '; MAX_FILE_BYTES as usize + 1]).unwrap();

        assert_eq!(
            recent(&MentionFile::new(home.path(), true)),
            Vec::<String>::new()
        );
    }

    #[tokio::test]
    async fn without_persistence_nothing_is_read_or_written() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join(FILE_NAME), "[\"skill://kept\"]").unwrap();
        let file = MentionFile::new(home.path(), false);

        file.record("skill://status".into()).await.unwrap();

        let Ev::MentionUsage(window) = file.load().await else {
            panic!("not the usage");
        };
        assert_eq!(window, Vec::<String>::new());
        assert_eq!(
            std::fs::read_to_string(home.path().join(FILE_NAME)).unwrap(),
            "[\"skill://kept\"]"
        );
    }
}
