//! The local state database at launch, and what happens when it cannot be opened.
//!
//! Oracle: `tui/src/startup_error.rs` (`LocalStateDbStartupError`), `tui/src/lib.rs`
//! `init_state_db_for_app_server_target` (an embedded start opens the database through
//! `state_db::try_init` and a failure ends the launch, typed), and the CLI's
//! `run_tui_with_recovery` with `cli/src/state_db_recovery.rs` (a locked database is told to the
//! user, a damaged one is moved into a backup folder and the launch is tried again, once per
//! database file). The port ends the app first, so the terminal is restored when the words are
//! printed, as in the oracle.

use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use codex_core::StateDbHandle;
use codex_core::config::Config;
use codex_state::RuntimeDbBackup;

/// The typed failure `main` finds after a launch that could not open the database
/// (`StateDbStartupError` set before the event that ends the app is sent, as
/// `startup::FailureSlot`).
pub type FailureSlot = Arc<Mutex<Option<StateDbStartupError>>>;

/// The local database could not be initialized: the file that failed, and why.
#[derive(Debug)]
pub struct StateDbStartupError {
    database_path: PathBuf,
    detail: String,
    source: anyhow::Error,
}

impl std::fmt::Display for StateDbStartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to initialize sqlite local db at {}: {}",
            self.database_path.display(),
            self.detail
        )
    }
}

impl std::error::Error for StateDbStartupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

impl StateDbStartupError {
    pub fn new(database_path: PathBuf, source: anyhow::Error) -> Self {
        Self {
            database_path,
            detail: format!("{source:#}"),
            source,
        }
    }

    pub fn is_corruption(&self) -> bool {
        codex_state::is_sqlite_corruption_error(&self.source)
    }

    pub fn database_path(&self) -> &Path {
        self.database_path.as_path()
    }

    pub fn detail(&self) -> &str {
        self.detail.as_str()
    }
}

/// Opens the local database for an embedded start (`state_db::try_init`): a failure is typed,
/// with the file of a corruption (`runtime_db_path_for_corruption_error`) or else the state
/// database's own path.
pub(crate) async fn init(config: &Config) -> Result<StateDbHandle, StateDbStartupError> {
    codex_rollout::state_db::try_init(config)
        .await
        .map_err(|err| {
            let database_path = codex_state::runtime_db_path_for_corruption_error(&err)
                .unwrap_or_else(|| config.sqlite_config().state_db_path());
            StateDbStartupError::new(database_path, err)
        })
}

/// What the launch does after a database failure was reported.
pub(crate) enum Recovery {
    /// The damaged files are moved aside and the user has read it: start again.
    Retry,
    /// The launch ends with this error, after the guidance was printed.
    Fatal(String),
}

/// The CLI's decision for `startup_error` (`run_tui_with_recovery`): `attempted` holds the
/// database files already moved aside in this process, a second failure of one is final.
pub(crate) async fn recover(
    startup_error: &StateDbStartupError,
    attempted: &mut HashSet<PathBuf>,
) -> std::io::Result<Recovery> {
    if is_locked(startup_error.detail()) {
        print_locked_guidance(startup_error);
        return Ok(Recovery::Fatal(startup_error.to_string()));
    }
    if !is_auto_backup_recoverable(startup_error) {
        print_diagnostic_guidance(startup_error);
        return Ok(Recovery::Fatal(startup_error.to_string()));
    }
    if !attempted.insert(startup_error.database_path().to_path_buf()) {
        print_diagnostic_guidance(startup_error);
        return Ok(Recovery::Fatal(startup_error.to_string()));
    }

    print_auto_backup_start(startup_error);
    match backup_files_for_fresh_start(startup_error).await {
        Ok(backups) => confirm_fresh_start_rebuild(startup_error, &backups)?,
        Err(backup_err) => {
            print_diagnostic_guidance(startup_error);
            return Ok(Recovery::Fatal(format!(
                "failed to move damaged Codex local database files into a backup folder automatically: {backup_err}"
            )));
        }
    }
    Ok(Recovery::Retry)
}

fn is_locked(detail: &str) -> bool {
    codex_state::sqlite_error_detail_is_lock(detail)
}

fn is_auto_backup_recoverable(startup_error: &StateDbStartupError) -> bool {
    startup_error.is_corruption() || sqlite_home_is_blocking_file(startup_error)
}

fn sqlite_home_is_blocking_file(startup_error: &StateDbStartupError) -> bool {
    startup_error
        .database_path()
        .parent()
        .and_then(|path| std::fs::metadata(path).ok())
        .is_some_and(|metadata| metadata.is_file())
}

