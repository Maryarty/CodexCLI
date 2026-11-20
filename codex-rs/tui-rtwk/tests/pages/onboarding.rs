//! Screen tests of the startup page's `onboarding` directory (S01-S04): its
//! oracle tests are the `onboarding` shard (`tests/oracle/onboarding.rs`);
//! these hold what the shard cannot name: the directory's debug entries.

#[test]
fn onboarding_lists_the_startup_page_states() {
    let dirs = codex_tui_rtwk::app::debug::dirs();
    let dir = dirs
        .iter()
        .find(|d| d.name == "onboarding")
        .expect("the onboarding directory");
    let names: Vec<&str> = (dir.debug_entries)().iter().map(|e| e.name).collect();
    for want in [
        "update_prompt",
        "update_prompt_windows",
        "welcome",
        "trust_git_repo",
        "trust_restricted",
        "flow_update_trust",
    ] {
        assert!(names.contains(&want), "no entry `{want}` in {names:?}");
    }
}
