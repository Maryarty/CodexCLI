//! What the settings dialogs are before they are saved (ADR-013): the checks a user made, the
//! order they gave, the page they went into. In Flutter it is the `State` of the sheet (a
//! `Set<String> checked`, a `List<String> order` and a `pageIndex`), initialised from what the
//! dialog is offered (`widget.items`) and read together with it: the model keeps the catalogue
//! as the server and the host gave it, and learns of the draft only through the request a
//! dialog makes (`Edit`). Each draft reads as the dialog's model type (`view`), so a sheet
//! draws the same rows whether the user changed them or not.

use std::collections::{BTreeMap, BTreeSet};

use codex_tui_core::screens::settings::hooks::{HookEvent, HookTrust, Hooks};
use codex_tui_core::screens::settings::{
    Edit, Experimental, Memories, Setup, SetupKind, USE_THEME_COLORS,
};

/// The status-line and title setups, and the skills toggle: which items are checked and in
/// which order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SetupDraft {
    checked: BTreeMap<String, bool>,
    order: Option<Vec<String>>,
}

impl SetupDraft {
    /// The setup as the user left it: the items in their order, each with its check.
    pub fn view(&self, offered: &Setup) -> Setup {
        let mut setup = offered.clone();
        if let Some(order) = &self.order {
            let rank = |id: &str| {
                order
                    .iter()
                    .position(|seen| seen == id)
                    .unwrap_or(usize::MAX)
            };
            setup.items.sort_by_key(|item| rank(&item.id));
        }
        for item in &mut setup.items {
            if let Some(enabled) = self.checked.get(&item.id) {
                item.enabled = *enabled;
            }
        }
        setup
    }

    /// The draft takes the setup `edited` (a view the user changed) as it is.
    fn keep(&mut self, edited: &Setup) {
        self.order = Some(edited.items.iter().map(|item| item.id.clone()).collect());
        self.checked = edited
            .items
            .iter()
            .map(|item| (item.id.clone(), item.enabled))
            .collect();
    }

    /// Space: flips the item. The skills are saved as each one is flipped.
    pub fn toggle(&mut self, offered: &Setup, id: &str) -> Option<Edit> {
        let mut setup = self.view(offered);
        let item = setup.items.iter_mut().find(|item| item.id == id)?;
        item.enabled = !item.enabled;
        let enabled = item.enabled;
        self.keep(&setup);
        (setup.kind == SetupKind::Skills).then(|| Edit::SkillToggled(id.to_owned(), enabled))
    }

    /// Left / Right: moves the item one step among the orderable ones. A row that is not
    /// orderable neither moves nor is crossed.
    pub fn shift(&mut self, offered: &Setup, id: &str, delta: i32) {
        let mut setup = self.view(offered);
        let Some(at) = setup.position(id) else {
            return;
        };
        let to = match at.checked_add_signed(delta.signum() as isize) {
            Some(to) if to < setup.items.len() => to,
            _ => return,
        };
        if setup.items[at].orderable && setup.items[to].orderable {
            setup.items.swap(at, to);
            self.keep(&setup);
        }
    }

    /// Enter: what the setup saves (the enabled items in order, the colour preference apart).
    pub fn save(&self, offered: &Setup) -> Edit {
        let setup = self.view(offered);
        let items = setup
            .items
            .iter()
            .filter(|item| item.enabled && item.id != USE_THEME_COLORS)
            .map(|item| item.id.clone())
            .collect();
        let use_theme_colors = setup
            .items
            .iter()
            .any(|item| item.id == USE_THEME_COLORS && item.enabled);
        Edit::SetupSaved {
            kind: setup.kind,
            items,
            use_theme_colors,
        }
    }

    /// Esc: the skills are read again, the others are given up.
    pub fn cancel(offered: &Setup) -> Edit {
        match offered.kind {
            SetupKind::Skills => Edit::SkillsClosed,
            kind => Edit::SetupCancelled(kind),
        }
    }
}

/// The memories dialog: the two switches as flipped, and whether the reset confirmation is up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoriesDraft {
    use_memories: bool,
    generate_memories: bool,
    pub confirming: bool,
}