fn print_auto_backup_start(startup_error: &StateDbStartupError) {
    eprintln!("Codex couldn't start because its local database appears to be damaged.");
    eprintln!("Moving the damaged local database aside so Codex can rebuild it from saved data.");
    print_technical_details(startup_error);
}

async fn backup_files_for_fresh_start(
    startup_error: &StateDbStartupError,
) -> std::io::Result<Vec<RuntimeDbBackup>> {
    codex_state::backup_runtime_db_for_fresh_start(startup_error.database_path()).await
}

fn confirm_fresh_start_rebuild(
    startup_error: &StateDbStartupError,
    backups: &[RuntimeDbBackup],
) -> std::io::Result<()> {
    eprintln!("Codex rebuilt its local database.");
    eprintln!(
        "Codex detected a damaged local database, moved it into a backup folder, and will continue startup with a fresh database."
    );
    eprintln!("Database path: {}", startup_error.database_path().display());
    if let Some(backup_folder) = backup_folder(backups) {
        eprintln!("Backup folder: {}", backup_folder.display());
    } else {
        eprintln!("Backup folder: unavailable");
    }

    if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        eprintln!("Press Enter to continue.");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    } else {
        eprintln!("Continuing startup with a fresh local database...");
    }
    Ok(())
}

fn print_diagnostic_guidance(startup_error: &StateDbStartupError) {
    eprintln!("Codex couldn't start because its local database appears to be damaged.");
    eprintln!("Run `codex doctor` to check your setup and get next-step guidance.");
    eprintln!("If this keeps happening, share the technical details below when asking for help.");
    print_technical_details(startup_error);
}

fn print_locked_guidance(startup_error: &StateDbStartupError) {
    eprintln!("Codex couldn't start because another Codex process is using its local data.");
    eprintln!("Quit any other copies of Codex that may still be running, then try again.");
    print_technical_details(startup_error);
}

fn print_technical_details(startup_error: &StateDbStartupError) {
    eprintln!("Technical details:");
    eprintln!("  Location: {}", startup_error.database_path().display());
    eprintln!("  Cause: {}", startup_error.detail());
}

