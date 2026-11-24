//! The keymap config (`tui.keymap.*`) made a `Keymap`: precedence, chords,
//! the leader key, new defaults that yield, and the conflict passes. The
//! expected values are the oracle's (`tui/src/keymap.rs` tests, `chords_tests.rs`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::keymap::actions::{
    ALL, CHAT_INTERRUPT_TURN, CHAT_OPEN_PRESETS, COMPOSER_QUEUE, COMPOSER_SUBMIT,
    COMPOSER_TOGGLE_SHORTCUTS, GLOBAL_CHANGE_HOST, GLOBAL_CLEAR_TERMINAL, GLOBAL_FOCUS_ACTIVITY,
    GLOBAL_OPEN_AGENTS, GLOBAL_OPEN_TRANSCRIPT, GLOBAL_SELECT_TRANSCRIPT,
    GLOBAL_TOGGLE_SIDE_CONVERSATION, LIST_ACCEPT, LIST_CANCEL, LIST_JUMP_TOP, LIST_MOVE_DOWN,
    LIST_MOVE_RIGHT, LIST_MOVE_UP, PAGER_CLOSE, PAGER_FIND, VIM_NORMAL_JUMP_TOP,
};
use codex_tui_core::keymap::catalogue::ROWS;
use codex_tui_core::keymap::{Binding, Context, Keymap, KeymapSpec, Stroke};
use serde_json::json;

fn stroke(spec: &str) -> Stroke {
    Stroke::parse(spec).expect("a stroke")
}

fn resolve(spec: KeymapSpec) -> Keymap {
    Keymap::resolve(&spec).expect("a valid keymap")
}

fn error_of(spec: KeymapSpec) -> String {
    match Keymap::resolve(&spec) {
        Ok(_) => panic!("the spec should conflict: {spec:?}"),
        Err(error) => error.message(),
    }
}

fn chord(prefix: &str, completion: &str) -> Binding {
    Binding::Chord {
        prefix: stroke(prefix),
        completion: stroke(completion),
    }
}

// The catalogue

#[test]
fn the_action_handles_are_the_catalogue() {
    assert_eq!(ROWS.len(), 173, "the oracle's action count");
    let rows: Vec<_> = ROWS.iter().map(|row| row.id).collect();
    assert_eq!(ALL, rows.as_slice());
}

#[test]
fn the_built_in_defaults_are_valid_and_the_oracles() {
    let keymap = Keymap::defaults();
    assert_eq!(keymap.singles(GLOBAL_OPEN_TRANSCRIPT), [stroke("ctrl-t")]);
    assert_eq!(
        keymap.singles(LIST_MOVE_DOWN),
        [
            stroke("down"),
            stroke("ctrl-n"),
            stroke("ctrl-j"),
            stroke("j")
        ]
    );
    assert_eq!(keymap.singles(LIST_ACCEPT), [stroke("enter")]);
    assert_eq!(keymap.singles(LIST_CANCEL), [stroke("esc")]);
    assert_eq!(keymap.singles(COMPOSER_SUBMIT), [stroke("enter")]);
    assert_eq!(keymap.singles(COMPOSER_QUEUE), [stroke("tab")]);
    assert_eq!(keymap.singles(CHAT_INTERRUPT_TURN), [stroke("esc")]);
    assert_eq!(keymap.singles(PAGER_CLOSE), [stroke("q"), stroke("ctrl-c")]);
    assert_eq!(keymap.singles(GLOBAL_FOCUS_ACTIVITY), [stroke("f4")]);
    assert_eq!(
        keymap.singles(GLOBAL_TOGGLE_SIDE_CONVERSATION),
        [stroke("ctrl-/")]
    );
    assert!(!keymap.is_configured(LIST_ACCEPT));
}

#[test]
fn a_hint_is_the_first_binding() {
    let keymap = Keymap::defaults();
    assert_eq!(keymap.label(GLOBAL_OPEN_TRANSCRIPT).as_deref(), Some("⌃t"));
    assert_eq!(keymap.label(LIST_ACCEPT).as_deref(), Some("enter"));
    assert_eq!(keymap.label(LIST_CANCEL).as_deref(), Some("esc"));
    assert_eq!(keymap.label(GLOBAL_OPEN_AGENTS), None);
    assert_eq!(keymap.label(GLOBAL_CHANGE_HOST).as_deref(), Some("⌃x b"));
    let keymap = resolve(KeymapSpec::default().with("list", "accept", &["alt-shift-up"]));
    assert_eq!(keymap.label(LIST_ACCEPT).as_deref(), Some("⇧⌥↑"));
}

