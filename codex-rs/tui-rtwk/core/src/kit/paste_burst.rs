//! Paste-burst detection for terminals without bracketed paste: a pure state
//! machine (carried from the oracle's `bottom_pane/paste_burst.rs`, REUSE).
//!
//! A paste often arrives as a rapid stream of `Char`, `Enter` and `Tab` key
//! events. The composer feeds this machine its text-producing characters and
//! applies the decision: hold a first char briefly (flicker suppression),
//! buffer a burst as one pasted string, or let input flow as typing. The
//! machine never touches the text; times are the app clock's (a `Duration`
//! since its start, a model input as in R11), so a test's fake clock drives it.
//!
//! - `on_plain_char` (ASCII, skill-completion punctuation) / `on_plain_char_no_hold`
//!   (other non-ASCII input) decide; the caller appends to the buffer through
//!   `append_char_to_buffer`.
//! - `append_control_char_if_active` takes Enter and Tab into a burst.
//! - `flush_if_due` on a tick: `Typed` inserts that char as typing, `Paste`
//!   inserts the string as a paste.
//! - `flush_before_modified_input` and `clear_window_after_non_char` before
//!   non-text input; `clear_after_explicit_paste` after a real paste.
//!
//! `flush_if_due` compares with `>`: a tick must cross the threshold by 1 ms
//! (`recommended_flush_delay`).

use std::time::Duration;

/// A time on the app's clock (since the app started).
pub type Moment = Duration;

// Heuristic thresholds for detecting paste-like input bursts.
// Detect quickly to avoid showing typed prefix before paste is recognized
const PASTE_BURST_MIN_CHARS: u16 = 3;
const PASTE_ENTER_SUPPRESS_WINDOW: Duration = Duration::from_millis(120);

// Maximum delay between consecutive chars to be considered part of a paste burst.
const PASTE_BURST_CHAR_INTERVAL: Duration = Duration::from_millis(8);

// Idle timeout before flushing buffered paste content.
// Slower paste bursts have been observed in Windows environments.
#[cfg(not(windows))]
const PASTE_BURST_ACTIVE_IDLE_TIMEOUT: Duration = Duration::from_millis(8);
#[cfg(windows)]
const PASTE_BURST_ACTIVE_IDLE_TIMEOUT: Duration = Duration::from_millis(60);

#[derive(Clone, Debug)]
pub struct PasteBurst {
    /// Idle time after which an active burst is flushed
    /// (`PASTE_BURST_ACTIVE_IDLE_TIMEOUT`; a field so a test can run the
    /// slower Windows timing on any platform).
    active_idle_timeout: Duration,
    last_plain_char_time: Option<Moment>,
    consecutive_plain_char_burst: u16,
    burst_window_until: Option<Moment>,
    buffer: String,
    active: bool,
    // Hold first fast char briefly to avoid rendering flicker
    pending_first_char: Option<(char, Moment)>,
}

