//! The welcome header above the sign-in and provider steps. Oracle:
//! `onboarding/welcome.rs`: the logo on a fixed 60 by 21 stage (the session's
//! blossom, its `AnimationController`), a blank row, the welcome line and
//! the blank row every step ends with. The logo shows only with animations
//! on and a screen of at least 64 by 41 cells.

use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::Text;

use crate::pages::session::blossom;

use super::strings::Strings;

const MIN_ANIMATION_HEIGHT: i32 = 41;
const MIN_ANIMATION_WIDTH: i32 = 64;
const ANIMATION_WIDTH: i32 = 60;
const ANIMATION_HEIGHT: i32 = 21;

/// Whether the logo shows on a screen of `width` by `height` cells.
pub fn shows_logo(animations: bool, width: i32, height: i32) -> bool {
    animations && height >= MIN_ANIMATION_HEIGHT && width >= MIN_ANIMATION_WIDTH
}

/// The header; `replay` changes to play the logo again; `faded` is a text
/// entry being open (`Presentation::Faded`: the logo stops and fades).
pub fn header(
    strings: &'static Strings,
    bold: Style,
    logo: bool,
    replay: u32,
    faded: bool,
) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    if logo {
        column = column
            .child(blossom::stage(
                ANIMATION_WIDTH,
                ANIMATION_HEIGHT,
                replay,
                faded,
            ))
            .child(SizedBox::new(None, Some(1), None));
    }
    let [before, name, after] = strings.welcome;
    column
        .child(Text::rich(vec![
            Span::new(before),
            Span::styled(name, bold),
            Span::new(after),
        ]))
        .child(SizedBox::new(None, Some(1), None))
        .into()
}