fn backup_folder(backups: &[RuntimeDbBackup]) -> Option<&Path> {
    backups.first()?.backup_path.parent()
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::config::ConfigBuilder;
    use codex_utils_absolute_path::test_support::PathExt;
    use tempfile::TempDir;

    /// A file with `contents` (`std::fs::write` is the shell's: R12).
    fn write_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        use std::io::Write as _;
        std::fs::File::create(path)?.write_all(contents.as_ref())
    }

    async fn build_config(temp_dir: &TempDir) -> std::io::Result<Config> {
        ConfigBuilder::default()
            .codex_home(temp_dir.path().to_path_buf())
            .fallback_cwd(Some(temp_dir.path().to_path_buf()))
            .build()
            .await
    }

    // oracle `embedded_state_db_failure_is_typed_for_cli_recovery` (tui/src/lib_tests.rs): an
    // embedded start whose sqlite home is a file fails with the typed error, at the state
    // database's path.
    #[tokio::test]
    async fn embedded_state_db_failure_is_typed_for_cli_recovery() -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = build_config(&temp_dir).await?;
        let occupied_sqlite_home = temp_dir.path().join("sqlite-home");
        write_file(&occupied_sqlite_home, "occupied")?;
        let sqlite =
            codex_state::SqliteConfig::new_for_testing(occupied_sqlite_home.as_path().abs());
        config.sqlite = sqlite.clone();

        let Err(startup_error) = init(&config).await else {
            panic!("embedded startup should surface state db init failures");
        };

        assert_eq!(startup_error.database_path(), sqlite.state_db_path());
        assert!(
            startup_error
                .detail()
                .contains("failed to initialize state runtime"),
            "startup error should preserve the underlying state db failure"
        );
        // the typed error survives the context `start_args` puts around it
        let wrapped = anyhow::Error::new(startup_error).context("starting");
        assert!(wrapped.downcast_ref::<StateDbStartupError>().is_some());
        Ok(())
    }

    // oracle `embedded_state_db_corruption_preserves_failed_database_for_cli_recovery`: the
    // file that is damaged is the one the error names, and it is a corruption.
    #[tokio::test]
    async fn embedded_state_db_corruption_preserves_failed_database_for_cli_recovery()
    -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = build_config(&temp_dir).await?;
        let sqlite_home = temp_dir.path().join("sqlite-home");
        std::fs::create_dir_all(&sqlite_home)?;
        let sqlite = codex_state::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
        let logs_db_path = sqlite.logs_db_path();
        write_file(&logs_db_path, "not a sqlite database")?;
        config.sqlite = sqlite;

        let Err(startup_error) = init(&config).await else {
            panic!("embedded startup should surface state db init failures");
        };

        assert_eq!(startup_error.database_path(), logs_db_path.as_path());
        assert!(
            startup_error.is_corruption(),
            "startup error should preserve the SQLite corruption cause, got: {}",
            startup_error.detail()
        );
        Ok(())
    }

    // oracle `cli/src/state_db_recovery.rs` tests, as they are.
    #[tokio::test]
    async fn backup_backs_up_only_failed_database_file() -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let sqlite = codex_state::SqliteConfig::new_for_testing(temp_dir.path().abs());
        let state_path = sqlite.state_db_path();
        let failed_db_path = sqlite.logs_db_path();
        write_file(state_path.as_path(), b"state")?;
        write_file(failed_db_path.as_path(), b"logs")?;

        let startup_error =
            StateDbStartupError::new(failed_db_path.clone(), anyhow::anyhow!("corrupt"));
        let backups = backup_files_for_fresh_start(&startup_error).await?;

        assert_eq!(
            backups
                .iter()
                .map(|backup| &backup.original_path)
                .collect::<Vec<_>>(),
            vec![&failed_db_path]
        );
        assert!(!failed_db_path.try_exists()?);
        assert!(state_path.try_exists()?);
        assert!(backups[0].backup_path.try_exists()?);
        Ok(())
    }

    #[tokio::test]
    async fn backup_replaces_blocking_sqlite_home_file() -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let sqlite_home = temp_dir.path().join("sqlite-home");
        write_file(sqlite_home.as_path(), b"not-a-directory")?;
        let sqlite = codex_state::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
        let startup_error =
            StateDbStartupError::new(sqlite.state_db_path(), anyhow::anyhow!("File exists"));

        assert!(is_auto_backup_recoverable(&startup_error));
        let backups = backup_files_for_fresh_start(&startup_error).await?;

        assert_eq!(backups.len(), 1);
        assert!(std::fs::metadata(sqlite_home.as_path())?.is_dir());
        assert!(backups[0].backup_path.try_exists()?);
        Ok(())
    }

    // oracle `run_tui_with_recovery`: a damaged file is moved into a backup folder and the launch
    // tried again; the same file failing again, or a locked database, ends it.
    #[tokio::test]
    async fn recovery_moves_a_damaged_file_aside_once_and_a_lock_ends_the_launch()
    -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = build_config(&temp_dir).await?;
        let sqlite_home = temp_dir.path().join("sqlite-home");
        std::fs::create_dir_all(&sqlite_home)?;
        let sqlite = codex_state::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
        let logs_db_path = sqlite.logs_db_path();
        write_file(&logs_db_path, "not a sqlite database")?;
        config.sqlite = sqlite;
        let Err(startup_error) = init(&config).await else {
            panic!("embedded startup should surface state db init failures");
        };

        let mut attempted = HashSet::new();
        assert!(matches!(
            recover(&startup_error, &mut attempted).await?,
            Recovery::Retry
        ));
        assert!(!logs_db_path.exists(), "the damaged file was moved aside");
        assert_eq!(attempted, HashSet::from([logs_db_path.clone()]));
        // the same file again: no second backup, the launch ends with the error's text
        let Recovery::Fatal(message) = recover(&startup_error, &mut attempted).await? else {
            panic!("a second failure of the same file ends the launch");
        };
        assert_eq!(message, startup_error.to_string());

        let locked =
            StateDbStartupError::new(logs_db_path.clone(), anyhow::anyhow!("database is locked"));
        let Recovery::Fatal(message) = recover(&locked, &mut HashSet::new()).await? else {
            panic!("a locked database ends the launch");
        };
        assert!(message.ends_with("database is locked"), "{message}");
        Ok(())
    }

    #[test]
    fn backup_folder_uses_parent_of_first_backup_path() {
        let backups = vec![RuntimeDbBackup {
            original_path: PathBuf::from("/tmp/state_5.sqlite"),
            backup_path: PathBuf::from("/tmp/db-backups/sqlite-1-0/state_5.sqlite"),
        }];

        assert_eq!(
            backup_folder(&backups),
            Some(Path::new("/tmp/db-backups/sqlite-1-0"))
        );
    }
}
