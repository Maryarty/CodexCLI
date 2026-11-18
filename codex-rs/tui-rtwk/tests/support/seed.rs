//! Seeds for a fresh `CODEX_HOME`: files the oracle's own tests make with
//! `create_fake_rollout` (codex-rs/app-server/tests/common/rollout.rs), here
//! written as JSON so the harness has no dependency on the backend crates.

use std::fs;
use std::path::Path;

/// One saved session: (filename timestamp, thread id, preview).
const AGENTS: [(&str, &str, &str); 3] = [
    (
        "2026-01-05T09-00-00",
        "5e4c0001-0000-4000-8000-000000000001",
        "Refactor the parser module",
    ),
    (
        "2026-01-06T10-30-00",
        "5e4c0002-0000-4000-8000-000000000002",
        "Write release notes for the next version",
    ),
    (
        "2026-01-07T14-15-00",
        "5e4c0003-0000-4000-8000-000000000003",
        "Investigate the flaky upload test",
    ),
];

/// Writes the seed called `name` into `codex_home` for sessions run in `workdir`.
pub fn apply(name: &str, codex_home: &Path, workdir: &Path) -> Result<(), String> {
    match name {
        "agents-3" => {
            for (ts, id, preview) in AGENTS {
                rollout(codex_home, workdir, ts, id, preview).map_err(|e| format!("seed {name}: {e}"))?;
            }
            Ok(())
        }
        other => Err(format!("unknown seed `{other}`")),
    }
}

/// `sessions/YYYY/MM/DD/rollout-<ts>-<id>.jsonl`: the session meta, the user's
/// message as a response item and as an event; the file's mtime is the
/// session's time.
fn rollout(codex_home: &Path, workdir: &Path, ts: &str, id: &str, preview: &str) -> std::io::Result<()> {
    let dir = codex_home
        .join("sessions")
        .join(&ts[0..4])
        .join(&ts[5..7])
        .join(&ts[8..10]);
    fs::create_dir_all(&dir)?;
    let rfc = format!("{}:{}:{}.000Z", &ts[0..13], &ts[14..16], &ts[17..19]);
    let line = |kind: &str, payload: serde_json::Value| {
        serde_json::json!({"timestamp": rfc, "type": kind, "payload": payload}).to_string()
    };
    let lines = [
        line(
            "session_meta",
            serde_json::json!({
                "session_id": id, "id": id, "timestamp": rfc,
                "cwd": workdir, "originator": "codex", "cli_version": "0.0.0",
                "source": "cli", "model_provider": "mock_provider",
            }),
        ),
        line(
            "response_item",
            serde_json::json!({"type": "message", "role": "user",
                "content": [{"type": "input_text", "text": preview}]}),
        ),
        line(
            "event_msg",
            serde_json::json!({"type": "user_message", "message": preview, "kind": "plain"}),
        ),
    ];
    let path = dir.join(format!("rollout-{ts}-{id}.jsonl"));
    fs::write(&path, lines.join("\n") + "\n")?;
    let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(epoch_seconds(&rfc));
    fs::OpenOptions::new()
        .append(true)
        .open(&path)?
        .set_times(fs::FileTimes::new().set_modified(when))
}

/// Seconds since the epoch of `YYYY-MM-DDThh:mm:ss.000Z` (days from civil).
fn epoch_seconds(rfc: &str) -> u64 {
    let n = |a: usize, b: usize| rfc[a..b].parse::<i64>().unwrap_or(0);
    let (y, m, d) = (n(0, 4), n(5, 7), n(8, 10));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + n(11, 13) * 3600 + n(14, 16) * 60 + n(17, 19)).unwrap_or(0)
}
