//! The titles of the full-screen read of a request (oracle `app/event_dispatch.rs`,
//! `FullScreenApprovalRequest` and `FullScreenUserVerificationRequest`): the oracle spaces the
//! letters of a pager's title, one table per locale, read through `of(cx)`.

use rtwk::l10n::DependCx;

use crate::l10n::Tables;
use crate::l10n::lang;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    pub exec: &'static str,
    pub permissions: &'static str,
    pub elicitation: &'static str,
    pub verification: &'static str,
}

pub const EN: Strings = Strings {
    exec: "E X E C",
    permissions: "P E R M I S S I O N S",
    elicitation: "E L I C I T A T I O N",
    verification: "U S E R  V E R I F I C A T I O N",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