impl MemoriesDraft {
    /// The dialog opens on the switches, or on the confirmation (`confirming`).
    pub fn new(offered: &Memories, confirming: bool) -> Self {
        Self {
            use_memories: offered.use_memories,
            generate_memories: offered.generate_memories,
            confirming,
        }
    }

    pub fn view(&self) -> Memories {
        Memories {
            use_memories: self.use_memories,
            generate_memories: self.generate_memories,
        }
    }

    /// Space on a switch (row 0: use, row 1: generate); the confirmation has none.
    pub fn toggle(&mut self, row: usize) {
        if self.confirming {
            return;
        }
        match row {
            0 => self.use_memories = !self.use_memories,
            1 => self.generate_memories = !self.generate_memories,
            _ => {}
        }
    }

    /// Enter on a switch: what is saved.
    pub fn save(&self) -> Option<Edit> {
        (!self.confirming).then_some(Edit::MemoriesSaved {
            use_memories: self.use_memories,
            generate_memories: self.generate_memories,
        })
    }

    /// Enter on "Reset all memories" in the confirmation.
    pub fn reset(&self) -> Option<Edit> {
        self.confirming.then_some(Edit::MemoriesReset)
    }
}

/// The experimental features: the keys flipped since the config was read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeaturesDraft {
    flipped: BTreeSet<String>,
    /// How many confirmed writes the draft has seen (`Experimental::confirmed`).
    seen: u32,
}

impl FeaturesDraft {
    /// A dialog that comes with writes that did not confirm shows their values: the keys
    /// are flipped from the start.
    pub fn new(offered: &Experimental) -> Self {
        Self {
            flipped: offered.unconfirmed.iter().cloned().collect(),
            seen: offered.confirmed,
        }
    }

    /// The server confirmed a write since the last call: the features are what the config
    /// reads now, nothing is flipped. Returns whether that happened.
    pub fn sync(&mut self, offered: &Experimental) -> bool {
        if offered.confirmed == self.seen {
            return false;
        }
        self.seen = offered.confirmed;
        self.flipped.clear();
        true
    }

    /// The dialog with each feature at the value the user gave it.
    pub fn view(&self, offered: &Experimental) -> Experimental {
        let mut dialog = offered.clone();
        for feature in &mut dialog.features {
            feature.enabled = offered.enabled(feature, &self.flipped);
        }
        dialog
    }

    /// Space: flips a feature that can be written, unless a write is on its way.
    pub fn toggle(&mut self, offered: &Experimental, key: &str) {
        if offered.saving {
            return;
        }
        if offered
            .features
            .iter()
            .any(|feature| feature.key == key && feature.writable)
            && !self.flipped.remove(key)
        {
            self.flipped.insert(key.to_owned());
        }
    }

    /// Enter: a write for what changed, or the close when nothing did; while a write is on
    /// its way it closes (`save_changes`).
    pub fn save(&self, offered: &Experimental) -> Saving {
        let updates = offered.updates(&self.flipped);
        if offered.saving || updates.is_empty() {
            Saving::Close
        } else {
            Saving::Write(Edit::FeaturesSave(updates))
        }
    }

    /// Esc saves what changed and closes (`on_ctrl_c`); after a write that did not confirm it
    /// only closes, so that it always can.
    pub fn close(&self, offered: &Experimental) -> Option<Edit> {
        if !offered.unconfirmed.is_empty() || offered.saving {
            return None;
        }
        let updates = offered.updates(&self.flipped);
        (!updates.is_empty()).then_some(Edit::FeaturesSave(updates))
    }
}

/// What Enter does in the experimental features.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saving {
    Close,
    Write(Edit),
}

/// What the hooks browser draws: the hooks as the user left them, and the page open (none: the
/// events, an event: its hooks).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HooksView {
    pub browser: Hooks,
    pub page: Option<HookEvent>,
}

impl std::ops::Deref for HooksView {
    type Target = Hooks;

    fn deref(&self) -> &Hooks {
        &self.browser
    }
}

