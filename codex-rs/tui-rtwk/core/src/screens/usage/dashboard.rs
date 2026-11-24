//! The dashboard of cards (`z`, oracle: `analytics/dashboard.rs`): how many
//! cards sit in a row and how tall each is for the room the body gets, and
//! what the first lines of a card say before its report. Heights depend on
//! the room only, never on the selected day or a load, so a load never shifts
//! the cards.

use super::{Section, View};

/// From this width the cards sit two in a row.
const TWO_COLUMNS: usize = 104;
/// The gap between the two cards of a row.
pub const CARD_GAP: usize = 4;
const MIN_HEIGHT: usize = 16;
const MAX_HEIGHT: usize = 24;
/// The rows a card keeps for its border and the blank row inside each.
const FRAME_ROWS: usize = 4;
/// The rows of a card's content the plot leaves to the rest of its report.
const PLOT_OVERHEAD: usize = 13;
/// The columns a card keeps for its border and the blank inside each.
const FRAME_COLUMNS: usize = 4;

/// The grid of the cards for a body of `width` by `height` cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    /// Cards in a row: 2 from 104 cells, else 1.
    pub columns: usize,
    pub card_width: usize,
    pub card_height: usize,
    /// Rows of cards (at least one, an account with no report has none to show).
    pub rows: usize,
    pub cards: usize,
}

impl Geometry {
    /// The grid for `cards` cards (oracle: `dashboard_lines`).
    pub fn new(width: usize, height: usize, cards: usize) -> Self {
        let columns = if width >= TWO_COLUMNS { 2 } else { 1 };
        let card_width = if columns == 2 {
            (width - CARD_GAP) / 2
        } else {
            width
        };
        let rows = cards.div_ceil(columns).max(1);
        let card_height = (height.saturating_sub(2) / rows).clamp(MIN_HEIGHT, MAX_HEIGHT);
        Self {
            columns,
            card_width,
            card_height,
            rows,
            cards,
        }
    }

    /// The cells a card's content gets across.
    pub fn inner_width(&self) -> usize {
        self.card_width.saturating_sub(FRAME_COLUMNS).max(1)
    }

    /// The rows a card's content gets.
    pub fn inner_height(&self) -> usize {
        self.card_height - FRAME_ROWS
    }

    /// The rows the plot of a report gets in a card (oracle: what is left
    /// of the content after the lead, the head, the table and the notes).
    pub fn plot_rows(&self) -> usize {
        self.inner_height().saturating_sub(PLOT_OVERHEAD).max(1)
    }

    /// The row the `card`-th card is in.
    pub fn row_of(&self, card: usize) -> usize {
        card / self.columns
    }

    /// The cards of row `row`, as indexes.
    pub fn cards_of(&self, row: usize) -> std::ops::Range<usize> {
        let first = row * self.columns;
        first..(first + self.columns).min(self.cards)
    }
}

/// What a card says first: its range, the model filter of a business token
/// report and its grouping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lead {
    /// The range is the long one (30 days); none for the Overview, Chats and
    /// Plan.
    pub range: Option<bool>,
    /// The model filter beside the range: `Some(None)` is all models.
    pub model: Option<Option<String>>,
    /// The grouping (`By <group>`).
    pub group: Option<usize>,
}

impl View<'_> {
    /// The lines that open the card of `section` (oracle: `dashboard_lines`).
    pub fn card_lead(&self, section: Section) -> Lead {
        let ranged = !matches!(section, Section::Chats | Section::Plan | Section::Summary);
        Lead {
            range: ranged.then(|| self.range_of(section) != 0),
            model: (ranged && section == Section::Usage && self.business())
                .then(|| self.sel.token_model.clone()),
            group: matches!(
                section,
                Section::Usage | Section::Credits | Section::Activity | Section::Plan
            )
            .then(|| self.group_of(section)),
        }
    }
}
