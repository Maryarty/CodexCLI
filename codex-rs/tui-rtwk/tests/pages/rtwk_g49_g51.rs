// test root: pages
//! Probes of rtwk gaps G49, G50 and G51 (the three routes of process.md §5,
//! one probe each). Every assertion pins what rtwk does today and the oracle's
//! row it differs from: when rtwk changes, the probe fails and the row closes.
//!
//! - G49: `TextWrap::Word` never breaks after `/` (the oracle's
//!   `cwd_picker_selected_40x16` breaks `(/` | `Users/example/current)`).
//! - G50: `Column` gives the whole-cell remainder of a short viewport to the
//!   FIRST flexible children (`rtwk-render/src/objects/flex.rs:1040-1052`), the
//!   oracle's `FlexRenderable::allocate` (`render/renderable.rs:296-360`) to
//!   the LAST one.
//! - G51: the same remainder rule, in the trust step's shape
//!   (`onboarding/trust_directory.rs:153-210`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use rtwk::layout::{
    Column, CrossAxisAlignment, FlexFit, Flexible, Grid, GridPlacement, GridTrack, MainAxisSize,
};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_test::{Options, Tester};

fn screen(width: u16, height: u16, widget: impl Into<AnyWidget>) -> Vec<String> {
    let mut t = Tester::with(Options::new().size(width, height));
    t.pump_widget(widget);
    let mut lines = t.lines();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// `n` rows of `tag`, as one `Text`.
fn rows(tag: &str, n: usize) -> Text {
    Text::new(vec![tag; n].join("\n"))
}

fn loose(child: impl Into<AnyWidget>) -> AnyWidget {
    Flexible {
        fit: FlexFit::Loose,
        ..Flexible::new(1, child)
    }
    .into()
}

fn column() -> Column {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
}

// ---- G49

const LABEL: &str = "Always use current directory (/Users/example/current)";

#[test]
fn probe_g49_route_1_word_wrap_does_not_break_after_a_slash() {
    // The oracle (`cwd_picker_selected_40x16`): `Always use current directory (/`
    // then `Users/example/current)`.
    let lines = screen(35, 4, Text::new(LABEL).wrap(TextWrap::Word));
    assert_eq!(
        lines,
        ["Always use current directory", "(/Users/example/current)"],
        "G49: Word breaks at the space before `(/`, never after the slash"
    );
}

#[test]
fn probe_g49_route_1_grapheme_wrap_breaks_everywhere() {
    let lines = screen(35, 4, Text::new(LABEL).wrap(TextWrap::Grapheme));
    assert_eq!(
        lines,
        ["Always use current directory (/User", "s/example/current)"],
        "G49: Grapheme fills the row to the edge, not the oracle's shape"
    );
}

#[test]
fn probe_g49_route_2_zwsp_after_the_slash_breaks_as_the_oracle_but_rewrites_the_string() {
    // rtwk-unicode `clusters.rs:395`: U+200B is a break opportunity. The rows
    // equal the oracle's, but the label string is rewritten (the boundary rule
    // of process.md §5 calls that a workaround) and the glyph lives in the
    // text a selection copies.
    let rewritten = LABEL.replace("(/", "(/\u{200B}");
    let lines = screen(35, 4, Text::new(rewritten.clone()).wrap(TextWrap::Word));
    assert_eq!(
        lines,
        ["Always use current directory (/", "Users/example/current)"],
        "the ZWSP route draws the oracle's rows"
    );
    assert_ne!(rewritten, LABEL, "and the string is no longer the label");
}

// ---- G50

/// Eight rows: six of fixed text, two free; five flexible one-row children
/// (`a`..`e`) interleaved as the OSS picker's blanks and notes are.
fn five_flex() -> AnyWidget {
    column()
        .child(loose(Text::new("a")))
        .child(rows("x", 2))
        .child(loose(Text::new("b")))
        .child(loose(Text::new("c")))
        .child(rows("y", 2))
        .child(loose(Text::new("d")))
        .child(loose(Text::new("e")))
        .child(rows("z", 2))
        .into()
}

#[test]
fn probe_g50_route_1_column_gives_the_remainder_to_the_first_flexible_children() {
    // 8 rows, 6 fixed: free 2, five flex 1: every share is 0, the two
    // leftover cells go to the first two (`flex.rs:1040-1052`). The oracle's
    // `allocate` gives step 3's slack to the last flexible child: `e` only.
    let lines = screen(10, 8, five_flex());
    assert_eq!(
        lines,
        ["a", "x", "x", "b", "y", "y", "z", "z"],
        "G50: `a` and `b` get the rows (the oracle would show `e`)"
    );
}

#[test]
fn probe_g50_route_1_weights_do_not_move_the_slack_to_the_last() {
    // Weight 1 for the first four and 1000 for `e` (the last): the share of
    // `e` is 2*1000/1004 = 1 and the leftover cell still goes to the first.
    let weighted = column()
        .child(loose(Text::new("a")))
        .child(rows("x", 2))
        .child(loose(Text::new("b")))
        .child(loose(Text::new("c")))
        .child(rows("y", 2))
        .child(loose(Text::new("d")))
        .child(Flexible {
            fit: FlexFit::Loose,
            ..Flexible::new(1000, Text::new("e"))
        })
        .child(rows("z", 2));
    let lines = screen(10, 8, weighted);
    assert_eq!(
        lines,
        ["a", "x", "x", "y", "y", "e", "z", "z"],
        "G50: with weights the result differs at other heights, never `e` alone"
    );
}

// ---- G51

/// The trust step at 40x16 with the oracle's row counts: heading 1, note 3,
/// disclosure 8, choices 2, hint 1 (15 mandatory rows, one free), the two
/// paths 26 rows each when they wrap, the blanks one row.
fn trust_column() -> Column {
    column()
        .child(loose(Text::new(" ")))
        .child(Text::new("Folder access"))
        .child(loose(rows("cwd", 26)))
        .child(loose(Text::new(" ")))
        .child(rows("note", 3))
        .child(loose(rows("root", 26)))
        .child(loose(Text::new(" ")))
        .child(rows("disclosure", 8))
        .child(loose(Text::new(" ")))
        .child(rows("choices", 2))
        .child(loose(Text::new(" ")))
        .child(Text::new("hint"))
        .child(loose(Text::new(" ")))
}

#[test]
fn probe_g51_route_1_column_gives_the_free_row_to_the_first_blank() {
    // The oracle (`only_repository_root_fits_40x16`): `Folder access` on the
    // first row, the root path on the sixth, no blank above.
    let lines = screen(40, 16, trust_column());
    assert_eq!(lines[0], "", "G51: the first blank takes the free row");
    assert_eq!(lines[1], "Folder access");
    assert!(
        !lines.iter().any(|l| l == "root"),
        "G51: and the root path has no row (the oracle shows it)"
    );
}

#[test]
fn probe_g51_route_1_a_heavier_path_still_loses_the_remainder_to_the_blank() {
    let heavy = |tag: &str| -> AnyWidget {
        Flexible {
            fit: FlexFit::Loose,
            ..Flexible::new(1000, rows(tag, 26))
        }
        .into()
    };
    let lines = screen(
        40,
        16,
        column()
            .child(loose(Text::new(" ")))
            .child(Text::new("Folder access"))
            .child(heavy("cwd"))
            .child(loose(Text::new(" ")))
            .child(rows("note", 3))
            .child(heavy("root"))
            .child(loose(Text::new(" ")))
            .child(rows("disclosure", 8))
            .child(loose(Text::new(" ")))
            .child(rows("choices", 2))
            .child(loose(Text::new(" ")))
            .child(Text::new("hint"))
            .child(loose(Text::new(" "))),
    );
    // free 1: cwd's share is 1*1000/2008 = 0, the leftover cell goes to the
    // first flexible child, the top blank.
    assert_eq!(lines[0], "", "G51: flex 1000 on the paths: still the blank");
    assert!(!lines.iter().any(|l| l == "root"));
}

#[test]
fn probe_g51_route_2_variants_by_the_measured_height_need_the_height_of_wrapped_text() {
    // A panel-level `LayoutBuilder` can pick prebuilt columns (blanks kept or
    // dropped) only from the height it is given; where the threshold lies is
    // the rows the mandatory text takes, and that depends on the width: the
    // disclosure takes 8 rows at 40 cells and fewer at 70. Knowing it means
    // measuring the wrapped strings in the app (the removed `reserved_height`
    // arithmetic, `measure::rows`).
    let disclosure = "Trust this folder? Codex can read, edit, and run files here, subject to \
        your permission settings. Folder settings can run code automatically, even without a \
        model request. Continue only if you trust these files. Your trust decision will be saved.";
    let at = |width: u16| screen(width, 20, Text::new(disclosure)).len();
    assert_eq!(at(36), 8, "the oracle's 40x16 disclosure rows");
    assert!(at(66) < at(36), "the threshold moves with the width");
    // The picker itself: the same height and different widths ask for
    // different variants, which the builder cannot tell from `max_height`.
    let pick = |width: u16| {
        screen(
            width,
            16,
            LayoutBuilder::new(|_, constraints| {
                if constraints.max_height < 17 {
                    Text::new("no blanks").into()
                } else {
                    Text::new("blanks").into()
                }
            }),
        )
    };
    assert_eq!(
        pick(40),
        pick(70),
        "height alone picks one variant for both"
    );
}

#[test]
fn probe_g51_route_2_grid_of_content_and_fraction_tracks_shares_the_rows_evenly() {
    // A `Grid` row list is rtwk's other composition for "paths first, blanks
    // after" (`rtwk-render/src/objects/grid.rs:480-490`: content tracks grow
    // by EVEN parts toward what they want, in whole cells, the extra cell to
    // the first; the fraction tracks share what is left). The oracle gives
    // the cwd path `path_height / 2` (floor) and the root the rest
    // (`trust_directory.rs:195-199`): with one path row free the root gets it.
    let tracks = [
        GridTrack::fr(1),
        GridTrack::content(),
        GridTrack::content(),
        GridTrack::fr(1),
        GridTrack::content(),
        GridTrack::content(),
        GridTrack::fr(1),
        GridTrack::content(),
        GridTrack::fr(1),
        GridTrack::content(),
        GridTrack::fr(1),
        GridTrack::content(),
        GridTrack::fr(1),
    ];
    let cells: Vec<AnyWidget> = vec![
        Text::new(" ").into(),
        Text::new("Folder access").into(),
        rows("cwd", 26).wrap(TextWrap::Grapheme).into(),
        Text::new(" ").into(),
        rows("note", 3).into(),
        rows("root", 26).wrap(TextWrap::Grapheme).into(),
        Text::new(" ").into(),
        rows("disclosure", 8).into(),
        Text::new(" ").into(),
        rows("choices", 2).into(),
        Text::new(" ").into(),
        Text::new("hint").into(),
        Text::new(" ").into(),
    ];
    let mut grid = Grid::new()
        .rows(tracks)
        .columns([GridTrack::fr(1)])
        .overflow_intended(true);
    for (row, cell) in cells.into_iter().enumerate() {
        grid = grid.child(GridPlacement::new(cell).at(i32::try_from(row).unwrap(), 0));
    }
    let lines = screen(40, 16, grid);
    // A content track takes its min-intrinsic height (the whole wrapped
    // text) before anything is shared: without `overflow_intended` the grid
    // overflows by 51 rows, with it the paths push the mandatory rows off
    // the screen instead of giving way.
    assert!(
        !lines.iter().any(|l| l == "hint"),
        "G51: the Grid does not clamp the paths to the rows left: {lines:?}"
    );
}

// ---- G52: "at most N lines, then one line cut in the middle by whole path
// segments" (the trust step's `path`, `trust_directory.rs:166-180`)

const LONG_PATH: &str =
    "workspace/nested-checkout/nested-checkout/nested-checkout/nested-checkout/repository";

#[test]
fn probe_g52_route_1_ellipsis_middle_cuts_inside_a_segment() {
    // The oracle: one row, `workspace/…/repository` (`center_truncate_path`
    // keeps the first segment and the last two, whole).
    let lines = screen(
        36,
        3,
        Text::new(LONG_PATH)
            .max_lines(1)
            .overflow(rtwk::text::TextOverflow::EllipsisMiddle),
    );
    assert_eq!(lines.len(), 1);
    assert_ne!(lines[0], "workspace/\u{2026}/repository", "G52: {lines:?}");
    assert!(lines[0].contains('\u{2026}'));
}

#[test]
fn probe_g52_route_1_max_lines_cuts_the_end_of_the_wrapped_path() {
    // Three rows where the oracle would show the one-line form: the text
    // wraps and the last allowed row ends in an ellipsis, with no way to ask
    // the widget whether it exceeded its lines.
    let lines = screen(
        36,
        3,
        Text::new(LONG_PATH)
            .wrap(TextWrap::Grapheme)
            .max_lines(2)
            .overflow(rtwk::text::TextOverflow::Ellipsis),
    );
    assert_eq!(lines.len(), 2);
    assert!(lines[1].ends_with('\u{2026}'), "G52: {lines:?}");
}