// Precedence: context, then global (the composer's three), then built-in

#[test]
fn a_configured_key_replaces_the_default() {
    let keymap = resolve(KeymapSpec::default().with("list", "move_down", &["z"]));
    assert_eq!(keymap.singles(LIST_MOVE_DOWN), [stroke("z")]);
    assert!(keymap.is_configured(LIST_MOVE_DOWN));
    assert_eq!(
        keymap.singles(LIST_MOVE_UP).len(),
        4,
        "the others keep theirs"
    );
}

#[test]
fn an_empty_list_unbinds() {
    let keymap = resolve(KeymapSpec::default().with("list", "cancel", &[]));
    assert!(keymap.singles(LIST_CANCEL).is_empty());
    assert!(keymap.is_configured(LIST_CANCEL));
    assert_eq!(keymap.label(LIST_CANCEL), None);
}

#[test]
fn the_composer_reads_global_when_its_own_is_unset() {
    let keymap = resolve(KeymapSpec::default().with("global", "submit", &["ctrl-enter"]));
    assert_eq!(keymap.singles(COMPOSER_SUBMIT), [stroke("ctrl-enter")]);
    let keymap = resolve(KeymapSpec::default().with("global", "queue", &["ctrl-q"]));
    assert_eq!(keymap.singles(COMPOSER_QUEUE), [stroke("ctrl-q")]);
    let keymap = resolve(KeymapSpec::default().with("global", "toggle_shortcuts", &["f1"]));
    assert_eq!(keymap.singles(COMPOSER_TOGGLE_SHORTCUTS), [stroke("f1")]);
}

#[test]
fn the_composers_own_entry_beats_global_and_an_empty_one_unbinds() {
    let spec = KeymapSpec::default()
        .with("global", "submit", &["ctrl-s"])
        .with("composer", "submit", &["ctrl-y"]);
    assert_eq!(resolve(spec).singles(COMPOSER_SUBMIT), [stroke("ctrl-y")]);
    let spec = KeymapSpec::default()
        .with("global", "submit", &["ctrl-y"])
        .with("composer", "submit", &[]);
    assert!(resolve(spec).singles(COMPOSER_SUBMIT).is_empty());
}

#[test]
fn a_repeated_key_is_one_binding_in_first_seen_order() {
    let keymap = resolve(KeymapSpec::default().with("list", "accept", &["f6", "enter", "f6"]));
    assert_eq!(keymap.singles(LIST_ACCEPT), [stroke("f6"), stroke("enter")]);
    // A terminal sends one key for Ctrl+5 and Ctrl+], and one for Ctrl+7 and Ctrl+/.
    assert!(stroke("ctrl-5").same_key(&stroke("ctrl-]")));
    assert!(stroke("ctrl-7").same_key(&stroke("ctrl-/")));
    let keymap = resolve(KeymapSpec::default().with("pager", "find", &["ctrl-8", "ctrl-8"]));
    assert_eq!(keymap.singles(PAGER_FIND).len(), 1);
}

// Config JSON

#[test]
fn the_config_json_is_read_as_the_config_writes_it() {
    let spec = KeymapSpec::from_json(&json!({
        "list": { "accept": "ctrl-y", "cancel": ["q", "esc"], "move_up": null },
        "global": { "submit": [] },
        "composer": null,
    }));
    assert_eq!(spec.get("list", "accept"), Some(&["ctrl-y".to_owned()][..]));
    assert_eq!(spec.get("list", "cancel").map(<[String]>::len), Some(2));
    assert_eq!(spec.get("list", "move_up"), None, "null is unset");
    assert_eq!(spec.get("global", "submit"), Some(&[][..]), "[] is unbound");
    assert!(KeymapSpec::from_json(&json!({})).is_empty());
}

// Chords and the leader

#[test]
fn a_chord_is_a_prefix_and_a_completion() {
    let keymap = resolve(KeymapSpec::default().with("global", "open_transcript", &["f10 enter"]));
    assert_eq!(
        keymap.bindings(GLOBAL_OPEN_TRANSCRIPT),
        [chord("f10", "enter")]
    );
    assert_eq!(
        keymap.label(GLOBAL_OPEN_TRANSCRIPT).as_deref(),
        Some("f10 enter")
    );
}

