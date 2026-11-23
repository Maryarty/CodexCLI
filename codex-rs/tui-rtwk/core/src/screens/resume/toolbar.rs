//! The picker's toolbar and list settings: the filter, status and sort
//! controls and which control has the keyboard focus.
//! Oracle: `resume_picker.rs` (`SessionFilterMode`, `SessionStatus`,
//! `ToolbarControl`).

/// Which directories' sessions the list shows (oracle: `SessionFilterMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    /// The sessions of the working directory.
    Cwd,
    /// The sessions of every checkout of the working directory's repository.
    Repo,
    /// Every session (a picker with no working directory shows all).
    #[default]
    All,
}

/// Which way a toolbar value moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Prev,
    Next,
}

/// A value picked by a tap on the toolbar: the control and the value itself
/// (Flutter's `SegmentedButton.onSelectionChanged`), where the arrow keys only
/// step to the neighbour (`Intent::Change`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Filter(Filter),
    Status(Status),
    Sort(Sort),
}

impl Pick {
    /// The control this value belongs to.
    pub fn control(self) -> Control {
        match self {
            Self::Filter(_) => Control::Filter,
            Self::Status(_) => Control::Status,
            Self::Sort(_) => Control::Sort,
        }
    }
}

impl Filter {
    /// The filter a launch starts with (oracle: `from_show_all`).
    pub fn initial(show_all: bool, cwd: Option<&str>) -> Self {
        if show_all || cwd.is_none() {
            Self::All
        } else {
            Self::Cwd
        }
    }

    /// The filter after a left or right arrow on its control (oracle:
    /// `SessionFilterMode::change`).
    pub fn change(self, dir: Dir, has_cwd: bool, repo_available: bool) -> Self {
        match (self, dir) {
            (Self::Cwd, Dir::Next) if repo_available => Self::Repo,
            (Self::Cwd, Dir::Next | Dir::Prev) | (Self::Repo, Dir::Next) => Self::All,
            (Self::Repo, Dir::Prev) => Self::Cwd,
            (Self::All, Dir::Prev) if repo_available => Self::Repo,
            (Self::All, _) if has_cwd => Self::Cwd,
            (Self::All, _) => Self::All,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Active,
    Archived,
}

impl Status {
    pub fn toggled(self) -> Self {
        match self {
            Self::Active => Self::Archived,
            Self::Archived => Self::Active,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Updated,
    Created,
}

impl Sort {
    pub fn toggled(self) -> Self {
        match self {
            Self::Updated => Self::Created,
            Self::Created => Self::Updated,
        }
    }
}

/// What Enter does with a row (oracle: `SessionPickerAction`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    Resume,
    Fork,
}

/// The control of the toolbar with the keyboard focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Control {
    #[default]
    Filter,
    Status,
    Sort,
}

impl Control {
    /// The controls Tab visits, in order: the status control exists only
    /// when resuming. A view holds which one has the focus as an index into
    /// this (a `TabController`), never as a `Control`.
    pub fn all(action: Action) -> &'static [Control] {
        match action {
            Action::Resume => &[Self::Filter, Self::Status, Self::Sort],
            Action::Fork => &[Self::Filter, Self::Sort],
        }
    }

    /// Tab (oracle: `ToolbarControl::next`): the status control exists only
    /// when resuming.
    pub fn next(self, action: Action) -> Self {
        match self {
            Self::Filter if action == Action::Resume => Self::Status,
            Self::Filter | Self::Status => Self::Sort,
            Self::Sort => Self::Filter,
        }
    }

    pub fn previous(self, action: Action) -> Self {
        match self {
            Self::Filter => Self::Sort,
            Self::Status => Self::Filter,
            Self::Sort if action == Action::Resume => Self::Status,
            Self::Sort => Self::Filter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_walks_cwd_repo_all_as_the_oracle_does() {
        assert_eq!(Filter::Cwd.change(Dir::Next, true, true), Filter::Repo);
        assert_eq!(Filter::Cwd.change(Dir::Next, true, false), Filter::All);
        assert_eq!(Filter::Repo.change(Dir::Prev, true, true), Filter::Cwd);
        assert_eq!(Filter::All.change(Dir::Prev, true, true), Filter::Repo);
        assert_eq!(Filter::All.change(Dir::Next, true, false), Filter::Cwd);
        assert_eq!(Filter::All.change(Dir::Next, false, false), Filter::All);
    }

    #[test]
    fn tab_skips_the_status_control_when_forking() {
        assert_eq!(Control::Filter.next(Action::Fork), Control::Sort);
        assert_eq!(Control::Sort.previous(Action::Fork), Control::Filter);
        assert_eq!(Control::Filter.next(Action::Resume), Control::Status);
    }
}
