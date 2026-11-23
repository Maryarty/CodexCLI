//! Which rows of chrome a short picker keeps: the heights, by priority, that
//! the picker's pieces get from the rows its pane has. The view asks
//! with the sizes it measured (a `LayoutBuilder` over the pane) and builds
//! each piece at the height given; nothing here measures a string or knows a
//! widget. Optional gaps and previews yield first; header, tabs, search and at
//! least one result stay whenever the height can hold them, and the arrows
//! for rows scrolled out of view are reserved apart from the minimum of
//! result rows. Oracle: `bottom_pane/selection_picker_layout.rs`
//! (`picker_areas`) and the footer/top-gap rules of
//! `list_selection_view.rs` (`render`).

/// What each piece would like (a piece with nothing to show asks 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sizes {
    pub header: u16,
    pub header_gap: u16,
    pub tabs: u16,
    pub search: u16,
    pub rows: u16,
    pub side: u16,
}

/// What each piece gets, top to bottom (the oracle's ten areas).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heights {
    pub header: u16,
    pub header_gap: u16,
    pub tabs: u16,
    pub tab_gap: u16,
    pub search: u16,
    /// The row above the results, for the arrow of rows scrolled out above.
    pub above: u16,
    pub rows: u16,
    /// The row under the results (the arrow of rows below).
    pub below: u16,
    pub side_gap: u16,
    pub side: u16,
}

impl Heights {
    /// The rows all pieces take together.
    pub fn total(&self) -> u16 {
        [
            self.header,
            self.header_gap,
            self.tabs,
            self.tab_gap,
            self.search,
            self.above,
            self.rows,
            self.below,
            self.side_gap,
            self.side,
        ]
        .iter()
        .fold(0u16, |all, part| all.saturating_add(*part))
    }
}

/// The heights of the pieces in a pane `height` rows high (`picker_areas`).
/// The multi-select picker carves the arrows out of the list's own area: a
/// list area of three rows or more has an arrow row at each end
/// (`multi_select_picker.rs`: `list_area.inset(vh(height >= 3, 0))`).
pub fn heights(height: u16, sizes: Sizes, surface: Surface) -> Heights {
    let at = areas(height, sizes);
    if surface != Surface::Multi {
        return at;
    }
    let list = at.above + at.rows + at.below;
    let arrows = u16::from(list >= 3);
    Heights {
        above: arrows,
        below: arrows,
        rows: list - 2 * arrows,
        ..at
    }
}

fn areas(height: u16, sizes: Sizes) -> Heights {
    let tabs = sizes.tabs.min(height);
    let search = sizes.search.min(height.saturating_sub(tabs));
    let minimum_rows = sizes.rows.min(if tabs > 0 { 1 } else { 3 });
    // Keep the elision notice when it fits alongside at least one result row.
    let minimum_header = u16::from(
        sizes.header > 0 && height.saturating_sub(tabs + search) > u16::from(sizes.rows > 0),
    );
    let header = sizes
        .header
        .min(height.saturating_sub(tabs + search + minimum_rows))
        .max(minimum_header);
    let mut remaining = height.saturating_sub(header + tabs + search);
    let arrows = u16::from(remaining >= minimum_rows + 2);
    remaining = remaining.saturating_sub(arrows * 2);
    let minimum_rows = sizes.rows.min(3).min(remaining);
    let header_gap = if header > 0 {
        sizes.header_gap.min(remaining.saturating_sub(minimum_rows))
    } else {
        0
    };
    remaining = remaining.saturating_sub(header_gap);
    let tab_gap = u16::from(tabs > 0 && search > 0 && remaining > minimum_rows);
    remaining = remaining.saturating_sub(tab_gap);
    let side = sizes.side.min(remaining.saturating_sub(minimum_rows + 1));
    let side_gap = u16::from(side > 0);
    let rows = sizes.rows.min(remaining.saturating_sub(side + side_gap));
    Heights {
        header,
        header_gap,
        tabs,
        tab_gap,
        search,
        above: arrows,
        rows,
        below: arrows,
        side_gap,
        side,
    }
}

/// What a picker is drawn on (`PickerSurface`; a multi-select has its own
/// frame, `multi_select_picker.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface {
    /// The terminal's own background: the footer yields, no blank on top.
    #[default]
    Terminal,
    /// The panel of the bottom-pane pickers: a blank row on top (when the
    /// content has more than two rows) and the panel colour.
    Panel,
    /// The multi-select picker: always a blank row on top, a footer that
    /// never yields.
    Multi,
}

