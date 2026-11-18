//! Localization (ADR-010). The app's strings are typed `const` tables, one
//! per screen directory (`src/screens/<dir>/strings.rs`) and the shared one
//! here (`strings.rs`): plain strings are `&'static str` fields, messages
//! with arguments are `Msg<N>` fields behind typed methods, plural
//! categories come from rtwk-l10n's rules. A table is read by reference,
//! with no lookup by key and no allocation. English is the oracle's text
//! byte for byte; a locale is one more `const` per table, and a table with
//! none falls back to English, so a screen is never edited to add a locale.
//!
//! The locale is negotiated once at start (`Cli::lang`: `--locale`, else
//! `LC_ALL`, `LC_MESSAGES`, `LANG`, `LANGUAGE`; English as the fallback) and
//! reaches the tree as rtwk's own `TwkLocalizations` (`ui::run::localized`),
//! which is also what makes rtwk's built-in widget strings follow it. A
//! screen asks the tree: `strings::of(cx)`.

pub mod strings;

use rtwk::l10n::DependCx;
use rtwk::l10n::Locale;
use rtwk::l10n::WidgetStrings;
use rtwk::l10n::requested;
use rtwk::l10n::requested_with;
use rtwk::l10n::tables;

/// The languages the app has tables for. A new one is a variant here and a
/// field in `Tables`; no screen changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

impl Lang {
    /// The locale this language's tables are for.
    pub fn locale(self) -> Locale {
        Locale::new(match self {
            Lang::En => "en",
            Lang::Ru => "ru",
        })
    }

    /// The first of `requested` (most wanted first) that a table serves,
    /// else English.
    pub fn negotiate(requested: &[Locale]) -> Lang {
        requested
            .iter()
            .find_map(|wanted| {
                [Lang::En, Lang::Ru]
                    .into_iter()
                    .find(|lang| lang.locale().serves(wanted).is_some())
            })
            .unwrap_or_default()
    }

    /// `negotiate` over the environment as `get` reads it.
    pub fn from_env_with(get: impl Fn(&str) -> Option<String>) -> Lang {
        Lang::negotiate(&requested_with(get))
    }

    /// `negotiate` over the process environment.
    pub fn from_env() -> Lang {
        Lang::negotiate(&requested())
    }

    /// rtwk's own table for this language (its built-in widget strings).
    pub fn widgets(self) -> &'static WidgetStrings {
        tables::negotiate(&[self.locale()])
    }
}

/// The language of the tree `cx` is in, from rtwk's localizations at the
/// root; depended on, so a switch rebuilds the screen.
pub fn lang(cx: &mut impl DependCx) -> Lang {
    Lang::negotiate(&[rtwk::l10n::locale(cx)])
}

/// A screen's tables, one per language. Only English is required; a
/// language without a table shows English.
#[derive(Debug)]
pub struct Tables<T: 'static> {
    en: &'static T,
    ru: Option<&'static T>,
}

impl<T> Tables<T> {
    /// A screen with its English table (the oracle's text).
    pub const fn english(en: &'static T) -> Self {
        Tables { en, ru: None }
    }

    /// With a Russian table.
    #[must_use]
    pub const fn with_ru(mut self, ru: &'static T) -> Self {
        self.ru = Some(ru);
        self
    }

    /// The table for `lang`; English where there is none.
    pub const fn pick(&self, lang: Lang) -> &'static T {
        match lang {
            Lang::En => self.en,
            Lang::Ru => match self.ru {
                Some(table) => table,
                None => self.en,
            },
        }
    }
}
