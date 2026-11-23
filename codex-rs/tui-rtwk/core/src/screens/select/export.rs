//! `/export` (oracle `chatwidget/transcript_export.rs` `show_transcript_export_popup`): the two
//! rows of the destination picker and what a destination is. The list is the view's (its title,
//! subtitle, names and descriptions are words, ADR-010) as a `Menu::List` whose row ids are
//! these; the app takes the pick (`Picked::Item(id)`) and carries the export out
//! (`app/session/export.rs`).

/// The row ids of the destination picker, in order (oracle: "Copy to clipboard", "Save to
/// file").
pub const CLIPBOARD: &str = "export-clipboard";
pub const FILE: &str = "export-file";

/// The effect requests of the export carry this bit: the UI's own request numbers never do.
pub const REQUEST_BIT: u64 = 1 << 59;

/// A row of the destination picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// "Copy to clipboard": the complete Markdown transcript is copied.
    Clipboard,
    /// "Save to file": a filename is asked for (the export prompt).
    File,
}

impl Row {
    /// The row a list row id stands for.
    pub fn of_row(id: &str) -> Option<Self> {
        match id {
            CLIPBOARD => Some(Self::Clipboard),
            FILE => Some(Self::File),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Clipboard => CLIPBOARD,
            Self::File => FILE,
        }
    }
}

/// Where an exported transcript goes (oracle `TranscriptExportDestination`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Clipboard,
    /// A path as typed: `~/` is the home directory, a relative path is under the thread's
    /// directory; a file that exists is never overwritten.
    File(String),
}