impl Default for PasteBurst {
    fn default() -> Self {
        Self::with_active_idle_timeout(PASTE_BURST_ACTIVE_IDLE_TIMEOUT)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CharDecision {
    /// Start buffering and retroactively capture some already-inserted chars.
    BeginBuffer { retro_chars: u16 },
    /// We are currently buffering; append the current char into the buffer.
    BufferAppend,
    /// Do not insert/render this char yet; temporarily save the first fast
    /// char while we wait to see if a paste-like burst follows.
    RetainFirstChar,
    /// Begin buffering using the previously saved first char (no retro grab needed).
    BeginBufferFromPending,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RetroGrab {
    pub start_byte: usize,
    pub grabbed: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FlushResult {
    Paste(String),
    Typed(char),
    None,
}

impl PasteBurst {
    /// A detector whose active burst flushes after `timeout` of idle.
    pub fn with_active_idle_timeout(timeout: Duration) -> Self {
        Self {
            active_idle_timeout: timeout,
            last_plain_char_time: None,
            consecutive_plain_char_burst: 0,
            burst_window_until: None,
            buffer: String::new(),
            active: false,
            pending_first_char: None,
        }
    }

    /// Recommended delay to wait between simulated keypresses (or before
    /// scheduling a UI tick) so that a pending fast keystroke is flushed
    /// out of the burst detector as normal typed input.
    ///
    /// Primarily used by tests and by the TUI to reliably cross the
    /// paste-burst timing threshold.
    pub fn recommended_flush_delay() -> Duration {
        PASTE_BURST_CHAR_INTERVAL + Duration::from_millis(1)
    }

    #[cfg(test)]
    pub fn recommended_active_flush_delay() -> Duration {
        PASTE_BURST_ACTIVE_IDLE_TIMEOUT + Duration::from_millis(1)
    }

    /// Entry point: decide how to treat a plain char with current timing.
    pub fn on_plain_char(&mut self, ch: char, now: Moment) -> CharDecision {
        self.note_plain_char(now);

        if self.active {
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            return CharDecision::BufferAppend;
        }

        // If we already held a first char and receive a second fast char,
        // start buffering without retro-grabbing (we never rendered the first).
        if let Some((held, held_at)) = self.pending_first_char
            && now.saturating_sub(held_at) <= PASTE_BURST_CHAR_INTERVAL
        {
            self.active = true;
            // take() to clear pending; we already captured the held char above
            let _ = self.pending_first_char.take();
            self.buffer.push(held);
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            return CharDecision::BeginBufferFromPending;
        }

        if self.consecutive_plain_char_burst >= PASTE_BURST_MIN_CHARS {
            return CharDecision::BeginBuffer {
                retro_chars: self.consecutive_plain_char_burst.saturating_sub(1),
            };
        }

        // Save the first fast char very briefly to see if a burst follows.
        self.pending_first_char = Some((ch, now));
        CharDecision::RetainFirstChar
    }

    /// Like on_plain_char(), but never holds the first char.
    ///
    /// Used for non-ASCII input paths (e.g., IMEs) where holding a character can
    /// feel like dropped input, while still allowing burst-based paste detection.
    ///
    /// Note: This method will only ever return BufferAppend or BeginBuffer.
    pub fn on_plain_char_no_hold(&mut self, now: Moment) -> Option<CharDecision> {
        self.note_plain_char(now);

        if self.active {
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            return Some(CharDecision::BufferAppend);
        }

        if self.consecutive_plain_char_burst >= PASTE_BURST_MIN_CHARS {
            return Some(CharDecision::BeginBuffer {
                retro_chars: self.consecutive_plain_char_burst.saturating_sub(1),
            });
        }

        None
    }

    fn note_plain_char(&mut self, now: Moment) {
        match self.last_plain_char_time {
            Some(prev) if now.saturating_sub(prev) <= PASTE_BURST_CHAR_INTERVAL => {
                self.consecutive_plain_char_burst =
                    self.consecutive_plain_char_burst.saturating_add(1)
            }
            _ => self.consecutive_plain_char_burst = 1,
        }
        self.last_plain_char_time = Some(now);
    }

    /// Flushes any buffered burst if the inter-key timeout has elapsed.
    ///
    /// Returns:
    ///
    /// - [`FlushResult::Paste`] when a paste burst was active and buffered text is emitted as one
    ///   pasted string.
    /// - [`FlushResult::Typed`] when a single fast first char was being held (flicker
    ///   suppression) and no burst followed before the timeout elapsed.
    /// - [`FlushResult::None`] when the timeout has not elapsed, or there is nothing to flush.
    pub fn flush_if_due(&mut self, now: Moment) -> FlushResult {
        let timeout = if self.is_active_internal() {
            self.active_idle_timeout
        } else {
            PASTE_BURST_CHAR_INTERVAL
        };
        let timed_out = self
            .last_plain_char_time
            .is_some_and(|t| now.saturating_sub(t) > timeout);
        if timed_out {
            self.flush_pending()
        } else {
            FlushResult::None
        }
    }

    /// Flush without waiting, retaining the typed-key versus paste distinction.
    pub fn flush_pending(&mut self) -> FlushResult {
        if self.is_active_internal() {
            self.active = false;
            let out = std::mem::take(&mut self.buffer);
            FlushResult::Paste(out)
        } else if let Some((ch, _at)) = self.pending_first_char.take() {
            FlushResult::Typed(ch)
        } else {
            FlushResult::None
        }
    }

    /// Accumulate a newline or tab into the burst instead of invoking a shortcut.
    /// A held first character is promoted into the buffer before the control character.
    ///
    /// Returns true if the character was appended (we are in a burst context),
    /// false otherwise.
    pub fn append_control_char_if_active(&mut self, ch: char, now: Moment) -> bool {
        if self.is_active() {
            if let Some((held, _)) = self.pending_first_char.take() {
                self.buffer.push(held);
            }
            self.active = true;
            self.buffer.push(ch);
            self.last_plain_char_time = Some(now);
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            true
        } else {
            false
        }
    }

    /// Decide if Enter should insert a newline (burst context) vs submit.
    pub fn newline_should_insert_instead_of_submit(&self, now: Moment) -> bool {
        let in_burst_window = self.burst_window_until.is_some_and(|until| now <= until);
        self.is_active() || in_burst_window
    }

    /// Decide if Enter should insert a newline for callers that insert chars immediately.
    pub fn direct_insert_newline_should_insert(&self, now: Moment) -> bool {
        self.newline_should_insert_instead_of_submit(now)
            || self
                .last_plain_char_time
                .is_some_and(|t| now.saturating_sub(t) <= PASTE_BURST_CHAR_INTERVAL)
    }

    /// Keep the burst window alive.
    pub fn extend_window(&mut self, now: Moment) {
        self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
    }

    /// Begin buffering with retroactively grabbed text.
    pub fn begin_with_retro_grabbed(&mut self, grabbed: String, now: Moment) {
        if !grabbed.is_empty() {
            self.buffer.push_str(&grabbed);
        }
        self.active = true;
        self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
    }

    /// Append a char into the burst buffer.
    pub fn append_char_to_buffer(&mut self, ch: char, now: Moment) {
        self.buffer.push(ch);
        self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
    }

    /// Try to append a char into the burst buffer only if a burst is already active.
    ///
    /// Returns true when the char was captured into the existing burst, false otherwise.
    pub fn try_append_char_if_active(&mut self, ch: char, now: Moment) -> bool {
        if self.active || !self.buffer.is_empty() {
            self.append_char_to_buffer(ch, now);
            true
        } else {
            false
        }
    }

    /// Decide whether to begin buffering by retroactively capturing recent
    /// chars from the slice before the cursor.
    ///
    /// Heuristic: if the retro-grabbed slice contains any whitespace or is
    /// sufficiently long (>= 16 characters), treat it as paste-like to avoid
    /// rendering the typed prefix momentarily before the paste is recognized.
    /// This favors responsiveness and prevents flicker for typical pastes
    /// (URLs, file paths, multiline text) while not triggering on short words.
    ///
    /// Returns Some(RetroGrab) with the start byte and grabbed text when we
    /// decide to buffer retroactively; otherwise None.
    pub fn decide_begin_buffer(
        &mut self,
        now: Moment,
        before: &str,
        retro_chars: usize,
    ) -> Option<RetroGrab> {
        let start_byte = retro_start_index(before, retro_chars);
        let grabbed = before[start_byte..].to_string();
        let looks_pastey =
            grabbed.chars().any(char::is_whitespace) || grabbed.chars().count() >= 16;
        if looks_pastey {
            // Note: caller is responsible for removing this slice from UI text.
            self.begin_with_retro_grabbed(grabbed.clone(), now);
            Some(RetroGrab {
                start_byte,
                grabbed,
            })
        } else {
            None
        }
    }

    /// Before applying modified/non-char input: flush buffered burst immediately.
    pub fn flush_before_modified_input(&mut self) -> Option<String> {
        if !self.is_active() {
            return None;
        }
        self.active = false;
        let mut out = std::mem::take(&mut self.buffer);
        if let Some((ch, _at)) = self.pending_first_char.take() {
            out.push(ch);
        }
        Some(out)
    }

    /// Clear only the timing window and any pending first-char.
    ///
    /// Does not emit or clear the buffered text itself; callers should have
    /// already flushed (if needed) via one of the flush methods above.
    pub fn clear_window_after_non_char(&mut self) {
        self.consecutive_plain_char_burst = 0;
        self.last_plain_char_time = None;
        self.burst_window_until = None;
        self.active = false;
        self.pending_first_char = None;
    }

    /// Returns true if we are in any paste-burst related transient state
    /// (actively buffering, have a non-empty buffer, or have saved the first
    /// fast char while waiting for a potential burst).
    pub fn is_active(&self) -> bool {
        self.is_active_internal() || self.pending_first_char.is_some()
    }

    fn is_active_internal(&self) -> bool {
        self.active || !self.buffer.is_empty()
    }

    pub fn clear_after_explicit_paste(&mut self) {
        self.last_plain_char_time = None;
        self.consecutive_plain_char_burst = 0;
        self.burst_window_until = None;
        self.active = false;
        self.buffer.clear();
        self.pending_first_char = None;
    }
}

pub fn retro_start_index(before: &str, retro_chars: usize) -> usize {
    if retro_chars == 0 {
        return before.len();
    }
    before
        .char_indices()
        .rev()
        .nth(retro_chars.saturating_sub(1))
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Behavior: for ASCII input we "hold" the first fast char briefly. If no burst follows,
    /// that held char should eventually flush as normal typed input (not as a paste).
    #[test]
    fn ascii_first_char_is_held_then_flushes_as_typed() {
        let mut burst = PasteBurst::default();
        let t0 = Moment::ZERO;
        assert!(matches!(
            burst.on_plain_char('a', t0),
            CharDecision::RetainFirstChar
        ));

        let t1 = t0 + PasteBurst::recommended_flush_delay() + Duration::from_millis(1);
        assert!(matches!(burst.flush_if_due(t1), FlushResult::Typed('a')));
        assert!(!burst.is_active());
    }

    /// Behavior: if two ASCII chars arrive quickly, we should start buffering without ever
    /// rendering the first one, then flush the whole buffered payload as a paste.
    #[test]
    fn ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste() {
        let mut burst = PasteBurst::default();
        let t0 = Moment::ZERO;
        assert!(matches!(
            burst.on_plain_char('a', t0),
            CharDecision::RetainFirstChar
        ));

        let t1 = t0 + Duration::from_millis(1);
        assert!(matches!(
            burst.on_plain_char('b', t1),
            CharDecision::BeginBufferFromPending
        ));
        burst.append_char_to_buffer('b', t1);

        let t2 = t1 + PasteBurst::recommended_active_flush_delay() + Duration::from_millis(1);
        assert!(matches!(
            burst.flush_if_due(t2),
            FlushResult::Paste(ref s) if s == "ab"
        ));
    }

    /// Behavior: when non-char input is about to be applied, we flush any transient burst state
    /// immediately (including a single pending ASCII char) so state doesn't leak across inputs.
    #[test]
    fn flush_before_modified_input_includes_pending_first_char() {
        let mut burst = PasteBurst::default();
        let t0 = Moment::ZERO;
        assert!(matches!(
            burst.on_plain_char('a', t0),
            CharDecision::RetainFirstChar
        ));

        assert_eq!(burst.flush_before_modified_input(), Some("a".to_string()));
        assert!(!burst.is_active());
    }

    /// Behavior: retro-grab buffering is only enabled when the already-inserted prefix looks
    /// paste-like (whitespace or "long enough") so short IME bursts don't get misclassified.
    #[test]
    fn decide_begin_buffer_only_triggers_for_pastey_prefixes() {
        let mut burst = PasteBurst::default();
        let now = Moment::ZERO;

        assert!(
            burst
                .decide_begin_buffer(now, "ab", /*retro_chars*/ 2)
                .is_none()
        );
        assert!(!burst.is_active());

        let grab = burst
            .decide_begin_buffer(now, "a b", /*retro_chars*/ 2)
            .expect("whitespace should be considered paste-like");
        assert_eq!(grab.start_byte, 1);
        assert_eq!(grab.grabbed, " b");
        assert!(burst.is_active());
    }

    /// Behavior: after a paste-like burst, we keep an "enter suppression window" alive briefly so
    /// a slightly-late Enter still inserts a newline instead of submitting.
    #[test]
    fn newline_suppression_window_outlives_buffer_flush() {
        let mut burst = PasteBurst::default();
        let t0 = Moment::ZERO;
        assert!(matches!(
            burst.on_plain_char('a', t0),
            CharDecision::RetainFirstChar
        ));

        let t1 = t0 + Duration::from_millis(1);
        assert!(matches!(
            burst.on_plain_char('b', t1),
            CharDecision::BeginBufferFromPending
        ));
        burst.append_char_to_buffer('b', t1);

        let t2 = t1 + PasteBurst::recommended_active_flush_delay() + Duration::from_millis(1);
        assert!(matches!(burst.flush_if_due(t2), FlushResult::Paste(ref s) if s == "ab"));
        assert!(!burst.is_active());

        assert!(burst.newline_should_insert_instead_of_submit(t2));
        let t3 = t1 + PASTE_ENTER_SUPPRESS_WINDOW + Duration::from_millis(1);
        assert!(!burst.newline_should_insert_instead_of_submit(t3));
    }
}
