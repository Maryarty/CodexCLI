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
//! {"check":"x","gap":"VS2: why"}     ... and codex-rtwk is expected to differ here
//! {"wait_exit":true}                 wait for the program to end; checks the final screen
//!                                    (checkpoint `after-exit`; it takes a "gap" too)
//! ```

use std::time::Duration;

use serde_json::Value;

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
    /// A checkpoint; `gap` says why codex-rtwk is expected to differ here.
    Check {
        name: String,
        until: Option<String>,
        gap: Option<String>,
    },
    /// The program ends (the screen it leaves is the checkpoint `after-exit`).
    WaitExit {
        gap: Option<String>,
    },
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
    let fixed: &[u8] = match name {
        "enter" => b"\r",
        "esc" => b"\x1b",
        "tab" => b"\t",
        "backtab" => b"\x1b[Z",
        "backspace" => b"\x7f",
        "delete" => b"\x1b[3~",
        "space" => b" ",
        "up" => b"\x1b[A",
        "down" => b"\x1b[B",
        "right" => b"\x1b[C",
        "left" => b"\x1b[D",
        "home" => b"\x1b[H",
        "end" => b"\x1b[F",
        "pageup" => b"\x1b[5~",
        "pagedown" => b"\x1b[6~",
        _ => b"",
    };
    if !fixed.is_empty() {
        return Ok(fixed.to_vec());
    }
    if let Some(letter) = name.strip_prefix("ctrl-") {
        let mut chars = letter.chars();
        if let (Some(c), None) = (chars.next(), chars.next())
            && c.is_ascii_alphabetic()
        {
            return Ok(vec![(c.to_ascii_lowercase() as u8) & 0x1f]);
        }
    }
    if let Some(rest) = name.strip_prefix("alt-") {
        let mut chars = rest.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            let mut bytes = vec![0x1b];
            bytes.extend(c.to_string().as_bytes());
            return Ok(bytes);
        }
    }
    Err(format!("unknown key `{name}`"))
}

fn string_list(value: &Value, key: &str, line: usize) -> Result<Vec<String>, String> {
    let Some(list) = value.get(key) else {
        return Ok(Vec::new());
    };
    list.as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|i| i.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| format!("line {line}: `{key}` must be a list of strings"))
}

fn header(value: &Value, line: usize) -> Result<Header, String> {
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let name =
        text("scenario").ok_or_else(|| format!("line {line}: the header needs `scenario`"))?;
    let mock = text("mock").ok_or_else(|| format!("line {line}: the header needs `mock`"))?;
    let size = match value.get("size") {
        None => (100, 30),
        Some(size) => {
            dims(size).ok_or_else(|| format!("line {line}: `size` is [width, height]"))?
        }
    };
    Ok(Header {
        name,
        mock,
        seed: text("seed"),
        size,
        oracle_args: string_list(value, "oracle_args", line)?,
        rtwk_args: string_list(value, "rtwk_args", line)?,
        scrollback: value
            .get("scrollback")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        timeout: Duration::from_millis(
            value
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or(20_000),
        ),
    })
}

fn dims(value: &Value) -> Option<(u16, u16)> {
    let list = value.as_array()?;
    let [w, h] = list.as_slice() else {
        return None;
    };
    Some((
        u16::try_from(w.as_u64()?).ok()?,
        u16::try_from(h.as_u64()?).ok()?,
    ))
}

fn step(value: &Value, line: usize) -> Result<Step, String> {
    let err = |what: &str| format!("line {line}: {what}");
    let str_of = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    if let Some(text) = str_of("wait") {
        return Ok(Step::Wait(text));
    }
    if let Some(text) = str_of("send") {
        return Ok(Step::Send(text.into_bytes()));
    }
    if let Some(name) = str_of("key") {
        return key_bytes(&name).map(Step::Send).map_err(|e| err(&e));
    }
    if let Some(text) = str_of("paste") {
        return Ok(Step::Paste(text));
    }
    if let Some(size) = value.get("resize") {
        let (w, h) = dims(size).ok_or_else(|| err("`resize` is [width, height]"))?;
        return Ok(Step::Resize(w, h));
    }
    if let Some(ms) = value.get("sleep_ms") {
        let ms = ms.as_u64().ok_or_else(|| err("`sleep_ms` is a number"))?;
        return Ok(Step::Sleep(Duration::from_millis(ms)));
    }
    if let Some(name) = str_of("check") {
        return Ok(Step::Check {
            name,
            until: str_of("until"),
            gap: str_of("gap"),
        });
    }
    if value.get("wait_exit").is_some() {
        return Ok(Step::WaitExit { gap: str_of("gap") });
    }
    Err(err(
        "unknown step (wait, send, key, paste, resize, sleep_ms, check, wait_exit)",
    ))
}

/// Parses a scenario file.
pub fn parse(text: &str) -> Result<Scenario, String> {
    let mut header_line = None;
    let mut steps = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        if raw.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(raw).map_err(|e| format!("line {line}: {e}"))?;
        match &header_line {
            None => header_line = Some(header(&value, line)?),
            Some(_) => steps.push(step(&value, line)?),
        }
    }
    let header = header_line.ok_or("the scenario file is empty")?;
    Ok(Scenario { header, steps })
}
