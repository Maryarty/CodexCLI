//! One binary in a pty: rtwk-term's `TerminalSession` runs it, its emulator
//! answers the startup probes (cursor position, device attributes, keyboard
//! flags, colours) and is the screen we read.

use std::path::Path;
use std::time::{Duration, Instant};

use rtwk_term::pty::PtyCommand;
use rtwk_term::{TerminalMode, TerminalSession};

/// What the terminal showed at a checkpoint.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shot {
    /// The visible rows, trailing blanks cut.
    pub rows: Vec<String>,
    /// The lines that scrolled off the top, oldest first, trailing blanks cut.
    pub scrollback: Vec<String>,
    /// (column, row) of the cursor.
    pub cursor: (usize, usize),
    pub cursor_visible: bool,
    pub alt_screen: bool,
}

impl Shot {
    /// The visible rows as one text.
    pub fn text(&self) -> String {
        self.rows.join("\n")
    }
}

/// The environment a binary runs in: nothing of the parent's but what a
/// terminal program needs, `HOME` inside the scenario's temp root, so
/// nothing reads or writes the real `~/.codex`.
pub fn environment(codex_home: &Path, home: &Path) -> Vec<(String, String)> {
    let parent = |key: &str| std::env::var(key).ok();
    let mut env = vec![
        ("CODEX_HOME".to_owned(), codex_home.display().to_string()),
        ("CODEX_SQLITE_HOME".to_owned(), codex_home.display().to_string()),
        ("HOME".to_owned(), home.display().to_string()),
        ("OPENAI_API_KEY".to_owned(), "dummy".to_owned()),
        ("TERM".to_owned(), "xterm-256color".to_owned()),
        ("LANG".to_owned(), "en_US.UTF-8".to_owned()),
        ("NO_COLOR".to_owned(), String::new()),
        ("RUST_BACKTRACE".to_owned(), "0".to_owned()),
    ];
    env.retain(|(key, _)| key != "NO_COLOR");
    for key in ["PATH", "SHELL", "TMPDIR", "USER"] {
        if let Some(value) = parent(key) {
            env.push((key.to_owned(), value));
        }
    }
    env
}

pub struct Pty {
    session: TerminalSession,
    last_output: Instant,
    pub bytes: usize,
}

impl Pty {
    /// Starts `exe` with `args` in `dir`, in a terminal of `size`.
    pub fn start(
        exe: &Path,
        args: &[String],
        dir: &Path,
        env: &[(String, String)],
        size: (u16, u16),
    ) -> Result<Pty, String> {
        let mut command = PtyCommand::new(exe)
            .args(args)
            .current_dir(dir)
            .include_parent_environment(false);
        for (key, value) in env {
            command = command.env(key, value);
        }
        let session = TerminalSession::start(command, Some(size));
        if let Some(error) = session.error() {
            return Err(format!("{} did not start: {error}", exe.display()));
        }
        Ok(Pty {
            session,
            last_output: Instant::now(),
            bytes: 0,
        })
    }

    /// Feeds what the program wrote since the last call to the emulator.
    pub fn pump(&mut self) -> usize {
        let n = self.session.pump();
        if n > 0 {
            self.bytes += n;
            self.last_output = Instant::now();
        }
        n
    }

    pub fn ended(&self) -> bool {
        self.session.ended()
    }

    /// Writes `bytes` as typed at the terminal.
    pub fn send(&mut self, bytes: &[u8]) {
        self.session
            .with_pty(|pty| pty.map(|pty| pty.write(bytes)).unwrap_or_default());
    }

    /// Pastes `text`, bracketed when the program asked for it.
    pub fn paste(&mut self, text: &str) {
        self.session.send_paste(text);
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.session
            .resize(usize::from(width), usize::from(height));
    }

    fn contains(&self, text: &str) -> bool {
        let em = self.session.emulator();
        (0..em.height()).any(|y| em.line_text(y).contains(text))
    }

    /// Pumps until the screen shows `text`; whether it did within `timeout`.
    pub fn wait_text(&mut self, text: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            self.pump();
            if self.contains(text) {
                return true;
            }
            if self.ended() || start.elapsed() > timeout {
                self.pump();
                return self.contains(text);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Pumps until the program wrote nothing for `quiet` and is outside a
    /// synchronized update (mode 2026); whether it got quiet within `max`.
    pub fn wait_quiet(&mut self, quiet: Duration, max: Duration) -> bool {
        let start = Instant::now();
        loop {
            self.pump();
            let in_update = self
                .session
                .emulator()
                .mode(TerminalMode::SynchronizedOutput);
            if !in_update && self.last_output.elapsed() >= quiet {
                return true;
            }
            if self.ended() && !in_update {
                return true;
            }
            if start.elapsed() > max {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Pumps until the program has ended; its exit code (None on timeout).
    pub fn wait_exit(&mut self, timeout: Duration) -> Option<i32> {
        let start = Instant::now();
        while !self.ended() && start.elapsed() < timeout {
            self.pump();
            std::thread::sleep(Duration::from_millis(2));
        }
        self.pump();
        if !self.ended() {
            return None;
        }
        self.session.with_pty(|pty| {
            pty.and_then(|pty| {
                pty.wait_exit(Some(timeout));
                pty.exit_code()
            })
        })
    }

    pub fn shot(&self) -> Shot {
        let em = self.session.emulator();
        let rows = (0..em.height())
            .map(|y| em.line_text(y).trim_end().to_owned())
            .collect();
        let scrollback = (0..em.scrollback_len())
            .map(|i| em.scrollback_line(i).text(em.glyphs()).trim_end().to_owned())
            .collect();
        Shot {
            rows,
            scrollback,
            cursor: em.cursor(),
            cursor_visible: em.cursor_visible(),
            alt_screen: em.alternate_screen(),
        }
    }
}
