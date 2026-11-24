//! `/resume <name or id>` (oracle `named_session_lookup.rs`, `lookup_session_target_with_app_server`,
//! `AppEvent::ResumeSessionByIdOrName`): the session a displayed label or an id stands for,
//! found through the server's listing, with no IO of its own. A session id is read directly
//! (`thread/read`); a label is looked for in the pages of `thread/list` (newest created
//! first, the interactive sources, the state database) and must name one session only.
//! The answer is a `Verdict`: the settings feature turns it into a wish for the surface (open
//! the thread: ADR-003, the one door is `OpenThread`) or a note.

/// A session of a listing, as the lookup reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub id: String,
    /// The name, else the preview (`display_label`), trimmed.
    pub label: String,
}

/// The label a session is shown and found by: its name, else its preview.
pub fn display_label(name: Option<&str>, preview: &str) -> String {
    name.map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| preview.trim())
        .to_owned()
}

/// Whether `text` is a UUID in one of the forms `Uuid::parse_str` takes (hyphenated or simple;
/// a session id is one of these).
pub fn is_session_id(text: &str) -> bool {
    let hex = |part: &str| part.chars().all(|ch| ch.is_ascii_hexdigit());
    let hyphenated = || {
        let parts: Vec<&str> = text.split('-').collect();
        parts.len() == 5
            && parts
                .iter()
                .zip([8, 4, 4, 4, 12])
                .all(|(part, len)| part.len() == len && hex(part))
    };
    (text.len() == 32 && hex(text)) || (text.len() == 36 && hyphenated())
}

/// A cursor whose position the server does not tie to a session (`<time>|<id>`): a page
/// boundary after it can skip sessions of an equal time.
fn unverified(cursor: &str) -> bool {
    !matches!(cursor.rsplit_once('|'), Some((time, id)) if !time.is_empty() && !id.is_empty())
}

/// What a lookup by label has read so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lookup {
    pub name: String,
    matched: Option<String>,
    unverified: bool,
}

/// What follows a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// The next page, from this cursor.
    Next(String),
    Done(Verdict),
}

/// How a lookup ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The one session of that label (or the one the id names).
    Found(String),
    /// `No saved chat found matching '{name}'.`
    Missing,
    /// `Multiple sessions match '{name}' (including {first} and {second}); use a session UUID to
    /// disambiguate.`
    Ambiguous { first: String, second: String },
    /// `Cannot verify a unique session label across server pages; matching session UUID: {id}.
    /// Use it only if this is the session you want.`
    Paginated(String),
}

impl Lookup {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Self::default()
        }
    }

    /// A page of the listing, with the cursor of the page after it.
    pub fn page(&mut self, rows: &[Listed], next: Option<String>) -> Step {
        for row in rows.iter().filter(|row| row.label == self.name) {
            match &self.matched {
                Some(first) if *first != row.id => {
                    return Step::Done(Verdict::Ambiguous {
                        first: first.clone(),
                        second: row.id.clone(),
                    });
                }
                _ => self.matched = Some(row.id.clone()),
            }
        }
        match next {
            Some(cursor) => {
                self.unverified |= unverified(&cursor);
                Step::Next(cursor)
            }
            None => Step::Done(match self.matched.take() {
                Some(id) if self.unverified => Verdict::Paginated(id),
                Some(id) => Verdict::Found(id),
                None => Verdict::Missing,
            }),
        }
    }
}