/// The hooks browser: which hooks the user switched or trusted, and the page open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HooksDraft {
    enabled: BTreeMap<String, bool>,
    trusted: BTreeSet<String>,
    /// The page open: none the list of events, an event the list of its hooks.
    pub page: Option<HookEvent>,
}

impl HooksDraft {
    /// The browser as the user left it: the hooks and the page open.
    pub fn view(&self, offered: &Hooks) -> HooksView {
        HooksView {
            browser: self.hooks(offered),
            page: self.page,
        }
    }

    /// The hooks as the user left them.
    fn hooks(&self, offered: &Hooks) -> Hooks {
        let mut hooks = offered.clone();
        for hook in &mut hooks.hooks {
            if let Some(enabled) = self.enabled.get(&hook.key) {
                hook.enabled = *enabled;
            }
            if self.trusted.contains(&hook.key) {
                hook.trust = HookTrust::Trusted;
            }
        }
        hooks
    }

    /// Space or Enter on a hook (`toggle_selected_hook`).
    pub fn toggle(&mut self, offered: &Hooks, key: &str) -> Option<Edit> {
        let enabled = self.hooks(offered).toggle(key)?;
        self.enabled.insert(key.to_owned(), enabled);
        Some(Edit::HookEnabled {
            key: key.to_owned(),
            enabled,
        })
    }

    /// `t` on a hook (`trust_selected_hook`).
    pub fn trust(&mut self, offered: &Hooks, key: &str) -> Option<Edit> {
        let current_hash = self.hooks(offered).trust(key)?;
        self.trusted.insert(key.to_owned());
        Some(Edit::HookTrusted {
            key: key.to_owned(),
            current_hash,
        })
    }

