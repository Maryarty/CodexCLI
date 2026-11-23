//! `/auto-review`: the denials the auto-reviewer made in this thread, one to approve for a retry
//! (the oracle's `open_auto_review_denials_popup`).
//!
//! Flutter: a `showModalBottomSheet` over a searchable `ListView` with two columns (the action
//! and the rationale); rtwk's generic selection list is that sheet, so this file only says WHAT
//! the list holds: a disabled header row, then a row per denial, newest first. The pick comes
//! back as `AutoReview::Approve { id }`; the model turns it into `ThreadEv::ApproveDenied`.

use rtwk::{Cx, overlay::DialogResult};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::approvals::guardian::{Denial, Denials};
use crate::widgets::picker::list::{Columns, Item, List};

use super::guardian::summary;
use super::strings::{self, Strings};
use crate::sheets::select::Purpose;

/// The list of the denials: a header row ("Action", "Rationale") that cannot be picked and is
/// kept by every search, then a row per denial with its rationale in the second column.
pub fn menu(s: &Strings, denials: &Denials) -> List {
    let header = Item {
        name: s.denials_action.to_string(),
        description: Some(s.denials_rationale.to_string()),
        disabled: true,
        search_value: Some(String::new()),
        ..Item::default()
    };
    let rows = denials.iter().map(|denial| row(s, denial));
    List {
        title: Some(s.denials_title.to_string()),
        subtitle: Some(s.denials_subtitle.to_string()),
        footer_hint: Some(s.denials_hint.to_string()),
        items: std::iter::once(header).chain(rows).collect(),
        searchable: true,
        numbered: false,
        // `picker()`: stable columns, the second one placed from all rows.
        columns: Columns::AutoAllRows,
        ..List::default()
    }
}

fn row(s: &Strings, denial: &Denial) -> Item {
    let summary = summary(s, &denial.action);
    let rationale = denial
        .rationale
        .clone()
        .unwrap_or_else(|| s.denials_no_rationale.to_string());
    Item {
        id: denial.id.clone(),
        search_value: Some(format!("{summary} {rationale}")),
        name: summary,
        selected_description: Some(rationale.clone()),
        description: Some(rationale),
        ..Item::default()
    }
}

/// Shows the list over the page that asks. With no denial the oracle says so in the history
/// instead (`AutoReview::NoDenials`), so the caller asks only when there is one. The pick is the
/// verb `AutoReview::Approve { id }` for the thread.
pub fn open<S>(cx: &mut Cx<'_, S>, thread: ThreadId, denials: &Denials) -> DialogResult<()> {
    let list = menu(strings::of(cx), denials);
    crate::sheets::select::pick(cx, list, Purpose::ApproveDenial(thread))
}
