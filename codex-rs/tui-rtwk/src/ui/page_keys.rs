//! What the pages share for their keys: the bottom menu (`footer`, rtwk's
//! `KeyHintsBar` styled to the oracle; its entries are the `hints` of the
//! page's `KeyBindings`, so every key shown is bound) and the popup a key
//! whose action is not built yet opens (`not_yet`), never a silent no-op.

use rtwk::controls::KeyHintsBar;
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, MainAxisSize};
use rtwk::overlay::{Dialog, DialogOptions, DialogResult, close_dialog_handler, show_dialog};
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use crate::l10n::strings as l10n;

/// The bottom menu of a page: `keys` (key, hint), the same list the page
/// binds with `KeyBindings`. A bar fed by the bindings alone leaves out a
/// plain character while a text field has the focus (it would type), as the
/// oracle's `? for shortcuts` is such a key; the pairs are shown as given.
/// The key in `key` style, its hint in `hint`, entries apart by two blanks.
pub fn footer<K, H>(keys: impl IntoIterator<Item = (K, H)>, key: Style, hint: Style) -> KeyHintsBar
where
    K: Into<rtwk::text::Str>,
    H: Into<rtwk::text::Str>,
{
    KeyHintsBar::from_pairs(keys)
        .join_keys(false)
        .separator("  ")
        .key_style(key)
        .hint_style(hint)
}

/// Shows the popup "Not implemented yet: <action>" centred over the page;
/// Esc and Enter close it. `action` is the name of what the key does, from
/// the string tables.
pub fn not_yet<S>(cx: &mut Cx<'_, S>, action: &'static str) -> DialogResult<()> {
    let text = l10n::of(cx).not_yet(action).to_string();
    show_dialog(
        cx,
        move |_| NotYet { text: text.clone() }.into(),
        DialogOptions::default(),
    )
}

/// The popup's body.
#[derive(Clone, Debug, PartialEq)]
pub struct NotYet {
    pub text: String,
}

pub struct NotYetState;

impl StatefulWidget for NotYet {
    type State = NotYetState;

    fn create_state(&self) -> NotYetState {
        NotYetState
    }
}

impl_widget!(stateful eq NotYet);

impl State for NotYetState {
    type Widget = NotYet;
    type Msg = ();

    fn build(&self, w: &NotYet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // Esc closes the dialog itself (rtwk); Enter is bound here.
        Focus::new(Dialog::new(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .child(Text::new(w.text.clone())),
        ))
        .autofocus()
        .bind(KeyPress::ENTER, close_dialog_handler(cx, Some(())))
        .into()
    }
}
