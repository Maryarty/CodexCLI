//! How a test says "not yet" without being skipped: every verdict other than
//! pass is a line the summary (`tests/oracle/tools/summary.py`) counts.
//!
//! - `pending`: the screen, the state or the seed the test needs does not
//!   exist yet. The test body is written in full under the marker; the marker
//!   fails the test with the reason and the owning slice. Under
//!   `KIT_PROBE_PENDING=1` it lets the body run, so a pending test whose body
//!   now passes is found (stale marker).
//! - `missing`: as `pending`, where there is no body to run (the kit itself
//!   cannot open the screen).
//! - `todo`: the test is not ported yet (the generated stubs).
//! - `na`: the oracle test needs what a screen test cannot have (the real
//!   `App`, the backend, a root switch); passes, and says why and whose.
//! - `delta`: the feature works and every functional check of the test passes; only the
//!   comparison of the screen with the oracle's rows or cell styles differs, by a small
//!   visual delta (`docs/rtwk-port/visual-deltas.md`). The test passes and the summary counts
//!   it in its own column, not as a pass.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

/// The slice that will make a pending test pass (tm aliases).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    SessionCore,
    UiPane,
    RoutesLists,
    DialogsPane,
    ChatNavPane,
    ChatLive,
    Mouse,
    ReuseCarry,
    StartupFlow,
    StartupPrompts,
    Onboarding,
    /// The host's doors, feeds and public verbs (`host_capability_list!`, `host_feed_list!`,
    /// `PublicVerb`): what a feature asks of the host or of another owner.
    FwHubsC,
}

impl Owner {
    pub fn slice(self) -> &'static str {
        match self {
            Owner::SessionCore => "session-core",
            Owner::UiPane => "ui-pane",
            Owner::RoutesLists => "routes-lists",
            Owner::DialogsPane => "dialogs-pane",
            Owner::ChatNavPane => "chat-nav-pane",
            Owner::ChatLive => "chat-live",
            Owner::Mouse => "mouse",
            Owner::ReuseCarry => "reuse-carry",
            Owner::StartupFlow => "startup-flow",
            Owner::StartupPrompts => "startup-prompts",
            Owner::Onboarding => "onboarding",
            Owner::FwHubsC => "fw-hubs-c",
        }
    }
}

fn probing() -> bool {
    std::env::var_os("KIT_PROBE_PENDING").is_some_and(|v| v != "0")
}

fn clean(reason: &str) -> String {
    reason.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Marks the test pending: put it first, write the whole test after it.
pub fn pending(owner: Owner, reason: &str) {
    assert!(
        !clean(reason).is_empty(),
        "pending needs a reason: what does not exist yet"
    );
    if probing() {
        println!("PROBE[{}]: {}", owner.slice(), clean(reason));
        return;
    }
    panic!("PENDING[{}]: {}", owner.slice(), clean(reason));
}

/// As `pending`, for a test that cannot run at all.
pub fn missing(owner: Owner, reason: &str) -> ! {
    panic!("PENDING[{}]: {}", owner.slice(), clean(reason));
}

thread_local! {
    /// The delta this test declared (`delta`), if any.
    static DELTA: RefCell<Option<Declared>> = const { RefCell::new(None) };
    /// How many `screen` comparisons are running (their panics are the verdict, not noise).
    static SOFT: Cell<usize> = const { Cell::new(0) };
}

struct Declared {
    owner: Owner,
    what: String,
    hits: usize,
}

/// Panic messages of a comparison `delta` turns into a verdict stay off stderr.
fn quiet_soft_panics() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if SOFT.with(Cell::get) == 0 {
                previous(info);
            }
        }));
    });
}

fn message_of(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_else(|| "a comparison failed".to_string())
}

