//! The detection of the sources (oracle: `flow.rs`
//! `ExternalAgentConfigDetection`): each source is asked, the ones with items
//! are kept, a failure of one is tolerated and a failure of all is an error.

use codex_app_server_protocol::ExternalAgentConfigMigrationItem as Item;

/// An app to import from (oracle: `ExternalAgentConfigMigrationSource`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum Source {
    #[serde(rename = "claude-code")]
    Claude,
    #[serde(rename = "cursor")]
    Cursor,
}

impl Source {
    /// The sources asked, in order.
    pub const ALL: [Source; 2] = [Source::Claude, Source::Cursor];

    /// What the app server calls it (`migration_source`).
    pub fn id(self) -> &'static str {
        match self {
            Source::Claude => "claude-code",
            Source::Cursor => "cursor",
        }
    }

    pub fn of_id(id: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|source| source.id() == id)
    }
}

/// A source with the items found in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detected {
    pub source: Source,
    pub items: Vec<Item>,
}

/// What the detection came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Sources(Vec<Detected>),
    NoItems,
    /// Every source failed: what each said.
    Failed(Vec<(Source, String)>),
}

#[derive(Debug, Default)]
pub struct Detection {
    sources: Vec<Detected>,
    errors: Vec<(Source, String)>,
}

impl Detection {
    pub fn record_items(&mut self, source: Source, items: Vec<Item>) {
        if !items.is_empty() {
            self.sources.push(Detected { source, items });
        }
    }

    pub fn record_error(&mut self, source: Source, error: &str) {
        self.errors.push((source, error.to_owned()));
    }

    pub fn finish(self) -> Outcome {
        if !self.sources.is_empty() {
            Outcome::Sources(self.sources)
        } else if self.errors.is_empty() {
            Outcome::NoItems
        } else {
            Outcome::Failed(self.errors)
        }
    }
}
