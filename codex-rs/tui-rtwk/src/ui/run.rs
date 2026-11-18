//! Runs the app full screen. This is `rtwk::run_app_with`'s body with the one
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

use crate::app::CodexApp;
use crate::shell;
use crate::ui::view::view;

/// The app as the tests host it: the view with the shell's `on_start`.
pub fn app_view(app: CodexApp) -> AppView<CodexApp> {
    AppView::new(app, view).on_start(shell::start)
}

/// The app under the overlay layer, with the app's theme at the root: what
/// the dialogs and routes below it read their tokens from.
pub fn root(view: AppView<CodexApp>) -> rtwk::overlay::App {
    rtwk::overlay::App::new(view).theme(crate::theme::data())
}

/// Opens the terminal and runs `app` until it quits.
pub fn run(app: CodexApp) -> Result<(), AppError> {
    let terminal = Terminal::open(TerminalOptions {
        alternate_screen: true,
        normal_screen_mouse: false,
        ..TerminalOptions::default()
    })?;
    let caps = terminal.info().caps.graphics;
    let mut frame_loop = App::with_options(
        terminal,
        AppOptions {
            scroll: Some(ScrollConfig::default()),
            ..AppOptions::default()
        },
    )?;
    frame_loop.styles_mut().pen(&Pen::new());
    let root = root(app_view(app).with_handle(frame_loop.handle()));
    let localized = TwkLocalizations::from_env(root);
    AppHost::of_widget(
        &mut frame_loop,
        GraphicsScope::new(caps, localized),
        HostOptions::default(),
    )
    .run(&mut frame_loop)
}
