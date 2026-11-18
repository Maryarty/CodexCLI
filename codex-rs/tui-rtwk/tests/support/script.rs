//! A scenario file: JSONL, one JSON object a line. The first line is the
//! header, the others are steps played in order against each binary.
//!
//! ```text
//! {"scenario":"text-100","mock":"text-100","size":[100,30],"oracle_args":[...],"rtwk_args":[...]}
//! {"wait":"›"}                       wait until the screen shows the text
//! {"send":"Stream 100 events\r"}     type bytes (JSON escapes: \r, \u001b)
//! {"key":"ctrl-c"}                   a named key (see `key_bytes`)
//! {"paste":"text"}                   bracketed paste, as the program asked
//! {"resize":[80,24]}
//! {"sleep_ms":300}
//! {"check":"after-answer","until":"END-text-100"}   compare both screens here
//! {"wait_exit":true}                 wait for the program to end; checks the final screen
//! ```

use std::time::Duration;

/// The header line.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub name: String,
    /// The mock-model scenario (`tools/rtwk-port/mock-model/scenarios`).
    pub mock: String,
    /// A seed written into the fresh home (`seed.rs`), if any.
    pub seed: Option<String>,
    pub size: (u16, u16),
    /// The oracle's arguments after the common ones; `{workdir}` is replaced.
    pub oracle_args: Vec<String>,
    /// codex-rtwk's arguments; `{workdir}` is replaced.
    pub rtwk_args: Vec<String>,
    /// Compare the scrollback too (inline mode).
    pub scrollback: bool,
    /// Why codex-rtwk is expected to differ today, if it is.
    pub gap: Option<String>,
    /// How long a `wait` may take.
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Wait(String),
    Send(Vec<u8>),
    Paste(String),
    Resize(u16, u16),
    Sleep(Duration),
    Check { name: String, until: Option<String> },
    WaitExit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub header: Header,
    pub steps: Vec<Step>,
}

/// The bytes a terminal sends for a named key: `enter`, `esc`, `tab`,
/// `backtab`, `backspace`, `delete`, `space`, arrows, `home`, `end`,
/// `pageup`, `pagedown`, `ctrl-<letter>`, `alt-<char>`.
pub fn key_bytes(name: &str) -> Result<Vec<u8>, String> {
    let _ = name;
    todo!("key_bytes")
}

/// Parses a scenario file.
pub fn parse(text: &str) -> Result<Scenario, String> {
    let _ = text;
    todo!("parse")
}
