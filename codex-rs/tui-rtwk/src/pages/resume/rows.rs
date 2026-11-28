//! The list's rows: one saved thread in either density, the expanded row's
//! details and conversation, the line a list with no rows shows, and the
//! overlay while a transcript loads. Each row is composed of rtwk's widgets
//! (a `Row` of a marker, a title that ends in the theme's ellipsis, a `Wrap` of
//! the meta parts) under the row's fill (`BoxWidget`), as Flutter writes a
//! `ListTile`; the list, its cursor and its scroll are `SessionList`'s.
//! Oracle: `resume_picker.rs` `render_list`, `render_comfortable_session_lines`,
//! `render_dense_session_lines`, `render_footer_lines`,
//! `render_expanded_session_details`, `render_conversation_preview_lines`.

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    BoxWidget, Center, Column, CrossAxisAlignment, EdgeInsets, Expanded, IntrinsicHeight,
    MainAxisSize, Padding, Row, SizedBox, VerticalDivider, Wrap,
};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{DefaultTextStyle, Text, TextOverflow, TextWrap};
use rtwk_rich::markdown::MarkdownTheme;

use codex_tui_core::screens::Shared;
use codex_tui_core::screens::agents::display_directory;
use codex_tui_core::screens::resume::{
    Key as RowKey, Load, Model, Preview, PreviewLine, Row as Saved, Sort, Speaker,
};

use super::density::Density;
use super::look::Look;
use super::strings::Strings;
use crate::widgets::age::age;

/// The date column of a meta line and of a dense row (oracle:
/// `SESSION_META_DATE_WIDTH`).
const DATE_WIDTH: i32 = 12;
/// The meta line sits two cells in (oracle: `SESSION_META_INDENT_WIDTH`).
const META_INDENT: i32 = 2;
/// The cwd column's bounds (oracle: `SESSION_META_MIN_CWD_WIDTH`, `_MAX_`).
const CWD_MIN: i32 = 30;
const CWD_MAX: i32 = 72;
/// The gap between two meta parts (oracle: `SESSION_META_FIELD_GAP_WIDTH`).
const FIELD_GAP: i32 = 2;
/// The first column of an expanded row's details.
const DETAIL_COLUMN: i32 = 10;

/// What one frame of the list draws from: the model, which rows pass the
/// filter and the search, and the width the rows get.
#[derive(Clone)]
pub struct Rows {
    pub look: Look,
    pub strings: &'static Strings,
    pub num: NumSyms,
    pub model: Shared<Model>,
    pub density: Density,
    pub expanded: Option<RowKey>,
    /// The index in `model.rows` of each row the list shows.
    pub shown: Rc<Vec<usize>>,
    pub home: Option<Rc<str>>,
    pub width: i32,
    pub markdown: MarkdownTheme,
    /// The session cells' strings, which the transcript preview's Markdown reads.
    pub cells: &'static crate::pages::session::cells::strings::Strings,
}

fn one_line(text: Text) -> Text {
    text.wrap(TextWrap::None)
        .max_lines(1)
        .overflow(TextOverflow::Ellipsis)
}

impl Rows {
    fn model(&self) -> &Model {
        &self.model
    }

    fn saved(&self, index: usize) -> &Saved {
        &self.model().rows[self.shown[index]]
    }

    /// The cwd column of a meta line (oracle: `cwd_column_width`).
    fn cwd_width(&self) -> i32 {
        ((self.width - META_INDENT - DATE_WIDTH - 2 * FIELD_GAP) / 2).clamp(CWD_MIN, CWD_MAX)
    }

    /// The row `index` of what the list shows, the cursor on it or not.
    pub fn row(&self, index: usize, selected: bool) -> AnyWidget {
        let row = self.saved(index);
        let expanded =
            selected && !row.id.as_str().is_empty() && self.expanded.as_ref() == Some(&row.key());
        let fill = if selected {
            self.look.selected.clone()
        } else if index.is_multiple_of(2) {
            self.look.zebra.clone()
        } else {
            Style::new()
        };
        let dense = self.density == Density::Dense;
        let summary = self.summary(row, selected, expanded, dense, &fill);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        if expanded {
            column = column
                .child(BoxWidget::new().style(fill).child(summary))
                .child(self.expansion(row));
        } else if dense {
            column = column.child(BoxWidget::new().style(fill).child(summary));
        } else {
            column = column.child(
                BoxWidget::new().style(fill.clone()).child(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(summary)
                        .child(self.meta(row, &fill)),
                ),
            );
        }
        if !dense && index + 1 < self.shown.len() {
            column = column.child(SizedBox::new(None, Some(1), None));
        }
        column.into()
    }