/// How a pane `height` rows high divides into the blank row on top, the
/// content (header to results) and the footer (note, preview and hint); the
/// footer yields before the title, the controls and the selected result
/// (`render`'s `footer_rows`).
pub fn split(height: u16, footer: u16, sizes: Sizes, surface: Surface) -> (u16, u16, u16) {
    let minimum =
        u16::from(surface == Surface::Panel) + sizes.header.min(1) + sizes.tabs + sizes.search + 1;
    let footer = match surface {
        Surface::Multi => footer.min(height),
        _ => footer.min(height.saturating_sub(minimum)),
    };
    let content = height - footer;
    let top = match surface {
        Surface::Terminal => 0,
        Surface::Panel => u16::from(content > 2),
        Surface::Multi => u16::from(content > 0),
    };
    (top, content - top, footer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sizes(header: u16, tabs: u16, search: u16, rows: u16) -> Sizes {
        Sizes {
            header,
            header_gap: 1,
            tabs,
            search,
            rows,
            side: 0,
        }
    }

    /// The browser (title, subtitle, five tabs, search, thirty rows) at the
    /// oracle's heights: the snapshots of `short_browser_keeps_header_tabs_search_and_footer`.
    #[test]
    fn the_browser_keeps_its_controls_when_short() {
        let browser = sizes(2, 1, 1, 24);
        // 48x6: one footer row, the blank on top, 4 rows of content.
        let (top, content, footer) = split(6, 1, browser, Surface::Panel);
        assert_eq!((top, content, footer), (1, 4, 1));
        let h = heights(content, browser, Surface::Panel);
        assert_eq!(
            (
                h.header,
                h.header_gap,
                h.tabs,
                h.tab_gap,
                h.search,
                h.above,
                h.rows
            ),
            (1, 0, 1, 0, 1, 0, 1)
        );
        // 48x7: the subtitle comes back.
        let (_, content, _) = split(7, 1, browser, Surface::Panel);
        let h = heights(content, browser, Surface::Panel);
        assert_eq!(
            (h.header, h.tabs, h.search, h.above, h.rows),
            (2, 1, 1, 0, 1)
        );
        // 48x13: the gaps, the arrows and three results.
        let (top, content, footer) = split(13, 1, browser, Surface::Panel);
        assert_eq!((top, content, footer), (1, 11, 1));
        let h = heights(content, browser, Surface::Panel);
        assert_eq!(
            (
                h.header,
                h.header_gap,
                h.tabs,
                h.tab_gap,
                h.search,
                h.above,
                h.rows,
                h.below
            ),
            (2, 1, 1, 1, 1, 1, 3, 1)
        );
        assert_eq!(h.total(), 11);
    }

    #[test]
    fn a_hint_that_wraps_yields_before_the_controls() {
        let browser = sizes(2, 1, 1, 24);
        // 24x6: the hint is two rows wide but only one stays.
        assert_eq!(split(6, 2, browser, Surface::Panel), (1, 4, 1));
        // 24x7: both stay.
        assert_eq!(split(7, 2, browser, Surface::Panel), (1, 4, 2));
    }

    #[test]
    fn nothing_overflows_and_a_result_row_stays_when_there_is_room() {
        for height in 0..40 {
            for rows in [1u16, 3, 8, 24] {
                let wanted = sizes(2, 1, 1, rows);
                let h = heights(height, wanted, Surface::Terminal);
                assert!(h.total() <= height, "{height} rows hold {h:?}");
                if height >= 4 {
                    assert!(h.rows >= 1, "{height} rows keep a result: {h:?}");
                }
            }
        }
    }

    /// The multi-select without a header: the rows of the oracle's snapshot
    /// `short_picker_preserves_selected_row_when_overflow_spacers_yield`
    /// (12 fields, an 80x`height` pane, the hint one row).
    #[test]
    fn the_multi_select_carves_its_arrows_out_of_the_list_area() {
        let multi = Sizes {
            header: 0,
            header_gap: 1,
            tabs: 0,
            search: 1,
            rows: 8,
            side: 0,
        };
        // (pane height, search, arrow above, rows, arrow below)
        let frames = [
            (1, 0, 0, 0, 0),
            (2, 0, 0, 0, 0),
            (3, 1, 0, 0, 0),
            (4, 1, 0, 1, 0),
            (5, 1, 0, 2, 0),
            (6, 1, 1, 1, 1),
            (7, 1, 1, 2, 1),
        ];
        for (pane, search, above, rows, below) in frames {
            let (top, content, footer) = split(pane, 1, multi, Surface::Multi);
            assert_eq!(footer, 1, "height {pane}");
            assert_eq!(top, u16::from(pane > 1), "height {pane}");
            let h = heights(content, multi, Surface::Multi);
            assert_eq!(
                (h.search, h.above, h.rows, h.below),
                (search, above, rows, below),
                "height {pane}"
            );
        }
    }

    #[test]
    fn a_side_preview_waits_for_the_rows() {
        let wanted = Sizes {
            header: 1,
            header_gap: 1,
            tabs: 0,
            search: 0,
            rows: 4,
            side: 3,
        };
        let tight = heights(8, wanted, Surface::Terminal);
        assert_eq!((tight.rows, tight.side), (4, 0));
        let ample = heights(20, wanted, Surface::Terminal);
        assert_eq!((ample.rows, ample.side, ample.side_gap), (4, 3, 1));
    }
}