/// The test passes with a small visual delta: put it first, write the whole test after it. Every
/// functional check of the test still runs and must pass; only the kit's comparisons of the
/// screen with the oracle's rows or cell styles (`assert_snap*`, `assert_rows_are`, the help and
/// layout comparisons: they all go through `screen` / `screen_styles`) are softened. Such a
/// comparison that differs prints `DELTA[slice]: what` (the summary counts the test as a delta,
/// not a pass) and the test goes on; one that matches costs nothing.
///
/// What counts as small is enforced, not trusted (docs/rtwk-port/visual-deltas.md): rows may
/// differ in colour, indent, glyphs, spacing and wrapping, never in what they say: a row
/// comparison whose letters and digits, read in order, differ from the oracle's (a missing row,
/// another word, a clipped text) is refused and fails the test. The cursor and every text, link,
/// style-presence or count check stay strict.
pub fn delta(owner: Owner, what: &str) {
    let what = clean(what);
    assert!(
        !what.is_empty(),
        "delta needs `what`: the small visual difference, in words"
    );
    quiet_soft_panics();
    DELTA.with(|d| {
        *d.borrow_mut() = Some(Declared {
            owner,
            what,
            hits: 0,
        })
    });
}

/// How many comparisons of this test differed under its `delta` (0 without one).
pub fn delta_hits() -> usize {
    DELTA.with(|d| d.borrow().as_ref().map_or(0, |d| d.hits))
}

/// The letters and digits of `rows`, in order: what the rows say, whatever the colour, indent,
/// glyph, spacing or wrap.
fn said<S: AsRef<str>>(rows: &[S]) -> String {
    rows.iter()
        .flat_map(|row| row.as_ref().chars())
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Runs `compare` under the declared delta: a failure is recorded, and `Err(message)` says it.
fn soft(compare: impl FnOnce()) -> Option<String> {
    SOFT.with(|s| s.set(s.get() + 1));
    let result = panic::catch_unwind(AssertUnwindSafe(compare));
    SOFT.with(|s| s.set(s.get() - 1));
    result.err().map(|payload| message_of(payload.as_ref()))
}

fn record(message: &str) {
    DELTA.with(|d| {
        if let Some(d) = d.borrow_mut().as_mut() {
            if d.hits == 0 {
                println!("DELTA[{}]: {}", d.owner.slice(), d.what);
            }
            d.hits += 1;
        }
    });
    for line in message.lines() {
        println!("  | {line}");
    }
}

/// A comparison of the screen's rows `ours` with the oracle's rows `oracle` (`compare` panics when
/// they differ). Without `delta` it is the comparison itself; under `delta` a difference that
/// keeps what the rows say becomes the `DELTA` line and the test goes on, any other difference
/// still fails the test.
pub(super) fn screen<A: AsRef<str>, B: AsRef<str>>(
    ours: &[A],
    oracle: &[B],
    compare: impl FnOnce(),
) {
    if DELTA.with(|d| d.borrow().is_none()) {
        compare();
        return;
    }
    let Some(message) = soft(compare) else {
        return;
    };
    let (ours, oracle) = (said(ours), said(oracle));
    assert!(
        ours == oracle,
        "{message}\n\nkit::delta refused: the rows say other things than the oracle's (not a small visual delta: \
         docs/rtwk-port/visual-deltas.md; keep the test pending)\nours:   {ours}\noracle: {oracle}"
    );
    record(&message);
}

/// As `screen` for a comparison of cell styles (colours, modifiers): a difference there is always
/// a visual delta, whatever the words.
pub(super) fn screen_styles(compare: impl FnOnce()) {
    if DELTA.with(|d| d.borrow().is_none()) {
        compare();
    } else if let Some(message) = soft(compare) {
        record(&message);
    }
}

/// A generated stub: the oracle test is not ported yet.
pub fn todo() -> ! {
    panic!("TODO[port]: this oracle test is not ported yet");
}

/// The oracle test cannot be a screen test; `reason` says why and `owner`
/// which slice covers it with a state test.
pub fn na(owner: Owner, reason: &str) {
    assert!(
        !clean(reason).is_empty(),
        "na needs a reason: what the test needs that a screen test lacks"
    );
    println!("N/A[{}]: {}", owner.slice(), clean(reason));
}