#[test]
fn single_keys_come_before_chords_in_a_hint() {
    let keymap =
        resolve(KeymapSpec::default().with("global", "open_transcript", &["f10 enter", "f6"]));
    assert_eq!(keymap.label(GLOBAL_OPEN_TRANSCRIPT).as_deref(), Some("f6"));
    assert_eq!(keymap.bindings(GLOBAL_OPEN_TRANSCRIPT).len(), 2);
}

#[test]
fn the_leader_is_ctrl_x_and_frees_the_default_chords() {
    let keymap = Keymap::defaults();
    assert_eq!(keymap.leaders(), [stroke("ctrl-x")]);
    assert_eq!(keymap.bindings(GLOBAL_CHANGE_HOST), [chord("ctrl-x", "b")]);
    assert_eq!(
        keymap.bindings(GLOBAL_SELECT_TRANSCRIPT),
        [chord("ctrl-x", "[")]
    );
    assert_eq!(keymap.bindings(CHAT_OPEN_PRESETS), [chord("ctrl-x", "p")]);
}

#[test]
fn a_leader_binding_expands_to_every_leader_key() {
    let spec = KeymapSpec::default()
        .with("global", "leader", &["f10", "f11"])
        .with("global", "open_transcript", &["leader t"]);
    let keymap = resolve(spec);
    assert_eq!(keymap.leaders(), [stroke("f10"), stroke("f11")]);
    assert_eq!(
        keymap.bindings(GLOBAL_OPEN_TRANSCRIPT),
        [chord("f10", "t"), chord("f11", "t")]
    );
    // The implicit shortcuts follow the configured leaders.
    assert_eq!(
        keymap.bindings(GLOBAL_CHANGE_HOST),
        [chord("f10", "b"), chord("f11", "b")]
    );
}

#[test]
fn the_default_leader_yields_to_a_key_the_user_uses() {
    let keymap = resolve(KeymapSpec::default().with("list", "accept", &["ctrl-x"]));
    assert!(keymap.leaders().is_empty());
    assert!(keymap.bindings(GLOBAL_CHANGE_HOST).is_empty());
    let keymap = resolve(KeymapSpec::default().with("composer", "submit", &["ctrl-x enter"]));
    assert!(keymap.leaders().is_empty(), "a chord prefix counts too");
}

#[test]
fn an_explicit_binding_or_unbind_beats_an_implicit_leader_chord() {
    let keymap = resolve(KeymapSpec::default().with("global", "change_host", &[]));
    assert!(keymap.bindings(GLOBAL_CHANGE_HOST).is_empty());
    let keymap = resolve(KeymapSpec::default().with("global", "change_host", &["f10"]));
    assert_eq!(keymap.bindings(GLOBAL_CHANGE_HOST), [stroke_binding("f10")]);
}

fn stroke_binding(spec: &str) -> Binding {
    Binding::Single(stroke(spec))
}

#[test]
fn vim_jump_top_is_g_g_unless_g_is_taken() {
    let keymap = Keymap::defaults();
    assert_eq!(keymap.bindings(VIM_NORMAL_JUMP_TOP), [chord("g", "g")]);
    let spec = KeymapSpec::default().with("vim_normal", "jump_top", &["home"]);
    assert_eq!(
        resolve(spec).bindings(VIM_NORMAL_JUMP_TOP),
        [stroke_binding("home")]
    );
    let spec = KeymapSpec::default().with("vim_normal", "move_left", &["g"]);
    assert!(resolve(spec).bindings(VIM_NORMAL_JUMP_TOP).is_empty());
}

// New defaults yield to what the user set

#[test]
fn a_new_default_gives_way_to_a_key_the_user_set() {
    // `home` is list.jump_top's default; the user moves list.move_up to it.
    let keymap = resolve(KeymapSpec::default().with("list", "move_up", &["home"]));
    assert_eq!(keymap.singles(LIST_MOVE_UP), [stroke("home")]);
    assert!(keymap.singles(LIST_JUMP_TOP).is_empty());
    // `ctrl-l` is list.move_right's default; clear_terminal is not a yielding default.
    let keymap = resolve(KeymapSpec::default().with("composer", "submit", &["f4"]));
    assert!(
        keymap.singles(GLOBAL_FOCUS_ACTIVITY).is_empty(),
        "f4 is the user's now"
    );
    let keymap = resolve(KeymapSpec::default().with("list", "accept", &["ctrl-7"]));
    assert!(keymap.singles(GLOBAL_TOGGLE_SIDE_CONVERSATION).is_empty());
}

#[test]
fn a_pane_shortcut_is_ignored_without_panes() {
    let keymap = resolve(KeymapSpec::default().with("global", "focus_main_composer", &["ctrl-t"]));
    assert_eq!(keymap.singles(GLOBAL_OPEN_TRANSCRIPT), [stroke("ctrl-t")]);
}

