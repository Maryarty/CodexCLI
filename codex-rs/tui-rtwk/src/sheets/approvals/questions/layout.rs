//! The rows the editor of the async questions gives its parts (oracle
//! `bottom_pane/async_questions/layout.rs`): the question, the choices, the inline input and the
//! footer share the height the pane has, in the oracle's order of need. The measures (rows of the
//! wrapped question, of the choices, of the input, of the footer) are the view's, read from the
//! measured width; what is decided here is only how many rows each part gets, so the view
//! composes the parts in boxes of those heights (Flutter: a `Column` of `SizedBox`es under a
//! `LayoutBuilder`).

/// The spacer rows the oracle would like between the sections.
pub const DESIRED_SPACERS: u16 = 2;

/// What the layout is told of the question on show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub has_options: bool,
    pub other_selected: bool,
    /// More than one question is open (a progress row shows).
    pub several: bool,
    /// The rows of the wrapped question.
    pub question_rows: u16,
    /// The rows the choices (and the inline input of Other) need.
    pub options_rows: u16,
    /// The rows the freeform input prefers (1 to 8).
    pub input_rows: u16,
    /// The rows of the footer, without and with the `option n/m` tip.
    pub footer_rows: u16,
    pub footer_rows_with_tip: u16,
}

/// The rows of each part, top to bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sections {
    pub progress: u16,
    pub spacer_before_question: u16,
    pub question: u16,
    pub spacer_after_question: u16,
    pub options: u16,
    pub spacer_after_options: u16,
    pub notes: u16,
    pub spacer_after_input: u16,
    pub footer: u16,
    /// Choices are cut: the footer says which one the cursor is on.
    pub show_option_tip: bool,
}

/// `layout_sections`: the rows of a pane `height` rows tall.
pub fn sections(height: u16, facts: Facts) -> Sections {
    let Facts {
        has_options,
        other_selected,
        several,
        question_rows,
        options_rows: full_options,
        input_rows,
        ..
    } = facts;
    let mut footer_pref = facts.footer_rows;
    if has_options
        && usize::from(question_rows)
            + usize::from(full_options)
            + usize::from(footer_pref)
            + usize::from(several)
            + usize::from(DESIRED_SPACERS)
            > usize::from(height)
    {
        footer_pref = facts.footer_rows_with_tip;
    }
    let progress_pref = u16::from(several);
    let min_notes = u16::from(!has_options).min(height);
    let available = height.saturating_sub(min_notes);
    let question = question_rows.min(available.saturating_sub(u16::from(other_selected)));
    let mut remaining = available.saturating_sub(question);
    let mut out = Sections {
        question,
        ..Sections::default()
    };
    if has_options {
        let min_options = remaining.min(1);
        let reserved = footer_pref + progress_pref + DESIRED_SPACERS;
        let mut options = full_options.min(remaining.saturating_sub(reserved).max(min_options));
        remaining -= options;
        out.progress = progress_pref.min(remaining);
        remaining -= out.progress;
        out.spacer_after_options = u16::from(remaining > footer_pref);
        remaining -= out.spacer_after_options;
        out.footer = footer_pref.min(remaining);
        remaining -= out.footer;
        out.spacer_after_question = u16::from(remaining > 0);
        remaining -= out.spacer_after_question;
        options += remaining.min(full_options.saturating_sub(options));
        out.options = options;
        out.notes = 0;
    } else {
        let preferred = input_rows
            .clamp(1, 8)
            .saturating_sub(min_notes)
            .min(remaining);
        remaining -= preferred;
        out.footer = footer_pref.min(remaining);
        remaining -= out.footer;
        out.progress = progress_pref.min(remaining);
        remaining -= out.progress;
        out.spacer_after_input = u16::from(remaining > 0);
        remaining -= out.spacer_after_input;
        out.spacer_before_question = u16::from(remaining >= 2 && several);
        out.spacer_after_question = u16::from(remaining >= 1);
        out.notes = min_notes + preferred + remaining
            - out.spacer_before_question
            - out.spacer_after_question;
    }
    out.show_option_tip = out.options > 0 && out.options < full_options;
    out
}
