//! Styles of the skeleton, taken from the rtwk `ThemeData` of the root
//! (`ui::run::root`), never from a literal colour: a theme of the app changes
//! the look in this one place.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::style::Style;
use rtwk::style::Theme;
use rtwk::style::ThemeData;
use rtwk_core::input::Brightness;
use rtwk_core::style::Attrs;

/// The theme of the app's root: the terminal's own colours, dark until the
/// terminal says otherwise.
pub fn data() -> Rc<ThemeData> {
    Rc::new(ThemeData::terminal(Brightness::Dark))
}

/// The style tokens of one theme.
#[derive(Clone, Debug)]
pub struct Tokens(Rc<ThemeData>);

impl Tokens {
    pub fn new(data: Rc<ThemeData>) -> Self {
        Self(data)
    }

    /// The tokens of the theme `cx` is under.
    pub fn of<S>(cx: &mut Cx<'_, S>) -> Self {
        Self(Theme::of(cx))
    }

    pub fn dim(&self) -> Style {
        self.0.dim()
    }

    pub fn bold(&self) -> Style {
        self.0.text.clone().attrs(Attrs::BOLD)
    }

    pub fn accent(&self) -> Style {
        self.0.accent.clone()
    }

    pub fn selected(&self) -> Style {
        self.0.accent.clone().attrs(Attrs::BOLD)
    }

    pub fn error(&self) -> Style {
        self.0.error.clone()
    }
}
