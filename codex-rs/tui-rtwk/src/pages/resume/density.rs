//! How much of a saved thread the list shows (oracle: `SessionListDensity`):
//! two lines and a blank row each, or one line. Ctrl+O switches it; it is the
//! page's state, nothing the backend needs.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    #[default]
    Comfortable,
    Dense,
}

impl Density {
    pub fn toggled(self) -> Self {
        match self {
            Self::Comfortable => Self::Dense,
            Self::Dense => Self::Comfortable,
        }
    }
}
