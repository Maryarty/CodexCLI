//! The backend of one run: Codex's own mock Responses server
//! (`tools/rtwk-port/mock-model`, served by `serve.py`) on a local port and
//! a fresh `CODEX_HOME` and working directory (`home.py`), both in a temp
//! root of their own. Each binary gets its own server and home made by the
//! same recipe, so the scenario is the same and neither run consumes the
//! other's queued responses.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::seed;

/// `codex-rs/tui-rtwk` → the repository root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tool_dir() -> PathBuf {
    repo_root().join("tools/rtwk-port/mock-model")
}

/// `bins.py oracle|rtwk`: where the binary is (it builds nothing); an
/// error naming what to do when there is none.
pub fn binary(which: &str) -> Result<PathBuf, String> {
    let out = Command::new("python3")
        .arg("-I")
        .arg(tool_dir().join("bins.py"))
        .arg(which)
        .output()
        .map_err(|e| format!("python3: {e}"))?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if !out.status.success() || path.is_empty() {
        return Err(format!(
            "no {which} binary: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(PathBuf::from(path))
}

pub struct Backend {
    root: PathBuf,
    server: Child,
    pub url: String,
    pub codex_home: PathBuf,
    pub workdir: PathBuf,
    /// A `HOME` for the binary, inside the root.
    pub home: PathBuf,
}

impl Backend {
    /// Serves the mock scenario `mock`, writes a fresh home for it and
    /// applies `seed` to the home.
    pub fn start(mock: &str, seed_name: Option<&str>) -> Result<Backend, String> {
        let root = std::env::temp_dir().join(format!(
            "rtwk-scn-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos())
        ));
        std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
        let url_file = root.join("url");
        let server = Command::new("python3")
            .arg("-I")
            .arg(tool_dir().join("serve.py"))
            .arg(mock)
            .arg("--loop")
            .arg("--url-file")
            .arg(&url_file)
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                std::fs::File::create(root.join("serve.err")).map_err(|e| e.to_string())?,
            ))
            .spawn()
            .map_err(|e| format!("serve.py: {e}"))?;
        let mut backend = Backend {
            root: root.clone(),
            server,
            url: String::new(),
            codex_home: PathBuf::new(),
            workdir: PathBuf::new(),
            home: root.join("home"),
        };
        let start = Instant::now();
        while !std::fs::read_to_string(&url_file).is_ok_and(|s| !s.is_empty()) {
            if start.elapsed() > Duration::from_secs(20) {
                let log = std::fs::read_to_string(root.join("serve.err")).unwrap_or_default();
                return Err(format!("the mock server did not start: {log}"));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        backend.url = std::fs::read_to_string(&url_file).map_err(|e| e.to_string())?;
        let out = Command::new("python3")
            .arg("-I")
            .arg(tool_dir().join("home.py"))
            .arg(mock)
            .arg(&backend.url)
            .arg(&root)
            .output()
            .map_err(|e| format!("home.py: {e}"))?;
        if !out.status.success() {
            return Err(format!("home.py: {}", String::from_utf8_lossy(&out.stderr)));
        }
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut lines = text.lines();
        backend.codex_home = PathBuf::from(lines.next().ok_or("home.py printed no home")?);
        backend.workdir = PathBuf::from(lines.next().ok_or("home.py printed no workdir")?);
        std::fs::create_dir_all(&backend.home).map_err(|e| e.to_string())?;
        if let Some(name) = seed_name {
            seed::apply(name, &backend.codex_home, &backend.workdir)?;
        }
        Ok(backend)
    }

    /// The model requests the server logged (`serve.err`), for a failure
    /// report.
    pub fn server_log(&self) -> String {
        std::fs::read_to_string(self.root.join("serve.err")).unwrap_or_default()
    }

    /// The temp root and its real path (macOS: `/var` is `/private/var`),
    /// for the normalizer.
    pub fn roots(&self) -> Vec<String> {
        let mut roots = vec![self.root.display().to_string()];
        if let Ok(real) = std::fs::canonicalize(&self.root) {
            roots.push(real.display().to_string());
        }
        roots.sort_by_key(|r| std::cmp::Reverse(r.len()));
        roots.dedup();
        roots
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
        if std::env::var_os("KEEP_SCENARIO_HOMES").is_some() {
            eprintln!("kept: {}", self.root.display());
        } else {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}
