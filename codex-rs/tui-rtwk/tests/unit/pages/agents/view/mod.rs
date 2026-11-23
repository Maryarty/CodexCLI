use codex_tui_rtwk::pages::agents::view::list::ListInputs;
use codex_tui_rtwk::pages::agents::view::*;
use std::rc::Rc;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::*;
use rtwk::AnyWidget;
use rtwk::BuildCx;
use rtwk::Handler;
use rtwk::LayoutBuilder;
use rtwk::StatelessWidget;
use rtwk::controls::SelectionController;
use rtwk::editing::TextEditingController;
use rtwk::editing::TextField;
use rtwk::focus::ExcludeFocus;
use rtwk::focus::Focus;
use rtwk::focus::FocusNode;
use rtwk::focus::KeyBindings;
use rtwk::foundation::ValueListenableBuilder;
use rtwk::foundation::ValueNotifier;
use rtwk::gestures::IgnorePointer;
use rtwk::gestures::PointerEvent;
use rtwk::impl_widget;
use rtwk::l10n::NumSyms;
use rtwk::layout::BorderType;
use rtwk::layout::BoxWidget;
use rtwk::layout::Center as Middle;
use rtwk::layout::Column;
use rtwk::layout::CrossAxisAlignment;
use rtwk::layout::Divider;
use rtwk::layout::EdgeInsets;
use rtwk::layout::Expanded;
use rtwk::layout::MainAxisSize;
use rtwk::layout::Offstage;
use rtwk::layout::Padding;
use rtwk::layout::Row;
use rtwk::layout::SizedBox;
use rtwk::layout::VerticalDivider;
use rtwk::overlay::Dialog;
use rtwk::scroll::ListController;
use rtwk::style::Attrs;
use rtwk::style::Span;
use rtwk::style::Style;
use rtwk::text::Text;
use rtwk::text::TextAlign;
use rtwk::text::TextWrap;
use rtwk::text::segmenter_of;
use codex_tui_core::screens::Shared;
use codex_tui_core::screens::agents::Action;
use codex_tui_core::screens::agents::Center;
use codex_tui_core::screens::agents::CenterRow;
use codex_tui_core::screens::agents::DetailLine;
use codex_tui_core::screens::agents::Failure;
use codex_tui_core::screens::agents::Group;
use codex_tui_core::screens::agents::GroupLabel;
use codex_tui_core::screens::agents::Grouping;
use codex_tui_core::screens::agents::Model;
use codex_tui_core::screens::agents::Notice;
use codex_tui_core::screens::agents::RowData;
use codex_tui_core::screens::agents::Sel;
use codex_tui_core::screens::agents::display_directory;
use codex_tui_rtwk::pages::agents::strings::EN;
use rtwk::style::ThemeData;

fn inputs(center: Center, agents: &Shared<Model>) -> ListInputs {
    ListInputs {
        look: Look::new(&ThemeData::NORD, false),
        strings: &EN,
        agents: agents.clone(),
        center: Rc::new(center),
    }
}

fn center(entries: Vec<CenterRow>, show_more_selected: bool) -> Center {
    Center {
        rows: Vec::new(),
        entries,
        tab_counts: [0; 5],
        selected: None,
        show_more_selected,
        more: false,
    }
}

/// S5 (vs1-review): a cursor that moves changes the center's
/// `show_more_selected` and `selected`, not what the rows are made of,
/// so the list compares equal and is not built again; a changed entry
/// or model does build it.
#[test]
fn the_list_inputs_ignore_the_cursor() {
    let agents: Shared<Model> = Shared::default();
    let entries = || vec![CenterRow::Gap, CenterRow::ShowMore];
    let base = inputs(center(entries(), false), &agents);
    assert!(
        base == inputs(center(entries(), true), &agents),
        "cursor on `Show more`"
    );
    assert!(
        base != inputs(center(vec![CenterRow::Gap], false), &agents),
        "other rows"
    );
    let mut later = agents;
    later.now += 1;
    assert!(
        base != inputs(center(entries(), false), &later),
        "other model"
    );
}
