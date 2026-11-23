//! The keys the page names in its footer, its filter hint and its help: the
//! labels of what the user's keymap binds (oracle: `agent_center/hints.rs`,
//! `center_list_hint`, `center_filter_hint`, `center_help_lines`). A list key
//! a task action has taken is not shown for the list (the action owns the
//! key), and a key of the page's own (`?`, Tab) is not shown once a center
//! action has taken it.

use std::collections::HashSet;
use std::rc::Rc;

use codex_tui_core::keymap::actions::*;
use codex_tui_core::keymap::fixed::{AGENTS_FILTER_NEXT, AGENTS_FILTER_PREVIOUS};
use codex_tui_core::keymap::{ActionId, Binding, Context, Keymap};

use super::keys::taken;
use crate::keymap::{binding_label, fixed_name, label, stroke_for, stroke_label};

/// The labels one build of the page shows.
#[derive(Clone)]
pub struct Hints {
    keymap: Rc<Keymap>,
    taken: HashSet<String>,
}

impl Hints {
    pub fn new(keymap: Rc<Keymap>) -> Hints {
        Hints {
            taken: taken(&keymap),
            keymap,
        }
    }

    /// The label of a list action's first binding, none while a task action
    /// binds the same single key.
    pub fn list(&self, id: ActionId) -> Option<String> {
        let binding = *self.keymap.bindings(id).first()?;
        if let Binding::Single(stroke) = binding
            && self
                .keymap
                .actions()
                .filter(|(other, _)| other.context == Context::Agents)
                .any(|(other, _)| self.keymap.is_single(other, &stroke))
        {
            return None;
        }
        Some(binding_label(&binding))
    }

    /// The label of a list action's first binding, whatever else binds it.
    pub fn list_any(&self, id: ActionId) -> Option<String> {
        label(&self.keymap, id)
    }

    /// The label of a task action's first binding.
    pub fn task(&self, id: ActionId) -> Option<String> {
        label(&self.keymap, id)
    }

    /// Whether the footer offers `?` for the help: no center action took it.
    pub fn help_is_offered(&self) -> bool {
        !self.taken.contains("?")
    }

    /// The Tab and Shift+Tab hint, each key while no center action took it
    /// (`center_filter_hint`); empty when both are taken.
    pub fn filter(&self) -> String {
        [AGENTS_FILTER_NEXT, AGENTS_FILTER_PREVIOUS]
            .into_iter()
            .filter(|id| fixed_name(*id).is_some_and(|name| !self.taken.contains(&name)))
            .filter_map(stroke_for)
            .map(|stroke| stroke_label(&stroke))
            .collect::<Vec<_>>()
            .join("/")
    }

    /// The up and down keys, `up/down` (the move hint of the footer).
    pub fn movement(&self) -> String {
        [LIST_MOVE_UP, LIST_MOVE_DOWN]
            .into_iter()
            .filter_map(|id| self.list(id))
            .collect::<Vec<_>>()
            .join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::keymap::KeymapSpec;

    fn with(entries: &[(&str, &str, &[&str])]) -> Hints {
        let mut spec = KeymapSpec::default();
        for (table, action, keys) in entries {
            spec = spec.with(table, action, keys);
        }
        Hints::new(Rc::new(Keymap::resolve_unchecked(&spec)))
    }

    #[test]
    fn the_defaults_offer_help_filter_and_movement() {
        let hints = Hints::new(Rc::new(Keymap::defaults()));
        assert!(hints.help_is_offered());
        assert!(hints.filter().contains("tab"));
        assert_eq!(hints.list(LIST_ACCEPT).as_deref(), Some("enter"));
        assert!(hints.movement().contains('\u{2191}'));
    }

    #[test]
    fn a_task_action_taking_a_list_key_removes_its_hint() {
        let hints = with(&[("agents", "resume", &["enter"])]);
        assert_eq!(hints.list(LIST_ACCEPT), None);
        assert_eq!(hints.list_any(LIST_ACCEPT).as_deref(), Some("enter"));
    }

    #[test]
    fn a_center_action_taking_a_page_key_hides_its_hint() {
        let hints = with(&[
            ("agents", "resume", &["?"]),
            ("list", "move_down", &["tab"]),
        ]);
        assert!(!hints.help_is_offered());
        assert_eq!(
            hints.filter(),
            stroke_for(AGENTS_FILTER_PREVIOUS)
                .map(|stroke| stroke_label(&stroke))
                .unwrap_or_default()
        );
    }
}
