<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Words in another language

rtwk writes a few words of its own: the `ETA` of a progress list, `Y/n` after a
question, `answer y or n` when the answer was neither, the names of the months
of a calendar, the labels of a pagination. They are one plain struct,
`WidgetStrings`, with a table per locale: English, Russian, Japanese and Arabic
ship (the last three are a test set of the plural rules, the scripts and the
digits; nobody has reviewed their wording). There is no localization package, no
generator and no global to set. Your translations, from whatever holds them,
become a `&'static WidgetStrings` that you hand over at the root.

<!-- not rtwk: OnceLock -->

## What an app gets without doing anything

`run_app_view` reads the locale the environment asks for (`LC_ALL`,
`LC_MESSAGES`, `LANG`, `LANGUAGE`; English under `C`), picks the shipped table
that serves it and places it at the root. the `locale` of `RunOptions` says another
one. A language no shipped table has falls back to English.

## A table of your own

A table is a base with some fields replaced. `WidgetStrings::builder(base)`
copies the base, `set` assigns the fields (typed: a misspelt field or a message
of the wrong arity does not compile), and `build` gives the table. A plain
string is a field; a message with a number or a time in it is a function of that
value (`msg!("noch {0}")`), and one that has plural forms is a `plural!`, so a
language puts the value where it belongs and picks the form its rule says.

<!-- file: crates/rtwk/examples/guides/l10n.rs -->
```rust
use std::process::ExitCode;
use std::sync::OnceLock;

use rtwk::app::ViewCx;
use rtwk::cli::{Cli, RunError, RunOptions};
use rtwk::controls::ConfirmInput;
use rtwk::l10n::{Locale, TwkLocalizations, WidgetStrings, msg, plural, tables};
use rtwk::prelude::*;

/// The words rtwk writes itself, in German: made once, a table lives as long as the program.
pub fn german() -> &'static WidgetStrings {
    static GERMAN: OnceLock<&'static WidgetStrings> = OnceLock::new();
    GERMAN.get_or_init(|| {
        WidgetStrings::builder(tables::english())
            .locale("de".parse::<Locale>().expect("a language tag"))
            .set(|s| {
                s.prompts.yes = "ja";
                s.prompts.no = "nein";
                s.prompts.default_yes = "J/n";
                s.prompts.default_no = "j/N";
                s.confirm.yes = "Ja";
                s.confirm.no = "Nein";
                // a message with a value is a function of it, so a language puts it where it belongs
                s.labels.eta = msg!("noch {0}");
                s.prompts.choose_at_least = plural! { other: "mindestens {0}" };
            })
            .decimal_separator(",")
            .build()
    })
}

pub fn main() -> ExitCode {
    run_app((), view)
}

/// An app: the words under `TwkLocalizations` are the table's.
pub fn view(_: &(), cx: &mut ViewCx<()>) -> AnyWidget {
    let done = cx.on_with(|_: &mut (), _: bool, cx| cx.quit());
    TwkLocalizations::new(
        german(),
        ConfirmInput::new("Weiter?")
            .on_confirmed(done)
            .autofocus(true),
    )
    .into()
}

/// A console has no widget tree, so its options take the table itself.
pub fn console_options() -> RunOptions {
    RunOptions {
        strings: Some(german()),
        ..RunOptions::default()
    }
}

/// Asks in the table's words, on a pipe and at a terminal.
pub fn ask(options: RunOptions) -> Result<bool, RunError> {
    Cli::run(options, |cli| {
        cli.confirm("Weiter?").default_value(false).ask()
    })
}
```

`build` leaks the table once, so that it lives as long as the program and has
an address of its own: widgets tell a change of locale by that address. Build it
once at the start (a `static` `OnceLock`, as above), never in a `build` method or
a loop: a table made there is a new one, and every widget that reads it is
built again each time, and the old one stays in memory.

## An app

`TwkLocalizations::new(table, child)` at the top of the app, or at the top of a
part of it. Widgets read the words with `rtwk::l10n::strings(cx)` (a component's
`Cx` or a view's `ViewCx`); showing a `TwkLocalizations` of another table builds
again the widgets that read it, and no others. `ConfirmInput` above reads its words
("Ja" and "Nein" there) from its table, unless its `yes` and `no` say otherwise.

## A console

A `Cli` has no widget tree, so it takes the table itself: `Cli::with_strings`,
or `strings` in the `cli::RunOptions` of `Cli::run` and `Cli::run_with_args`.
What it shows or asks in its inline session is wrapped in `TwkLocalizations`
for you, as it is in your theme. `console_options` and `ask` above are that.

On a plain terminal a yes or no is read as `y`, `yes`, `n` and `no` whatever the
language. The interactive prompt reads the table's `yes` and `no` and their first
letters too; the plain one does not yet, so a German `ja` typed to it is asked
again.

## From a file of translations

The table is plain Rust, so the glue between a file of translations and a
`WidgetStrings` is a function that fills the fields from it, run once at the
start inside `set`. rtwk reads no translation file itself: loading a table at
run time is deferred (the design). The number and time
formats follow the table too (`decimal_separator(",")` writes `3,1 s`), and the
plural rule and the digits of a locale are part of it.

An app puts `TwkLocalizations` at its root with a table it made once for the
language, and shows another when the language changes.
