//! Where the keys of the review are: the view, the focus, the cursor and the
//! highlighted action. That is the view's own state (a `Nav` the widget state
//! holds, never a field of the model: process.md A4.4); what a key means for
//! the flow comes back from `Nav::key` as a domain `Intent` the app applies.
//! Oracle: the key handling of `external_agent_config_migration/mod.rs`
//! (`handle_key`, `move_selection`, `confirm`).

use codex_tui_core::screens::import::{Intent, Model};

/// Which page of the review is up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Summary,
    Customize,
}

/// What has the keys: the checklist or the actions under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    Items,
    #[default]
    Actions,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    Proceed,
    Customize,
    Skip,
    Back,
}

/// A key of the review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    /// Enter: toggle the entry or run the action.
    Confirm,
    /// Space, in the checklist.
    Toggle,
    /// `c`, in the summary.
    Customize,
    /// `b`, in the checklist.
    Back,
    /// `a` and `n`, in the checklist.
    SelectAll,
    SelectNone,
    /// A digit: the action at that place.
    Number(u8),
    Escape,
}

/// The review's navigation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nav {
    pub view: View,
    pub focus: Focus,
    /// The entry the cursor is on (an index of the model's `items`, or of its
    /// `groups` in the summary).
    pub cursor: Option<usize>,
    pub action: Action,
}

impl Nav {
    /// The state a review opens with: the summary, the actions have the keys.
    pub fn open(model: &Model) -> Nav {
        let mut nav = Nav {
            view: View::Summary,
            focus: Focus::Actions,
            cursor: (!model.groups.is_empty()).then_some(0),
            action: Action::Proceed,
        };
        nav.settle(model);
        nav
    }

    /// The actions of the view, top to bottom.
    pub fn actions(&self, model: &Model) -> Vec<Action> {
        match self.view {
            View::Summary => {
                let mut actions = Vec::new();
                if model.proceed_enabled() {
                    actions.push(Action::Proceed);
                }
                actions.extend([Action::Customize, Action::Skip]);
                actions
            }
            View::Customize => vec![Action::Back],
        }
    }

    /// The highlighted action is one the view has (a toggle can take the
    /// first one away).
    pub fn settle(&mut self, model: &Model) {
        if !self.actions(model).contains(&self.action) {
            self.action = self.first_action(model);
        }
    }

    /// A key; the intent for the app when the key changes the flow.
    pub fn key(&mut self, model: &Model, key: Key) -> Option<Intent> {
        match key {
            Key::Up => self.up(model),
            Key::Down => self.down(model),
            Key::Confirm => return self.confirm(model),
            Key::Toggle => return self.toggle(),
            Key::Customize => {
                if self.view == View::Summary {
                    self.customize(model);
                }
            }
            Key::Back => {
                if self.view == View::Customize {
                    self.back(model);
                }
            }
            Key::SelectAll => return self.set_all(true),
            Key::SelectNone => return self.set_all(false),
            Key::Number(number) => return self.number(model, number),
            Key::Escape => match self.view {
                View::Summary => return Some(Intent::Cancel),
                View::Customize => self.back(model),
            },
        }
        None
    }

    fn first_action(&self, model: &Model) -> Action {
        self.actions(model).first().copied().unwrap_or(Action::Back)
    }

    fn last_action(&self, model: &Model) -> Action {
        self.actions(model).last().copied().unwrap_or(Action::Back)
    }

    fn before(&self, model: &Model) -> Option<Action> {
        let actions = self.actions(model);
        let at = actions.iter().position(|a| *a == self.action)?;
        actions.get(at.checked_sub(1)?).copied()
    }

    fn after(&self, model: &Model) -> Option<Action> {
        let actions = self.actions(model);
        let at = actions.iter().position(|a| *a == self.action)?;
        actions.get(at + 1).copied()
    }

    fn up(&mut self, model: &Model) {
        if self.view == View::Summary {
            self.focus = Focus::Actions;
            self.action = self
                .before(model)
                .unwrap_or_else(|| self.last_action(model));
            return;
        }
        match self.focus {
            Focus::Items => match self.cursor {
                Some(at) if at > 0 => self.cursor = Some(at - 1),
                _ => {
                    self.focus = Focus::Actions;
                    self.action = self.last_action(model);
                }
            },
            Focus::Actions => match self.before(model) {
                Some(action) => self.action = action,
                None => {
                    self.focus = Focus::Items;
                    if !model.items.is_empty() {
                        self.cursor = Some(model.items.len() - 1);
                    }
                }
            },
        }
    }

    fn down(&mut self, model: &Model) {
        if self.view == View::Summary {
            self.focus = Focus::Actions;
            self.action = self
                .after(model)
                .unwrap_or_else(|| self.first_action(model));
            return;
        }
        match self.focus {
            Focus::Items => match self.cursor {
                Some(at) if at + 1 < model.items.len() => self.cursor = Some(at + 1),
                _ => {
                    self.focus = Focus::Actions;
                    self.action = self.first_action(model);
                }
            },
            Focus::Actions => match self.after(model) {
                Some(action) => self.action = action,
                None => {
                    self.focus = Focus::Items;
                    if !model.items.is_empty() {
                        self.cursor = Some(0);
                    }
                }
            },
        }
    }

    fn toggle(&self) -> Option<Intent> {
        if self.view != View::Customize || self.focus != Focus::Items {
            return None;
        }
        self.cursor.map(Intent::Toggle)
    }

    fn set_all(&self, on: bool) -> Option<Intent> {
        (self.view == View::Customize).then_some(Intent::SetAll(on))
    }

    fn customize(&mut self, model: &Model) {
        self.view = View::Customize;
        self.cursor = (!model.items.is_empty()).then_some(0);
        self.focus = Focus::Items;
        self.action = Action::Back;
    }

    fn back(&mut self, model: &Model) {
        self.view = View::Summary;
        self.cursor = (!model.groups.is_empty()).then_some(0);
        self.focus = Focus::Actions;
        self.action = self.first_action(model);
    }

    fn confirm(&mut self, model: &Model) -> Option<Intent> {
        match self.focus {
            Focus::Items => self.toggle(),
            Focus::Actions => match self.action {
                Action::Proceed => Some(Intent::Proceed),
                Action::Customize => {
                    self.customize(model);
                    None
                }
                Action::Skip => Some(Intent::Cancel),
                Action::Back => {
                    self.back(model);
                    None
                }
            },
        }
    }

    fn number(&mut self, model: &Model, number: u8) -> Option<Intent> {
        let action = usize::from(number)
            .checked_sub(1)
            .and_then(|at| self.actions(model).get(at).copied())?;
        self.focus = Focus::Actions;
        self.action = action;
        self.confirm(model)
    }
}
