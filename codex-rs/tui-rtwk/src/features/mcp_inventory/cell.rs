//! The `/mcp` cell (oracle `history_cell/mcp.rs` `new_mcp_tools_output_from_statuses`,
//! `empty_mcp_output`, `McpInventoryLoadingCell`): the magenta typed command, the title, one row
//! per server with its state coloured (`status_style`), and under it, for `/mcp verbose`, its
//! auth, tools and resources; an empty list says where the docs are, with a link.
//!
//! Flutter: a `Column` of `Text.rich` (the rows wrap by word); the words are `strings.rs`'s, the
//! data the feature's (`Inventory`).

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::{Attrs, Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::features::mcp_inventory::{Connection, Entry, Inventory, Note, Server};

use super::strings::{
    AUTH_LABEL, COMMAND, DETAIL_MARK, DOCS_AFTER, DOCS_BEFORE, DOCS_LINK, DOCS_URL, LOADING_DOTS,
    LOADING_MARK, LOADING_WORDS, NO_SERVERS, NO_TOOLS, NONE, RESOURCES_LABEL, SERVER_MARK,
    TEMPLATES_LABEL, TITLE, TITLE_MARK, TOOLS_LABEL, VERBOSE_HINT, auth, connection, tool_count,
    uri,
};
use crate::app::theme::Tokens;

/// What `/mcp` left in the transcript: the inventory or the empty list (a failure is a row of
/// text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryCell {
    pub note: Note,
}

impl_widget!(stateless eq InventoryCell);

impl StatelessWidget for InventoryCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let mut rows = Rows::new(&tokens);
        rows.head();
        match &self.note {
            Note::Empty => rows.empty(),
            Note::Inventory(inventory) => rows.inventory(inventory),
            Note::Failed(_) => {}
        }
        rows.column.into()
    }
}

/// The row of the loading ask: the bullet, the bold words and their dim dots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadingCell;

impl_widget!(stateless eq LoadingCell);

impl StatelessWidget for LoadingCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        Text::rich(vec![
            Span::styled(LOADING_MARK, tokens.dim().attrs(Attrs::DIM)),
            Span::new(" "),
            Span::styled(LOADING_WORDS, tokens.bold()),
            Span::styled(LOADING_DOTS, tokens.dim().attrs(Attrs::DIM)),
        ])
        .into()
    }
}

struct Rows<'a> {
    tokens: &'a Tokens,
    column: Column,
}

impl<'a> Rows<'a> {
    fn new(tokens: &'a Tokens) -> Self {
        Rows {
            tokens,
            column: Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
        }
    }

    fn push(&mut self, spans: Vec<Span>) {
        let column = std::mem::take(&mut self.column);
        self.column = column.child(Text::rich(spans));
    }

    fn blank(&mut self) {
        let column = std::mem::take(&mut self.column);
        self.column = column.child(SizedBox::new(None, Some(1), None));
    }

    /// The command, the title and a blank under each.
    fn head(&mut self) {
        self.push(vec![Span::styled(COMMAND, self.tokens.accent())]);
        self.blank();
        self.push(vec![
            Span::new(TITLE_MARK),
            Span::styled(TITLE, self.tokens.bold()),
        ]);
        self.blank();
    }

    fn italic(&self) -> Style {
        Style::new().attrs(Attrs::ITALIC)
    }

    fn empty(&mut self) {
        self.push(vec![Span::styled(NO_SERVERS, self.italic())]);
        let dim = self.tokens.dim().attrs(Attrs::DIM);
        self.push(vec![
            Span::styled(DOCS_BEFORE, dim.clone()),
            Span::styled(DOCS_LINK, dim.clone().attrs(Attrs::UNDERLINE)).link(DOCS_URL),
            Span::styled(DOCS_AFTER, dim),
        ]);
    }

    fn inventory(&mut self, inventory: &Inventory) {
        if inventory.no_tools() {
            self.push(vec![Span::styled(NO_TOOLS, self.italic())]);
            self.blank();
        }
        for server in &inventory.servers {
            self.server(server, inventory.full);
        }
        if !inventory.full {
            self.blank();
            self.push(vec![Span::styled(
                VERBOSE_HINT,
                self.tokens.dim().attrs(Attrs::DIM),
            )]);
        }
    }

    /// The state's style (oracle `status_style`: green, yellow and red bold; the accent for a
    /// server still starting; dim for one that is not running).
    fn state(&self, connection: Connection) -> Style {
        let tokens = self.tokens;
        match connection {
            Connection::Connected => tokens.success().attrs(Attrs::BOLD),
            Connection::Starting => tokens.accent().attrs(Attrs::BOLD),
            Connection::AuthenticationRequired => tokens.warning().attrs(Attrs::BOLD),
            Connection::Failed => tokens.error().attrs(Attrs::BOLD),
            Connection::NotStarted
            | Connection::Disabled
            | Connection::Cancelled
            | Connection::Unknown => tokens.dim().attrs(Attrs::DIM),
        }
    }

    fn server(&mut self, server: &Server, full: bool) {
        let state = self.state(server.connection);
        let dim = self.tokens.dim().attrs(Attrs::DIM);
        self.push(vec![
            Span::styled(SERVER_MARK, state.clone()),
            Span::styled(server.name.clone(), self.tokens.bold()),
            Span::new(": "),
            Span::styled(connection(server.connection), state),
            Span::styled(tool_count(server.tools.len()), dim),
        ]);
        if !full {
            return;
        }
        self.push(vec![
            Span::new(DETAIL_MARK),
            Span::new(AUTH_LABEL),
            Span::new(auth(server.auth)),
        ]);
        let tools = if server.tools.is_empty() {
            NONE.to_owned()
        } else {
            server.tools.join(", ")
        };
        self.push(vec![
            Span::new(DETAIL_MARK),
            Span::new(TOOLS_LABEL),
            Span::new(tools),
        ]);
        self.entries(RESOURCES_LABEL, &server.resources);
        self.entries(TEMPLATES_LABEL, &server.templates);
        self.blank();
    }

    /// `• Resources: a (uri), b (uri)`, the uris dim.
    fn entries(&mut self, label: &'static str, entries: &[Entry]) {
        let mut spans = vec![Span::new(DETAIL_MARK), Span::new(label)];
        if entries.is_empty() {
            spans.push(Span::new(NONE));
        }
        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                spans.push(Span::new(", "));
            }
            spans.push(Span::new(entry.label.clone()));
            spans.push(Span::new(" "));
            spans.push(Span::styled(
                uri(&entry.uri),
                self.tokens.dim().attrs(Attrs::DIM),
            ));
        }
        self.push(spans);
    }
}