// Conflicts and reserved keys, with the oracle's texts

#[test]
fn a_spec_that_is_no_key_is_an_error() {
    assert_eq!(
        error_of(KeymapSpec::default().with("list", "accept", &["bogus"])),
        "Invalid `tui.keymap.list.accept` = `bogus`. Use values like `ctrl-a`, `shift-enter`, or \
`page-down`. See the Codex keymap documentation for supported actions and examples."
    );
    assert_eq!(
        error_of(KeymapSpec::default().with("list", "accept", &["a b c"])),
        "Invalid `tui.keymap.list.accept` = `a b c`. Use a single key such as `ctrl-a` or a \
two-stroke chord such as `ctrl-x ctrl-t`."
    );
    assert_eq!(
        error_of(KeymapSpec::default().with("list", "accept", &["leader"])),
        "Invalid `tui.keymap.list.accept` = `leader`. Use values like `ctrl-a`, `shift-enter`, or \
`page-down`. See the Codex keymap documentation for supported actions and examples."
    );
    assert!(
        error_of(KeymapSpec::default().with("list", "accept", &["leader a b"]))
            .contains("`leader a b`. Use `leader` followed by one key")
    );
    assert!(
        error_of(KeymapSpec::default().with("list", "accept", &["leader bogus"]))
            .contains("Use `leader` followed by one key")
    );
}

#[test]
fn one_key_may_not_be_two_actions_of_a_context() {
    // list.move_down's default holds `down`.
    let message = error_of(KeymapSpec::default().with("list", "move_up", &["down"]));
    assert!(
        message.starts_with(
            "Ambiguous `tui.keymap.list` bindings: `move_up` and `move_down` use the same key."
        ),
        "{message}"
    );
}

#[test]
fn one_key_may_not_be_two_actions_that_are_active_together() {
    // ctrl-t opens the transcript before the composer's editor reads a key.
    let message = error_of(KeymapSpec::default().with("editor", "move_word_left", &["ctrl-t"]));
    assert!(
        message.contains("`open_transcript` shadows `editor.move_word_left`"),
        "{message}"
    );
    // The app's keys run before an approval or list overlay reads a key.
    let message = error_of(KeymapSpec::default().with("list", "accept", &["ctrl-t"]));
    assert!(
        message.contains("`open_transcript` shadows `list.accept`"),
        "{message}"
    );
    // ... except the pair the oracle allows: ctrl-l clears the terminal and moves a list.
    let keymap = Keymap::defaults();
    assert!(keymap.is_single(GLOBAL_CLEAR_TERMINAL, &stroke("ctrl-l")));
    assert!(keymap.is_single(LIST_MOVE_RIGHT, &stroke("ctrl-l")));
}

#[test]
fn contexts_that_are_never_active_together_may_share_a_key() {
    // pager.close_transcript and global.open_transcript are both ctrl-t.
    let keymap = Keymap::defaults();
    assert!(keymap.is_single(GLOBAL_OPEN_TRANSCRIPT, &stroke("ctrl-t")));
    // composer and list: enter submits one and accepts the other.
    resolve(KeymapSpec::default().with("composer", "submit", &["enter"]));
}

#[test]
fn a_reserved_key_stays_reserved() {
    let message = error_of(KeymapSpec::default().with("composer", "submit", &["ctrl-c"]));
    assert!(
        message.contains("`composer.submit` uses a key reserved by `fixed.interrupt_or_quit`"),
        "{message}"
    );
    // interrupt_turn may keep esc, the one allowed overlap.
    resolve(KeymapSpec::default().with("chat", "interrupt_turn", &["esc"]));
}

#[test]
fn a_printable_key_is_text_input() {
    assert_eq!(
        error_of(KeymapSpec::default().with("global", "change_host", &["x"])),
        "tui.keymap.global.change_host: printable keys are reserved for text input"
    );
}

#[test]
fn ctrl_z_suspends_the_terminal() {
    assert_eq!(
        error_of(KeymapSpec::default().with("global", "open_agents", &["ctrl-z"])),
        "tui.keymap.global.open_agents: ctrl-z is reserved for suspend"
    );
}

#[test]
fn a_chord_prefix_may_not_hide_a_key() {
    // ctrl-t is global.open_transcript; as a prefix it would swallow it.
    let message = error_of(KeymapSpec::default().with("composer", "queue", &["ctrl-t enter"]));
    assert!(
        message.contains("its prefix shadows `tui.keymap.global.open_transcript`"),
        "{message}"
    );
}