    /// The marker, the unread dot and the title (and the date column when
    /// dense), in the row's fill.
    fn summary(
        &self,
        row: &Saved,
        selected: bool,
        expanded: bool,
        dense: bool,
        fill: &Style,
    ) -> AnyWidget {
        let (look, strings) = (&self.look, self.strings);
        let paint = |own: Style| fill.merge(&own);
        let (marker, marker_style) = match (selected, expanded) {
            (true, true) => (
                strings.marker_expanded,
                look.selected.clone().attrs(Attrs::BOLD),
            ),
            (true, false) => (strings.marker_selected, look.selected.clone()),
            (false, _) => (strings.marker_other, Style::new()),
        };
        let mut line = Row::new().child(Text::line(marker).style(paint(marker_style)));
        if dense {
            let sort = self.model().sort;
            let stamp = match sort {
                Sort::Created => row.created,
                Sort::Updated => row.updated.or(row.created),
            };
            let date = strings.age(age(self.model().clock, stamp), &self.num);
            line = line.child(SizedBox::new(
                Some(DATE_WIDTH),
                Some(1),
                Some(one_line(Text::line(date).style(paint(look.dim.clone()))).into()),
            ));
        }
        if row.unread {
            line = line.child(Text::line(strings.unread_dot).style(paint(look.attention.clone())));
        }
        let base = if selected {
            Style::new().attrs(Attrs::BOLD)
        } else {
            Style::new()
        };
        let title = look.thread(&base, row.id.as_str());
        let title = one_line(Text::new(row.display_preview().to_owned()).style(paint(title)));
        // A dense title is cut one cell short of its column (oracle:
        // `dense_column_text`).
        let title: AnyWidget = if dense {
            Padding::new(EdgeInsets::only(0, 0, 1, 0), title).into()
        } else {
            title.into()
        };
        line.child(Expanded::new(1, title)).into()
    }

    /// The meta line: the date, the directory unless the list is that
    /// directory's, the branch; wrapped onto another line where they do not
    /// fit (oracle: `pack_footer_parts`).
    fn meta(&self, row: &Saved, fill: &Style) -> AnyWidget {
        let (look, strings) = (&self.look, self.strings);
        let model = self.model();
        let paint = |own: Style| fill.merge(&own);
        let stamp = match model.sort {
            Sort::Created => row.created,
            Sort::Updated => row.updated.or(row.created),
        };
        let date = strings.age(age(model.clock, stamp), &self.num);
        let part = |icon: &'static str, text: Option<String>, none: &'static str| -> Text {
            let (text, style) = match text {
                Some(text) => (text, paint(look.dim.clone())),
                None => (none.to_owned(), paint(look.italic.clone())),
            };
            one_line(Text::rich([
                Span::styled(icon, paint(look.dim.clone())),
                Span::styled(text, style),
            ]))
        };
        let mut wrap = Wrap::new()
            .spacing(FIELD_GAP)
            .run_spacing(0)
            .child(SizedBox::new(
                Some(DATE_WIDTH),
                Some(1),
                Some(one_line(Text::line(date).style(paint(look.dim.clone()))).into()),
            ));
        if model.shows_cwd(row) {
            let cwd = row
                .cwd
                .as_deref()
                .map(|path| display_directory(path, self.home.as_deref()));
            wrap = wrap.child(SizedBox::new(
                Some(self.cwd_width()),
                Some(1),
                Some(part(strings.cwd_icon, cwd, strings.no_cwd).into()),
            ));
        }
        wrap = wrap.child(part(
            strings.branch_icon,
            row.branch.clone(),
            strings.no_branch,
        ));
        Padding::new(EdgeInsets::only(META_INDENT, 0, 0, 0), wrap).into()
    }

