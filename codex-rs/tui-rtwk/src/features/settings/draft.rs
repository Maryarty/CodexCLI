//! What the settings dialogs are before they are saved (ADR-013): the checks a user made, the
//! order they gave, the page they went into. In Flutter it is the `State` of the sheet (a
//! `Set<String> checked`, a `List<String> order` and a `pageIndex`), initialised from what the
//! dialog is offered (`widget.items`) and read together with it: the model keeps the catalogue
//! as the server and the host gave it, and learns of the draft only through the request a
//! dialog makes (`Edit`). Each draft reads as the dialog's model type (`view`), so a sheet
//! draws the same rows whether the user changed them or not.

use std::collections::{BTreeMap, BTreeSet};

use codex_tui_core::features::settings::hooks::{HookEvent, HookTrust, Hooks};
use codex_tui_core::features::settings::{
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
