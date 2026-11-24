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
