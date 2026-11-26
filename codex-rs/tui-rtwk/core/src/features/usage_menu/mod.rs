//! `/usage` as a feature slice (ADR-012; oracle `chatwidget/slash_dispatch.rs` `SlashCommand::Usage`,
//! `chatwidget/usage.rs` `open_usage_menu`): a bare `/usage` opens the usage menu, a sheet over the
//! chat with `View analytics` and `Redeem reset`. The slice has no state of its own: what the menu
//! and the reset flow behind it keep is the usage screen's (`screens::usage::reset::State`: the
//! count of resets last learned, the credits of the list, the redemption by its idempotency key),
//! changed by `ScreenIntent::Usage` (`Menu`, `Resets`, `Redeem`, `Retry`); the sheet, its
//! picker and its confirmation are the UI half's (`src/features/usage_menu/`). `/usage daily`,
//! `weekly` and `cumulative` open the analytics page directly (`Target::Usage`), as the oracle
//! sends `OpenAnalytics` for them; `/usage` without a ChatGPT login is the notice.

use crate::commands::{
    CommandRule, DispatchCx, Need, Notice, Outcome, SlashCommand, Target, UsageKey, open,
};
use crate::screens::usage::TokenActivityView;

/// The marker type of the feature.
pub struct UsageMenu;

/// What `/usage` names (`FeatureTarget`): the feature's own sheet, opened by its UI half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet;

feature! {
    impl Feature for UsageMenu {
        const NAME: &'static str = "usage_menu";
        const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Usage, Sheet));
        const COMMAND_RULES: &'static [CommandRule] = &[
            CommandRule::new(SlashCommand::Usage)
                .requiring(Need::BackendAuth(|_| Notice::UsageNeedsLogin))
                .arguments(analytics),
        ];
        type Target = Sheet;
    }
}

fn analytics(command: SlashCommand, args: &str, _: &DispatchCx) -> Outcome {
    match TokenActivityView::parse(args) {
        Some(view) => open(Target::Usage, command, view.arg()),
        None => Outcome::Notice(Notice::Usage(UsageKey::Usage)),
    }
}