#[test]
fn a_chord_may_not_use_esc_or_a_plain_character_prefix() {
    let message = error_of(KeymapSpec::default().with("global", "open_agents", &["f10 esc"]));
    assert!(message.contains("plain `esc` is reserved for cancelling a pending chord"));
    let message = error_of(KeymapSpec::default().with("global", "open_agents", &["a b"]));
    assert!(message.contains("a chord prefix outside Vim or the command center must use ctrl"));
    // Vim may.
    resolve(KeymapSpec::default().with("vim_normal", "move_left", &["a b"]));
}

#[test]
fn the_same_chord_may_not_be_two_actions() {
    let spec = KeymapSpec::default()
        .with("global", "open_agents", &["f10 b"])
        .with("global", "open_transcript", &["f10 b"]);
    let message = error_of(spec);
    assert!(
        message.contains("the same chord is already assigned to"),
        "{message}"
    );
}

#[test]
fn the_leader_is_a_prefix_key_that_text_input_does_not_need() {
    assert_eq!(
        error_of(KeymapSpec::default().with("global", "leader", &["ctrl-a b"])),
        "Invalid `tui.keymap.global.leader`: use a single prefix key."
    );
    assert!(
        error_of(KeymapSpec::default().with("global", "leader", &["x"]))
            .contains("use ctrl, alt, super, or a non-character key")
    );
    assert!(
        error_of(KeymapSpec::default().with("global", "leader", &["ctrl-z"]))
            .contains("ctrl-z is reserved for suspending the terminal on Unix")
    );
    assert!(
        error_of(KeymapSpec::default().with("global", "leader", &["ctrl-c"]))
            .contains("this key is reserved by `fixed.interrupt_or_quit`")
    );
}

#[test]
fn repeating_the_leader_cancels() {
    let spec = KeymapSpec::default()
        .with("global", "leader", &["f10"])
        .with("global", "open_agents", &["f10 f10"]);
    let message = error_of(spec);
    assert!(
        message.contains("repeating the leader is reserved for cancellation"),
        "{message}"
    );
}

#[test]
fn too_many_leader_combinations_are_refused() {
    let keys: Vec<String> = (1..=24).map(|n| format!("alt-f{n}")).collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let mut spec = KeymapSpec::default().with("global", "leader", &refs);
    for row in ROWS
        .iter()
        .filter(|row| row.id.context != Context::Global)
        .take(45)
    {
        spec = spec.with(row.id.context.config_name(), row.id.name, &["leader a"]);
    }
    // 45 actions x 24 leader keys = 1080, over the 1024 the oracle allows.
    assert!(error_of(spec).starts_with("Too many leader shortcut combinations (maximum 1024)."));
}

// The unchecked resolve the kit uses

#[test]
fn the_unchecked_resolve_keeps_what_validation_would_refuse() {
    let spec = KeymapSpec::default().with("composer", "submit", &["ctrl-t"]);
    assert!(
        Keymap::resolve(&spec).is_err(),
        "ctrl-t is open_transcript's"
    );
    let keymap = Keymap::resolve_unchecked(&spec);
    assert_eq!(keymap.singles(COMPOSER_SUBMIT), [stroke("ctrl-t")]);
}

// The causes of a refusal are typed: a caller tells them apart without reading the text.

#[test]
fn a_refusal_names_its_cause() {
    use codex_tui_core::keymap::{ActionId, KeymapError, Shape};
    let error = |spec: KeymapSpec| Keymap::resolve(&spec).expect_err("the spec is refused");
    let accept = ActionId::new(Context::List, "accept");
    assert_eq!(
        error(KeymapSpec::default().with("list", "accept", &["bogus"])),
        KeymapError::Invalid {
            id: accept,
            raw: "bogus".to_owned(),
            shape: Shape::Single,
        }
    );
    assert_eq!(
        error(KeymapSpec::default().with("list", "accept", &["a b c"])),
        KeymapError::Invalid {
            id: accept,
            raw: "a b c".to_owned(),
            shape: Shape::Chord,
        }
    );
    assert!(matches!(
        error(KeymapSpec::default().with("list", "move_up", &["down"])),
        KeymapError::Ambiguous { scope: "list", shadows: false, .. }
    ));
    assert!(matches!(
        error(KeymapSpec::default().with("global", "leader", &["a b"])),
        KeymapError::LeaderNotSingle
    ));
}
