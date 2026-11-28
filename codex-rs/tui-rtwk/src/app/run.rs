//! The composition root: runs the app full screen, wiring the view (`ui`)
//! to the shell. This is `rtwk::run_app_with`'s body with the one
//! thing it lacks: `AppView::on_start`, which starts the shell as the root is
//! mounted (rtwk gap, the parity pack). The root is what `run_app_with`
//! makes: the overlay layer above the app, localizations and graphics
//! capabilities around it.

use rtwk::app::AppHost;
use rtwk::app::AppView;
use rtwk::app::HostOptions;
use rtwk::graphics::GraphicsScope;
use rtwk::l10n::TwkLocalizations;
use rtwk_core::app::App;
use rtwk_core::app::AppError;
use rtwk_core::app::AppOptions;
use rtwk_core::scroll::ScrollConfig;
use rtwk_core::style::Pen;
use rtwk_core::terminal::Terminal;
use rtwk_core::terminal::TerminalOptions;

use crate::app::UiApp;
use crate::app::view::view;
use crate::l10n::Lang;
use crate::services::shell;

/// The app as the tests host it: the view with the shell's `on_start`.
pub fn app_view(app: UiApp) -> AppView<UiApp> {
    AppView::new(app, view).on_start(shell::start)
}

/// The app under the overlay layer, with the app's theme at the root: what
/// the dialogs and routes below it read their tokens from.
pub fn root(view: AppView<UiApp>) -> rtwk::overlay::App {
    rtwk::overlay::App::new(view).theme(crate::app::theme::data())
}

/// `root` under rtwk's localizations for `lang`: what the app's own tables
/// (`l10n::lang`) and rtwk's built-in widget strings both read.
pub fn localized(root: impl Into<rtwk::AnyWidget>, lang: Lang) -> TwkLocalizations {
    TwkLocalizations::new(lang.widgets(), root)
}

/// Opens the terminal and runs `app` until it quits.
pub fn run(app: UiApp) -> Result<(), AppError> {
    run_on(app, true)
}

/// As `run`; without `alternate_screen` the app is drawn inline, keeping the
/// terminal's scrollback (`--no-alt-screen`).
pub fn run_on(app: UiApp, alternate_screen: bool) -> Result<(), AppError> {
    let terminal = Terminal::open(TerminalOptions {
        alternate_screen,
        normal_screen_mouse: false,
        ..TerminalOptions::default()
    })?;
    let caps = terminal.info().caps.graphics;
    let app = app.with_color_mode(terminal.info().color_mode);
    let mut frame_loop = App::with_options(
        terminal,
        AppOptions {
            scroll: Some(ScrollConfig::default()),
            ..AppOptions::default()
        },
    )?;
    frame_loop.styles_mut().pen(&Pen::new());
    let lang = app.ui.lang;
    let root = root(app_view(app).with_handle(frame_loop.handle()));
    let localized = localized(root, lang);
    AppHost::of_widget(
        &mut frame_loop,
        GraphicsScope::new(caps, localized),
        HostOptions::default(),
    )
    .run(&mut frame_loop)
}
