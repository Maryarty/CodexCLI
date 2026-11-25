//! What the pages share for their keys: the bottom menu (`footer`, rtwk's
//! `KeyHintsBar` styled to the oracle; its entries are the `hints` of the
//! page's `KeyBindings`, so every key shown is bound) and the popup a key
//! whose action is not built yet opens (`not_yet`), never a silent no-op.

use crate::widgets::measure::Measure;
use rtwk::LayoutBuilder;
use rtwk::controls::KeyHintsBar;
use rtwk::focus::{Focus, KeyCombo};
use rtwk::layout::{Column, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::overlay::{Dialog, DialogOptions, DialogResult, close_dialog_handler, show_dialog};
use rtwk::prelude::KeyCode;
use rtwk::style::Glyphs;
use rtwk::style::Style;
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use crate::keymap::bind_fixed;
use crate::l10n::strings as l10n;
use codex_tui_core::keymap::fixed;

/// The bottom menu of a page: `keys` (key as the oracle writes it, hint), the
/// same list the page binds with `KeyBindings`. The key text is the oracle's
/// (`esc`, `enter`, `↑/↓`, `←/→`, `pgup/pgdn`, `ctrl + t`, `⌃c`, a character): each is
/// turned into the keys rtwk's bar names (a family such as `↑/↓` into its two
/// keys, shown as one entry) and shown as written. A bar fed by the bindings
/// alone leaves out a plain character while a text field has the focus (it
/// would type), as the oracle's `? for shortcuts` is such a key; the pairs are
/// shown as given. The key in `key` style, its hint in `hint`, entries apart
/// by two blanks.
pub fn footer<K, H>(keys: impl IntoIterator<Item = (K, H)>, key: Style, hint: Style) -> KeyHintsBar
where
    K: AsRef<str>,
    H: Into<rtwk::text::Str>,
{
    footer_with(keys, key, hint, "  ")
}

/// `footer` with entries apart by `separator` (a page whose oracle row reads
/// `report · dashboard · help`).
pub fn footer_with<K, H>(
    keys: impl IntoIterator<Item = (K, H)>,
    key: Style,
    hint: Style,
    separator: &'static str,
) -> KeyHintsBar
where
    K: AsRef<str>,
    H: Into<rtwk::text::Str>,
{
    let mut shown: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut pairs: Vec<(String, rtwk::text::Str)> = Vec::new();
    for (display, hint_text) in keys {
        let hint_text: rtwk::text::Str = hint_text.into();
        for name in key_names(display.as_ref()) {
            if let Ok(combo) = KeyCombo::parse(&name) {
                shown.insert(combo.to_string(), display.as_ref().to_owned());
            }
            pairs.push((name, hint_text.clone()));
        }
    }
    KeyHintsBar::from_pairs(pairs)
        .key_label(move |combo| {
            // The text the page gave for the key; the oracle's look for the rest.
            shown
                .get(&combo.to_string())
                .cloned()
                .unwrap_or_else(|| oracle_label(combo))
        })
        .join_keys(false)
        .separator(separator)
        .key_style(key)
        .hint_style(hint)
}

/// One spelling of a footer row: its entries (key, hint; an empty hint shows
/// the key alone) and the narrowest width it is used from (the oracle's wide
/// labels only from 120 cells, whatever room is left).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub min_width: i32,
    pub entries: Vec<(String, String)>,
}

impl Variant {
    /// The cells the row takes: each key, a blank and its hint, the entries
    /// apart by `separator`.
    pub fn width(&self, seg: Measure, separator: &str) -> i32 {
        let cells = |text: &str| seg.width(text) as i32;
        let entries: i32 = self
            .entries
            .iter()
            .map(|(key, hint)| cells(key) + if hint.is_empty() { 0 } else { 1 + cells(hint) })
            .sum();
        entries + cells(separator) * (self.entries.len() as i32 - 1).max(0)
    }
}

/// A footer row that takes the first of `variants` (in the order of
/// priority: full, compact, minimal) that fits the width it is given, `pad`
/// cells in; an empty row when none does (the oracle's `hint_line_for_row`).
pub fn footer_fit(
    variants: Vec<Variant>,
    key: Style,
    hint: Style,
    separator: &'static str,
    pad: i32,
) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let room = constraints.max_width;
        match variants
            .iter()
            .find(|v| room >= v.min_width && v.width(seg, separator) + pad <= room)
        {
            Some(v) => Padding::new(
                EdgeInsets::only(pad, 0, 0, 0),
                footer_with(v.entries.clone(), key.clone(), hint.clone(), separator),
            )
            .into(),
            None => SizedBox::new(None, Some(1), None).into(),
        }
    })
    .into()
}

/// The key names rtwk reads for a key as the oracle writes it.
fn key_names(display: &str) -> Vec<String> {
    if display == "/" {
        return vec!["/".to_owned()];
    }
    display
        .split('/')
        .flat_map(|part| match part.split_once('\u{2013}') {
            // `1–6`: the keys 1 to 6.
            Some((from, to)) => match (from.parse::<u32>(), to.parse::<u32>()) {
                (Ok(from), Ok(to)) => (from..=to).map(|n| n.to_string()).collect(),
                _ => vec![part.to_owned()],
            },
            None => vec![part.to_owned()],
        })
        .map(|part| {
            let part = part.trim().replace(" + ", "+");
            // `⌃c`, `⇧?`, `⌥x`, `⌘k`: the modifier signs (what a configured binding is shown
            // with on macOS, `stroke_label`) and the key after them.
            let mut mods = String::new();
            let mut rest = part.as_str();
            while let Some(sign) = rest.chars().next() {
                let name = match sign {
                    '\u{2303}' => "ctrl+",
                    '\u{21e7}' => "shift+",
                    '\u{2325}' => "alt+",
                    '\u{2318}' => "super+",
                    _ => break,
                };
                mods.push_str(name);
                rest = &rest[sign.len_utf8()..];
            }
            let key = match rest {
                "\u{2191}" => "up",
                "\u{2193}" => "down",
                "\u{2190}" => "left",
                "\u{2192}" => "right",
                "esc" => "escape",
                "pgup" => "pageup",
                "pgdn" => "pagedown",
                _ => rest,
            };
            [mods.as_str(), key].concat()
        })
        .collect()
}

/// A key as the oracle writes it: the arrows as the pair `↑/↓`, `←/→`, Page
/// Up and Down as `pgup/pgdn` (the pair's second key has the same name, so the
/// bar lists the entry once), modifiers as `ctrl + t`.
fn oracle_label(combo: &KeyCombo) -> String {
    match combo.code {
        KeyCode::Up | KeyCode::Down => "\u{2191}/\u{2193}".to_owned(),
        KeyCode::Left | KeyCode::Right => "\u{2190}/\u{2192}".to_owned(),
        KeyCode::PageUp | KeyCode::PageDown => "pgup/pgdn".to_owned(),
        _ => KeyHintsBar::signs(combo, &Glyphs::UNICODE).replace('+', " + "),
    }
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
        let focus = Focus::new(Dialog::new(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .child(Text::new(w.text.clone())),
        ))
        .autofocus();
        bind_fixed(focus, fixed::ENTER, close_dialog_handler(cx, Some(()))).into()
    }
}