    /// `t` on the events page (`trust_all_hooks`).
    pub fn trust_all(&mut self, offered: &Hooks) -> Option<Edit> {
        let updates = self.hooks(offered).trust_all();
        if updates.is_empty() {
            return None;
        }
        self.trusted
            .extend(updates.iter().map(|(key, _)| key.clone()));
        Some(Edit::HooksTrusted(updates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::fixtures::sandbox::Sandbox;
    use codex_tui_core::screens::settings::{Offer, Which};

    const STATUS_LINE: &str = include_str!("fixtures/status_line.jsonl");
    const TERMINAL_TITLE: &str = include_str!("fixtures/title.jsonl");
    const SKILLS: &str = include_str!("fixtures/skills_long_names.jsonl");
    const MEMORIES: &str = include_str!("fixtures/memories.jsonl");
    const FEATURES: &str = include_str!("fixtures/experimental_discovery.jsonl");
    const HOOKS_EVENTS: &str = include_str!("fixtures/hooks_events.jsonl");
    const HOOKS_REVIEW: &str = include_str!("fixtures/hooks_events_review_mixed.jsonl");

    fn offered(script: &str, which: Which) -> Offer {
        let sandbox = Sandbox::play(script).expect("plays");
        sandbox
            .app()
            .screens()
            .settings
            .offers
            .get(&which)
            .cloned()
            .expect("the dialog is offered")
    }

    fn setup(script: &str, which: Which) -> Setup {
        match offered(script, which) {
            Offer::Setup(setup) => setup,
            other => panic!("setup expected, got {other:?}"),
        }
    }

    fn features() -> Experimental {
        match offered(FEATURES, Which::Experimental) {
            Offer::Experimental(dialog) => dialog,
            other => panic!("features expected, got {other:?}"),
        }
    }

    fn hooks(script: &str) -> Hooks {
        match offered(script, Which::Hooks) {
            Offer::Hooks(hooks) => hooks,
            other => panic!("hooks expected, got {other:?}"),
        }
    }

    fn order(setup: &Setup) -> Vec<&str> {
        setup.items.iter().map(|item| item.id.as_str()).collect()
    }

    #[test]
    fn a_draft_nobody_touched_reads_as_what_is_offered() {
        let status = setup(STATUS_LINE, Which::StatusLine);
        assert_eq!(SetupDraft::default().view(&status), status);
        let dialog = features();
        assert_eq!(FeaturesDraft::new(&dialog).view(&dialog), dialog);
        let hooks = hooks(HOOKS_EVENTS);
        assert_eq!(HooksDraft::default().view(&hooks).browser, hooks);
    }

    #[test]
    fn toggling_an_item_changes_the_preview() {
        let status = setup(STATUS_LINE, Which::StatusLine);
        let mut draft = SetupDraft::default();
        assert_eq!(draft.toggle(&status, "model-with-reasoning"), None);
        assert_eq!(
            draft.view(&status).preview(),
            "gpt-5-codex · ~/codex-rs · jif/statusline-preview · gpt-5.2-codex medium"
        );
        assert_eq!(draft.toggle(&status, "unknown"), None);
    }

    #[test]
    fn moving_reorders_the_orderable_items_and_stops_at_the_fixed_row() {
        let status = setup(STATUS_LINE, Which::StatusLine);
        let mut draft = SetupDraft::default();
        // The fixed "use theme colors" row neither moves nor is crossed.
        draft.shift(&status, USE_THEME_COLORS, 1);
        assert_eq!(order(&draft.view(&status))[0], USE_THEME_COLORS);
        draft.shift(&status, "model", -1);
        assert_eq!(order(&draft.view(&status))[1], "model");
        draft.shift(&status, "model", 1);
        assert_eq!(
            &order(&draft.view(&status))[1..4],
            ["current-dir", "model", "git-branch"]
        );
        assert_eq!(
            draft.view(&status).preview(),
            "~/codex-rs · gpt-5-codex · jif/statusline-preview"
        );
    }

    #[test]
    fn enter_saves_the_enabled_items_in_order_and_the_color_preference() {
        for use_theme_colors in [false, true] {
            let script = STATUS_LINE.replace(
                "\"use_theme_colors\":true",
                &format!("\"use_theme_colors\":{use_theme_colors}"),
            );
            let status = setup(&script, Which::StatusLine);
            assert_eq!(
                SetupDraft::default().save(&status),
                Edit::SetupSaved {
                    kind: SetupKind::StatusLine,
                    items: vec!["model".into(), "current-dir".into(), "git-branch".into()],
                    use_theme_colors,
                }
            );
        }
        let status = setup(STATUS_LINE, Which::StatusLine);
        let mut draft = SetupDraft::default();
        draft.toggle(&status, "model");
        draft.shift(&status, "git-branch", -1);
        assert_eq!(
            draft.save(&status),
            Edit::SetupSaved {
                kind: SetupKind::StatusLine,
                items: vec!["git-branch".into(), "current-dir".into()],
                use_theme_colors: true,
            }
        );
    }

    #[test]
    fn escape_cancels_the_setup_and_reads_the_skills_again() {
        for (script, which, kind) in [
            (STATUS_LINE, Which::StatusLine, SetupKind::StatusLine),
            (TERMINAL_TITLE, Which::Title, SetupKind::Title),
        ] {
            assert_eq!(
                SetupDraft::cancel(&setup(script, which)),
                Edit::SetupCancelled(kind)
            );
        }
        assert_eq!(
            SetupDraft::cancel(&setup(SKILLS, Which::Skills)),
            Edit::SkillsClosed
        );
    }

    #[test]
    fn a_title_without_the_activity_joins_with_bars_and_without_the_prefix() {
        let title = setup(TERMINAL_TITLE, Which::Title);
        let mut draft = SetupDraft::default();
        draft.toggle(&title, "activity");
        assert_eq!(
            draft.view(&title).preview(),
            "my-project | Working | thread title"
        );
        for id in ["run-state", "thread-title", "project-name"] {
            draft.toggle(&title, id);
        }
        assert_eq!(draft.view(&title).preview(), "");
    }

    /// `skills_toggle_view.rs`: `toggle_selected` asks for each flip to be saved
    /// (`AppEvent::SetSkillEnabled`), the skills keep their order.
    #[test]
    fn each_skill_flip_is_a_request_and_the_skills_keep_their_order() {
        let skills = setup(SKILLS, Which::Skills);
        let id = "polish:superpowers-verification-before-completion";
        let mut draft = SetupDraft::default();
        assert_eq!(
            draft.toggle(&skills, id),
            Some(Edit::SkillToggled(id.to_owned(), true))
        );
        draft.shift(&skills, id, -1);
        let view = draft.view(&skills);
        assert!(view.items[1].enabled);
        assert_eq!(view.items[1].id, id);
        assert_eq!(
            draft.toggle(&skills, id),
            Some(Edit::SkillToggled(id.to_owned(), false))
        );
    }

    /// Space flips a switch (`toggle_selected`); Enter on one saves both switches as they
    /// stand. The confirmation takes neither toggles nor a save.
    #[test]
    fn memories_toggles_flip_and_enter_saves_both_switches() {
        let Offer::Memories(dialog) = offered(MEMORIES, Which::Memories) else {
            panic!("memories offered");
        };
        let mut draft = MemoriesDraft::new(&dialog, false);
        assert_eq!(
            draft.view(),
            Memories {
                use_memories: true,
                generate_memories: false,
            }
        );
        assert_eq!(draft.reset(), None);
        draft.toggle(2);
        draft.toggle(1);
        draft.toggle(0);
        assert_eq!(
            draft.save(),
            Some(Edit::MemoriesSaved {
                use_memories: false,
                generate_memories: true,
            })
        );

        let mut asking = MemoriesDraft::new(&dialog, true);
        asking.toggle(0);
        assert_eq!(asking.view(), dialog);
        assert_eq!(asking.save(), None);
        assert_eq!(asking.reset(), Some(Edit::MemoriesReset));
    }

    /// Enter sends what differs from the config and closes only when nothing does; Esc saves
    /// what changed (`save_changes`, `on_ctrl_c`).
    #[test]
    fn the_features_save_what_the_user_flipped() {
        let dialog = features();
        let idle = FeaturesDraft::new(&dialog);
        assert_eq!(idle.save(&dialog), Saving::Close);
        assert_eq!(idle.close(&dialog), None);

        let mut draft = FeaturesDraft::new(&dialog);
        for key in ["future_feature", "network_proxy", "prevent_idle_sleep"] {
            draft.toggle(&dialog, key);
        }
        let updates = vec![
            ("network_proxy".to_string(), false),
            ("future_feature".to_string(), false),
            ("prevent_idle_sleep".to_string(), false),
        ];
        assert_eq!(
            draft.save(&dialog),
            Saving::Write(Edit::FeaturesSave(updates.clone()))
        );
        assert_eq!(
            draft.close(&dialog),
            Some(Edit::FeaturesSave(updates.clone()))
        );
        let shown: Vec<(String, bool)> = draft
            .view(&dialog)
            .features
            .iter()
            .map(|feature| (feature.key.clone(), feature.enabled))
            .collect();
        assert_eq!(shown, updates);

        // a toggle while the write is on its way changes nothing, and Enter closes
        let mut saving = dialog.clone();
        saving.saving = true;
        draft.toggle(&saving, "network_proxy");
        assert!(!draft.view(&saving).features[0].enabled);
        assert_eq!(draft.save(&saving), Saving::Close);
        assert_eq!(draft.close(&saving), None);
    }

    #[test]
    fn a_failed_write_keeps_the_choices_and_enter_sends_them_again_while_esc_closes() {
        let mut dialog = features();
        let mut draft = FeaturesDraft::new(&dialog);
        draft.toggle(&dialog, "network_proxy");
        // the write was sent and refused: the keys stay unconfirmed
        dialog.unconfirmed = vec!["network_proxy".into()];
        assert!(!draft.view(&dialog).features[0].enabled);
        assert_eq!(draft.close(&dialog), None, "Esc only closes");
        // putting the value back still corrects a write that may have landed
        draft.toggle(&dialog, "network_proxy");
        assert_eq!(
            draft.save(&dialog),
            Saving::Write(Edit::FeaturesSave(vec![("network_proxy".into(), true)]))
        );
        // a dialog that comes with unconfirmed writes shows their values
        let reopened = FeaturesDraft::new(&dialog);
        assert!(!reopened.view(&dialog).features[0].enabled);
    }

    #[test]
    fn a_confirmed_write_clears_the_flips() {
        let mut dialog = features();
        let mut draft = FeaturesDraft::new(&dialog);
        draft.toggle(&dialog, "network_proxy");
        assert!(!draft.sync(&dialog));
        dialog.confirmed += 1;
        dialog.features[0].enabled = false;
        assert!(draft.sync(&dialog));
        assert_eq!(draft.view(&dialog), dialog);
        assert!(!draft.sync(&dialog));
    }

    #[test]
    fn a_read_only_feature_cannot_be_flipped() {
        let mut dialog = features();
        dialog.features[0].writable = false;
        let mut draft = FeaturesDraft::new(&dialog);
        draft.toggle(&dialog, "network_proxy");
        assert!(draft.view(&dialog).features[0].enabled);
        assert_eq!(draft.save(&dialog), Saving::Close);
    }

    /// A switch is a request (`SetHookEnabled`); managed hooks and those waiting for a review
    /// stay as they are (`toggle_selected_hook`), as an unknown key.
    #[test]
    fn hooks_toggle_flips_a_user_hook_and_leaves_the_others() {
        let offered = hooks(HOOKS_EVENTS);
        let enabled = |draft: &HooksDraft, key: &str| {
            draft
                .view(&offered)
                .hooks
                .iter()
                .find(|hook| hook.key == key)
                .map(|hook| hook.enabled)
        };
        let mut draft = HooksDraft::default();
        assert_eq!(
            draft.toggle(&offered, "path:user-config"),
            Some(Edit::HookEnabled {
                key: "path:user-config".into(),
                enabled: true
            })
        );
        assert_eq!(enabled(&draft, "path:user-config"), Some(true));
        assert_eq!(draft.toggle(&offered, "path:managed"), None);
        assert_eq!(draft.toggle(&offered, "unknown"), None);
        assert_eq!(enabled(&draft, "path:managed"), Some(true));
        draft.toggle(&offered, "path:user-config");
        assert_eq!(enabled(&draft, "path:user-config"), Some(false));

        let review = hooks(HOOKS_REVIEW);
        let mut draft = HooksDraft::default();
        assert_eq!(draft.toggle(&review, "path:untrusted"), None);
        assert_eq!(draft.toggle(&review, "path:modified"), None);
    }

    /// `t` trusts the hook waiting for a review and leaves its enablement; a trusted one
    /// records nothing (`trust_selected_hook`). On the events page it trusts them all in one
    /// request (`trust_all_hooks`).
    #[test]
    fn hooks_trust_vouches_for_one_hook_or_all_of_them() {
        let review = hooks(HOOKS_REVIEW);
        let mut draft = HooksDraft::default();
        assert_eq!(draft.trust(&review, "path:trusted"), None);
        assert_eq!(
            draft.trust(&review, "path:untrusted"),
            Some(Edit::HookTrusted {
                key: "path:untrusted".into(),
                current_hash: "sha256:current".into()
            })
        );
        let view = draft.view(&review);
        let untrusted = view
            .hooks
            .iter()
            .find(|hook| hook.key == "path:untrusted")
            .expect("the hook");
        assert_eq!(
            (untrusted.trust, untrusted.enabled),
            (HookTrust::Trusted, false)
        );
        assert_eq!(view.review_total(), 1);
        assert_eq!(
            draft.trust_all(&review),
            Some(Edit::HooksTrusted(vec![(
                "path:modified".into(),
                "sha256:current".into()
            )]))
        );
        assert_eq!(draft.view(&review).review_total(), 0);
        assert_eq!(draft.trust_all(&review), None);
    }

    /// The page is the draft's: none the events, an event its hooks.
    #[test]
    fn the_page_open_is_part_of_the_view() {
        let offered = hooks(HOOKS_EVENTS);
        let mut draft = HooksDraft::default();
        assert_eq!(draft.view(&offered).page, None);
        draft.page = Some(HookEvent::PreToolUse);
        assert_eq!(draft.view(&offered).page, Some(HookEvent::PreToolUse));
    }
}
