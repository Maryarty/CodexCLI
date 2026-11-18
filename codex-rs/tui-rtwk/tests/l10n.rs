//! Localization (ADR-010): the locale is negotiated once, the app's strings
//! are typed tables read by reference, rtwk's own widget strings follow the
//! same locale. English is the oracle's text; the Russian table of
//! `src/l10n` is a three-string proof that the path works.

#![allow(clippy::expect_used)]

use codex_tui_rtwk::Cli;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::l10n::{Lang, Tables};
use codex_tui_rtwk::run::{app_view, localized, root};
use codex_tui_rtwk::shell::Backend;
use rtwk::l10n::Locale;

use crate::common::start_lang;

fn want(tags: &[&str]) -> Vec<Locale> {
    tags.iter().map(|tag| tag.parse().expect("a tag")).collect()
}

#[test]
fn the_locale_is_negotiated_from_what_the_user_asked_for() {
    assert_eq!(Lang::negotiate(&[]), Lang::En);
    assert_eq!(Lang::negotiate(&want(&["ru_RU.UTF-8"])), Lang::Ru);
    assert_eq!(Lang::negotiate(&want(&["de", "ru-RU", "en"])), Lang::Ru);
    assert_eq!(Lang::negotiate(&want(&["de"])), Lang::En, "fallback en");
}

#[test]
fn the_environment_is_read_as_lc_all_lc_messages_lang() {
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |key: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    };
    assert_eq!(
        Lang::from_env_with(env(&[("LANG", "ru_RU.UTF-8")])),
        Lang::Ru
    );
    assert_eq!(
        Lang::from_env_with(env(&[("LC_ALL", "en_US.UTF-8"), ("LANG", "ru_RU.UTF-8")])),
        Lang::En
    );
    assert_eq!(Lang::from_env_with(env(&[("LANG", "C")])), Lang::En);
    assert_eq!(Lang::from_env_with(env(&[])), Lang::En);
}

#[test]
fn the_locale_flag_overrides_the_environment() {
    let cli = Cli::parse(["--locale".to_string(), "ru".to_string()]).expect("parsed");
    assert_eq!(cli.lang(&want(&["en-US"])), Lang::Ru);
    let cli = Cli::parse(["--locale=en".to_string()]).expect("parsed");
    assert_eq!(cli.lang(&want(&["ru"])), Lang::En);
    let cli = Cli::parse([]).expect("parsed");
    assert_eq!(
        cli.lang(&want(&["ru"])),
        Lang::Ru,
        "no flag: the environment"
    );
}

#[test]
fn a_bad_locale_flag_is_an_error() {
    assert!(Cli::parse(["--locale".to_string()]).is_err(), "no value");
    assert!(
        Cli::parse(["--locale=!!".to_string()]).is_err(),
        "not a tag"
    );
}

#[test]
fn a_table_without_a_locale_falls_back_to_english() {
    static EN: u8 = 1;
    static RU: u8 = 2;
    let only_english = Tables::english(&EN);
    assert_eq!(*only_english.pick(Lang::Ru), 1);
    let both = Tables::english(&EN).with_ru(&RU);
    assert_eq!(*both.pick(Lang::Ru), 2);
    assert_eq!(*both.pick(Lang::En), 1);
}

#[test]
fn rtwks_own_widget_strings_follow_the_locale() {
    for (lang, tag) in [(Lang::En, "en"), (Lang::Ru, "ru")] {
        let wrapped = localized(root(app_view(UiApp::new(Backend::None))), lang);
        assert_eq!(wrapped.strings().locale.to_string(), tag);
    }
}

#[test]
fn the_skeleton_shows_the_russian_strings_under_locale_ru() {
    let mut h = start_lang(Lang::Ru);
    h.wait_for("the ready word", |s| s.contains("готово"));
    let screen = h.t.screen();
    assert!(screen.contains("Спросите Codex"), "{screen}");
    assert!(!screen.contains("Ask Codex to do anything"), "{screen}");
}

#[test]
fn a_string_without_a_russian_entry_stays_english() {
    let mut h = start_lang(Lang::Ru);
    h.wait_for("ready", |s| s.contains("готово"));
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        codex_tui_core::app::AppEv::Backend(
            codex_tui_core::fixtures::builders::tool_input_request(
                codex_tui_rtwk::shell::fake::FAKE_THREAD,
                "turn-1",
                7,
            ),
        ),
    );
    h.wait_for("the dialog", |s| s.contains("needs an answer"));
    assert!(h.t.screen().contains("Dismiss"), "{}", h.t.screen());
}