    /// The expanded row's details and conversation under a bar that ends in
    /// `└` once the conversation is read (oracle:
    /// `render_transcript_preview_lines`).
    fn expansion(&self, row: &Saved) -> AnyWidget {
        let (look, strings) = (&self.look, self.strings);
        let model = self.model();
        let dim = look.dim.clone();
        let mut lines = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let detail = |label: &'static str, value: String| -> AnyWidget {
            Row::new()
                .child(SizedBox::new(
                    Some(DETAIL_COLUMN),
                    Some(1),
                    Some(Text::line(label).style(dim.clone()).into()),
                ))
                .child(SizedBox::new(Some(2), Some(1), None))
                .child(Expanded::new(1, one_line(Text::new(value))))
                .into()
        };
        let session = match (row.name(), row.id.as_str()) {
            (Some(name), id) if !id.is_empty() => format!("{name} ({id})"),
            (Some(name), _) => name.to_owned(),
            (None, id) if !id.is_empty() => id.to_owned(),
            (None, _) => strings.detail_none.to_owned(),
        };
        let when = |stamp: Option<i64>| -> String {
            match stamp {
                Some(epoch) => format!(
                    "{} · {}",
                    strings.age_long(age(model.clock, stamp), &self.num),
                    strings.timestamp(epoch, model.clock_24h)
                ),
                None => strings.detail_none.to_owned(),
            }
        };
        let directory = row.cwd.as_deref().map_or_else(
            || strings.detail_none.to_owned(),
            |path| display_directory(path, self.home.as_deref()),
        );
        let branch = format!(
            "{}{}",
            strings.branch_icon,
            row.branch.as_deref().unwrap_or(strings.no_branch)
        );
        lines = lines
            .child(detail(strings.detail_session, session))
            .child(detail(strings.detail_created, when(row.created)))
            .child(detail(
                strings.detail_updated,
                when(row.updated.or(row.created)),
            ))
            .child(detail(strings.detail_directory, directory))
            .child(detail(strings.detail_branch, branch))
            .child(Text::line(strings.blank))
            .child(Text::line(strings.detail_conversation).style(dim.clone()));
        let mut ends = false;
        match model.previews.get(&row.key()) {
            Some(Preview::Loading) => {
                lines = lines.child(Text::new(strings.preview_loading).style(look.italic.clone()));
            }
            Some(Preview::Failed) => {
                lines = lines.child(
                    Text::new(strings.preview_failed)
                        .style(look.error.clone().attrs(Attrs::ITALIC)),
                );
            }
            Some(Preview::Loaded(preview)) => {
                ends = true;
                if preview.is_empty() {
                    lines = lines.child(Text::new(strings.preview_none).style(look.italic.clone()));
                }
                for line in preview {
                    lines = lines.child(self.said(line, row.cwd.as_deref()));
                }
            }
            None => {}
        }
        // The bar: `│` down the lines, `└` on the last one (a loaded
        // conversation ends it).
        let bar = Padding::new(
            EdgeInsets::only(META_INDENT, 0, 1, 0),
            VerticalDivider::new().style(dim.clone()),
        );
        let gutter: AnyWidget = if ends {
            Column::new()
                .child(Expanded::new(1, bar))
                .child(Text::line(strings.bar_end).style(dim.clone()))
                .into()
        } else {
            bar.into()
        };
        IntrinsicHeight::new(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(SizedBox::new(Some(4), None, Some(gutter)))
                .child(Expanded::new(1, lines)),
        )
        .into()
    }

    /// One line of the conversation: the user's in italics, the assistant's
    /// as Markdown in the dim style.
    fn said(&self, line: &PreviewLine, cwd: Option<&str>) -> AnyWidget {
        match line.speaker {
            Speaker::User => Text::new(line.text.clone())
                .style(Style::new().attrs(Attrs::ITALIC))
                .wrap(TextWrap::Word)
                .into(),
            Speaker::Assistant => DefaultTextStyle::new(
                self.look.dim.clone(),
                // read under the user's `tui.rendering` where it is built (math, tables, lists)
                crate::pages::session::cells::citation::cited(
                    &line.text,
                    false,
                    cwd,
                    self.cells,
                    self.markdown.clone(),
                ),
            )
            .into(),
        }
    }

    /// What a list with no rows says (oracle: `render_empty_state_line`).
    pub fn empty(&self) -> AnyWidget {
        let (look, strings, model) = (&self.look, self.strings, self.model());
        let text = if !model.query.is_empty() {
            if model.listing.search.is_some() || model.listing.is_loading() {
                strings.searching
            } else {
                strings.no_results
            }
        } else if model.listing.is_loading() || matches!(model.load, Load::Loading) {
            if model.rows.is_empty() {
                strings.loading
            } else {
                strings.loading_older
            }
        } else {
            strings.empty
        };
        Text::new(text).style(look.italic.clone()).into()
    }

    /// The line under the last row while a page loads (oracle: `render_list`).
    pub fn loading_older(&self) -> AnyWidget {
        Row::new()
            .child(SizedBox::new(Some(2), Some(1), None))
            .child(Text::line(self.strings.loading_older).style(self.look.italic.clone()))
            .into()
    }
}

/// The overlay over the list while a transcript loads: a filled box of three
/// rows (one when the list is short), the message bold in its middle
/// (oracle: `render_transcript_loading_overlay`).
pub fn overlay(
    look: &Look,
    strings: &'static Strings,
    message_width: i32,
    room: (i32, i32),
) -> AnyWidget {
    let width = if room.0 >= message_width + 10 {
        message_width + 10
    } else {
        room.0
    };
    let height = if room.1 >= 3 { 3 } else { 1 };
    Center::new(SizedBox::new(
        Some(width),
        Some(height),
        Some(
            BoxWidget::new()
                .style(look.overlay.clone())
                .child(Center::new(
                    Text::line(strings.overlay).style(Style::new().attrs(Attrs::BOLD)),
                ))
                .into(),
        ),
    ))
    .into()
}
