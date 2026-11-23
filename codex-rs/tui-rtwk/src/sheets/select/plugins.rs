//! `/plugins` in the selection sheet (oracle `chatwidget/plugin_catalog.rs`, `chatwidget/plugins.rs`):
//! the browser with its marketplace tabs, the detail of a plugin, the loading, failure and
//! confirmation pages around it and the prompt of a marketplace source. The model (core's
//! `plugins::Plugins`) says which page is shown and what is on it; the words are
//! `plugins/strings.rs`. Every page is the shared picker's list: the browser is its tabbed form with
//! the opener's extras (a toggle key, Ctrl-R and Ctrl-U on a marketplace tab, a note per tab, a
//! result area that keeps its height). In Flutter the browser is a `TabBarView` of `ListView`s in
//! a `showModalBottomSheet`, the other pages are the sheet's `Navigator` routes.

use rtwk::Handler;

use codex_tui_core::keymap::{Key, Keymap, Mods, Stroke};
use codex_tui_core::screens::select::list::{Columns, Item, List, Role, Span, Tab as ListTab};
use codex_tui_core::screens::select::plugins::catalog::{
    Auth as AuthPolicy, Catalog, Note, Placeholder, Source, tab_matching,
};
use codex_tui_core::screens::select::plugins::{
    self, Action, AuthStep, Detail, Kind, Menu, Plugins, Product, Row, Tab, Toggle,
};

pub mod strings;

use self::strings::Strings;
use super::prompt::{Answer, Prompt};
use crate::keymap::stroke_label;
use crate::widgets::picker::{Extras, Hot, TabNote, list_key_free};

/// The name of the hot key that removes the marketplace of the tab (Ctrl-R).
pub const REMOVE_KEY: &str = "remove";
/// The name of the hot key that upgrades it (Ctrl-U).
pub const UPGRADE_KEY: &str = "upgrade";

/// The width of the browser's name column is at most this many cells (`plugins_popup_params`).
const NAME_COLUMN_LIMIT: usize = 36;
/// The mark, the toggle and the cursor before a row's name (`PLUGIN_ROW_PREFIX_WIDTH`).
const ROW_PREFIX_WIDTH: usize = 6;

/// What the plugin menu on top is shown as.
pub enum Page {
    /// The tabs of the catalog and what the opener adds to the picker.
    Browser(List, Extras),
    /// Any other page: a list of rows.
    List(List),
    /// A question over the chat: two rows, centred.
    Confirm(List),
    /// The prompt of the marketplace source.
    Add,
}

/// A number that tells the pages apart: a refresh of the same page keeps its picker (and its
/// cursor, its query and its tab), another page is a new one.
pub fn shape(menu: &Menu) -> u32 {
    match menu {
        Menu::Loading => 0,
        Menu::Failed(_) => 1,
        Menu::List => 2,
        Menu::AddPrompt => 3,
        Menu::AddLoading => 4,
        Menu::AddFailed => 5,
        Menu::RemoveConfirm { .. } => 6,
        Menu::RemoveLoading { .. } => 7,
        Menu::RemoveFailed { .. } => 8,
        Menu::UpgradeLoading { .. } => 9,
        Menu::DetailLoading { .. } => 10,
        Menu::Detail(_) => 11,
        Menu::DetailFailed { .. } => 12,
        Menu::InstallLoading { .. } => 13,
        Menu::UninstallLoading { .. } => 14,
        Menu::Auth => 15,
    }
}

/// The page of the menu the model holds; none for the setup of apps when no step is on.
pub fn page(plugins: &Plugins, auth: Option<&AuthStep>, keys: &Keymap, s: &Strings) -> Option<Page> {
    Some(match plugins.menu() {
        Menu::Loading => Page::List(loading(s.loading_head.to_string(), Some(s.loading_note), s.loading_row, s.loading_row_description, s)),
        Menu::Failed(error) => Page::List(plain(
            s.failed_subtitle,
            vec![disabled(s.failed_row, error)],
            s,
        )),
        Menu::List => match browser(plugins, keys, s) {
            Some((list, extras)) => Page::Browser(list, extras),
            None => Page::List(loading(s.loading_head.to_string(), Some(s.loading_note), s.loading_row, s.loading_row_description, s)),
        },
        Menu::AddPrompt => Page::Add,
        Menu::AddLoading => Page::List(loading(
            s.add_loading.to_string(),
            None,
            s.add_loading,
            s.add_loading_description,
            s,
        )),
        Menu::AddFailed => {
            let mut items = vec![
                disabled(s.add_failed_row, s.add_failed_description),
                row(plugins::RETRY, s.retry, s.retry_add_description),
            ];
            items.extend(back(plugins, s));
            Page::List(closing(s.add_failed_subtitle, items, s))
        }
        Menu::RemoveConfirm { label, .. } => Page::Confirm(confirmation(label, s)),
        Menu::RemoveLoading { label } => Page::List(plain(
            &s.with(s.remove_loading_subtitle, label),
            vec![disabled(s.remove_loading_row, s.remove_loading_description)],
            s,
        )),
        Menu::RemoveFailed { .. } => {
            let mut items = vec![
                disabled(s.remove_failed_row, s.remove_failed_description),
                row(plugins::RETRY, s.retry, s.retry_remove_description),
            ];
            items.extend(back(plugins, s));
            Page::List(closing(s.remove_failed_subtitle, items, s))
        }
        Menu::UpgradeLoading { name } => {
            let text = match name {
                Some(name) => s.with(s.upgrade_loading_one, name),
                None => s.upgrade_loading_all.to_string(),
            };
            Page::List(loading(
                text.clone(),
                None,
                &text,
                s.upgrade_loading_description,
                s,
            ))
        }
        Menu::DetailLoading { name } => Page::List(loading(
            s.with(s.detail_loading, name),
            None,
            s.detail_loading_row,
            s.detail_loading_description,
            s,
        )),
        Menu::Detail(detail) => Page::List(detail_page(detail, s)),
        Menu::DetailFailed { error, back } => {
            let mut items = vec![disabled(s.detail_failed_row, error)];
            if *back {
                items.push(row(plugins::BACK, s.back, s.back_description));
            }
            Page::List(closing(s.detail_failed_subtitle, items, s))
        }
        Menu::InstallLoading { name } => Page::List(plain(
            &s.with(s.install_loading_subtitle, name),
            vec![disabled(s.install_loading_row, s.install_loading_description)],
            s,
        )),
        Menu::UninstallLoading { name } => Page::List(plain(
            &s.with(s.uninstall_loading_subtitle, name),
            vec![disabled(
                s.uninstall_loading_row,
                s.uninstall_loading_description,
            )],
            s,
        )),
        Menu::Auth => Page::List(auth_page(auth?, s)),
    })
}

/// The prompt of the marketplace source (`open_marketplace_add_prompt`).
pub fn add_prompt(on_answer: Handler<Answer>, s: &'static Strings) -> Prompt {
    Prompt {
        title: s.prompt_title,
        context: Some(s.prompt_context),
        placeholder: s.prompt_placeholder,
        hint: s.prompt_hint,
        mark: s.prompt_mark,
        on_answer,
    }
}

// ---------------------------------------------------------------- the rows every page has

fn dim(text: impl Into<String>) -> Vec<Span> {
    vec![Span {
        text: text.into(),
        role: Role::Dim,
    }]
}

fn base(s: &Strings) -> List {
    List {
        title: Some(s.title.to_string()),
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}

/// A row that is only said, never picked.
fn disabled(name: &str, description: &str) -> Item {
    Item {
        name: name.to_string(),
        description: Some(description.to_string()),
        disabled: true,
        ..Item::default()
    }
}

/// A row that is picked; it says the same highlighted or not.
fn row(id: &str, name: &str, description: &str) -> Item {
    Item {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        selected_description: Some(description.to_string()),
        ..Item::default()
    }
}

/// `Back to plugins` when the list can be shown again.
fn back(plugins: &Plugins, s: &Strings) -> Option<Item> {
    plugins
        .catalog()
        .map(|_| row(plugins::BACK, s.back, s.back_description))
}

/// A page the loading of something stands in for (`DelayedLoadingHeader`): the words of the load
/// under the title, one row that waits.
fn loading(head: String, note: Option<&str>, row_name: &str, row_description: &str, s: &Strings) -> List {
    let mut header = vec![dim(head)];
    header.extend(note.map(dim));
    List {
        header,
        standard_hint: true,
        items: vec![disabled(row_name, row_description)],
        ..base(s)
    }
}

/// A page of a subtitle and its rows, the standard hint under them.
fn plain(subtitle: &str, items: Vec<Item>, s: &Strings) -> List {
    List {
        subtitle: Some(subtitle.to_string()),
        standard_hint: true,
        items,
        ..base(s)
    }
}

/// A page of a subtitle and its rows that says only how to close it (`plugin_detail_hint_line`).
fn closing(subtitle: &str, items: Vec<Item>, s: &Strings) -> List {
    List {
        footer_hint: Some(s.hint_close.to_string()),
        standard_hint: false,
        ..plain(subtitle, items, s)
    }
}

/// `marketplace_remove_confirmation_popup_params`.
fn confirmation(label: &str, s: &Strings) -> List {
    let mut remove = row(plugins::REMOVE, s.remove_row, s.remove_row_description);
    remove.selected_description = Some(s.remove_row_description.to_string());
    List {
        header: vec![
            dim(s.with(s.remove_confirm_head, label)),
            dim(s.remove_confirm_note),
        ],
        footer_hint: Some(s.hint_select_close.to_string()),
        items: vec![
            remove,
            row(plugins::BACK, s.back, s.remove_keep_description),
        ],
        ..base(s)
    }
}

// ------------------------------------------------------------------------ the browser

/// The text a row says for its marketplace: the product's label, else the marketplace's own.
fn market_label(product: Product, own: &str, s: &Strings) -> String {
    s.product(product).unwrap_or(own).to_string()
}

fn width_of(text: &str) -> usize {
    rtwk_core::unicode::string_width(text, rtwk_core::unicode::TextMode::Graphemes, 1)
}

/// The label of a tab, told apart from its twins (`disambiguate_duplicate_tab_labels`).
fn tab_label(tab: &Tab, catalog: &Catalog, s: &Strings) -> String {
    match &tab.kind {
        Kind::All => s.tab_all.to_string(),
        Kind::Installed => s.with_count(s.tab_installed, tab.installed),
        Kind::Curated => s.product_curated.to_string(),
        Kind::Marketplace {
            name,
            product,
            twin,
            ..
        } => {
            let own = catalog
                .marketplaces
                .iter()
                .find(|market| market.tab_id() == tab.id)
                .map_or(name.as_str(), |market| market.display());
            let label = market_label(*product, own, s);
            match twin {
                Some((index, total)) => s.twin(&label, *index, *total),
                None => label,
            }
        }
        Kind::Section { section, .. } => s.section(*section).to_string(),
        Kind::Add => s.tab_add.to_string(),
    }
}

/// The two lines under the title of a tab (`plugins_header`).
fn tab_header(tab: &Tab, label: &str, s: &Strings) -> Vec<Vec<Span>> {
    let (first, second) = match &tab.kind {
        Kind::All => (
            s.all_title.to_string(),
            s.with_counts(s.all_count, tab.installed, tab.total),
        ),
        Kind::Installed => (
            s.installed_title.to_string(),
            s.with_count(s.installed_count, tab.installed),
        ),
        Kind::Curated => (
            s.curated_title.to_string(),
            s.with_counts(s.curated_count, tab.installed, tab.total),
        ),
        Kind::Marketplace { .. } if tab.fresh => (
            s.with(s.market_fresh_title, label),
            s.market_fresh_count.to_string(),
        ),
        Kind::Marketplace { .. } => (
            s.with(s.market_title, label),
            s.market_counts(tab.installed, tab.total, label),
        ),
        Kind::Section { state, .. } => match state {
            Placeholder::Loading => (
                s.with(s.section_loading_title, label),
                s.section_loading_count.to_string(),
            ),
            Placeholder::Empty => (
                s.with(s.market_title, label),
                s.section_empty_count.to_string(),
            ),
            Placeholder::Failed(_) => (
                s.with(s.section_failed_title, label),
                s.section_failed_count.to_string(),
            ),
        },
        Kind::Add => (s.add_title.to_string(), s.add_count.to_string()),
    };
    vec![dim(first), dim(second)]
}

/// The footer of the browser; a marketplace the user configured adds its keys
/// (`plugins_popup_hint_line`).
fn hint_line(remove: bool, upgrade: bool, toggle_free: bool, s: &Strings) -> String {
    let mut hints = Vec::new();
    if upgrade {
        hints.push(s.with(s.hint_upgrade, &stroke_label(&upgrade_stroke())));
    }
    if remove {
        hints.push(s.with(s.hint_remove, &stroke_label(&remove_stroke())));
    }
    let owned = remove || upgrade;
    if owned && toggle_free {
        hints.push(s.hint_toggle.to_string());
    }
    hints.push(s.hint_tabs.to_string());
    hints.push(s.hint_details.to_string());
    if !owned && toggle_free {
        hints.push(s.hint_toggle.to_string());
    }
    hints.push(s.hint_close.to_string());
    hints.join(s.hint_join)
}

fn remove_stroke() -> Stroke {
    Stroke::new(Key::Char('r'), Mods::CTRL)
}

fn upgrade_stroke() -> Stroke {
    Stroke::new(Key::Char('u'), Mods::CTRL)
}

/// The tabs of the catalog (`plugins_popup_params`); none while the model has no list.
fn browser(plugins: &Plugins, keys: &Keymap, s: &Strings) -> Option<(List, Extras)> {
    let catalog = plugins.catalog()?;
    let tabs = plugins.tabs();
    let all = tabs.first()?;
    let toggle = Stroke::new(Key::Tab, Mods::NONE);
    let toggle_free = list_key_free(keys, &toggle);
    let name_column_width = all
        .rows
        .iter()
        .map(|row| ROW_PREFIX_WIDTH + width_of(&row.name))
        .chain([width_of(s.add_row)])
        .max()
        .map(|width| width.min(NAME_COLUMN_LIMIT));

    let mut shown = Vec::new();
    let mut notes = Vec::new();
    let mut hot = Vec::new();
    for tab in &tabs {
        let label = tab_label(tab, catalog, s);
        let mut note = TabNote {
            header: tab_header(tab, &label, s),
            hint: None,
        };
        if let Kind::Marketplace {
            remove, upgrade, ..
        } = &tab.kind
        {
            if *remove || *upgrade {
                note.hint = Some(hint_line(*remove, *upgrade, toggle_free, s));
            }
            if *remove {
                hot.push(Hot {
                    tab: tab.id.clone(),
                    stroke: remove_stroke(),
                    name: REMOVE_KEY,
                });
            }
            if *upgrade {
                hot.push(Hot {
                    tab: tab.id.clone(),
                    stroke: upgrade_stroke(),
                    name: UPGRADE_KEY,
                });
            }
        }
        notes.push(note);
        shown.push(ListTab {
            id: tab.id.clone(),
            label,
            items: items_of(tab, catalog, toggle_free, s),
        });
    }
    let list = List {
        footer_hint: Some(hint_line(false, false, toggle_free, s)),
        tabs: shown,
        // The tab the user is at, or the one that took its place (a loading section that has
        // loaded: `plugin_tab_id_matching_saved_id`).
        initial_tab: Some(
            tab_matching(plugins.tab(), &tabs).unwrap_or_else(|| plugins.tab().to_string()),
        ),
        searchable: true,
        search_placeholder: Some(s.search.to_string()),
        single_line: true,
        name_column_width,
        ..base(s)
    };
    let extras = Extras {
        toggle: Some(toggle),
        hot,
        notes,
        reserve: true,
        keep_position: true,
    };
    Some((list, extras))
}

/// The rows of a tab: its plugins, or what stands in for them.
fn items_of(tab: &Tab, catalog: &Catalog, toggle_free: bool, s: &Strings) -> Vec<Item> {
    let width = tab
        .rows
        .iter()
        .map(|row| width_of(s.status(row.status)))
        .max()
        .unwrap_or(0);
    let with_market = matches!(tab.kind, Kind::All | Kind::Installed);
    let mut items: Vec<Item> = tab
        .rows
        .iter()
        .map(|row| plugin_item(row, catalog, with_market, width, toggle_free, s))
        .collect();
    let has = !items.is_empty();
    match &tab.kind {
        Kind::All if !has => items.push(disabled(s.none_all, s.none_all_description)),
        Kind::Installed if !has => items.push(disabled(s.none_installed, s.none_installed)),
        Kind::Marketplace { .. } if !has => items.push(disabled(s.none_market, s.none_market)),
        Kind::Curated => {
            let label = s.product_curated;
            match &tab.note {
                Note::Loading => items.push(disabled(
                    &s.with(s.row_loading, label),
                    s.loading_curated,
                )),
                Note::Failed(error) => items.push(disabled(
                    &s.with(s.row_unavailable, s.part(error.part)),
                    &s.section_error(&error.cause),
                )),
                Note::None if !has => items.push(disabled(s.none_curated, s.none_curated)),
                Note::None => {}
            }
        }
        Kind::Section { section, state } => {
            let label = s.section(*section);
            items.push(match state {
                Placeholder::Loading => disabled(
                    &s.with(s.row_loading, label),
                    match section {
                        plugins::Section::Workspace => s.loading_workspace,
                        plugins::Section::SharedWithMe => s.loading_shared,
                    },
                ),
                Placeholder::Empty => match section {
                    plugins::Section::Workspace => {
                        disabled(s.none_workspace, s.none_workspace_description)
                    }
                    plugins::Section::SharedWithMe => {
                        disabled(s.none_shared, s.none_shared_description)
                    }
                },
                Placeholder::Failed(error) => disabled(
                    &s.with(s.row_unavailable, label),
                    &s.section_error(&error.cause),
                ),
            });
        }
        Kind::Add => items.push(Item {
            id: plugins::ADD_ROW.to_string(),
            name: s.add_row.to_string(),
            description: Some(s.add_row_description.to_string()),
            selected_description: Some(s.add_row_selected.to_string()),
            ..Item::default()
        }),
        _ => {}
    }
    items
}

/// A plugin's row (`plugin_selection_items`). `width` is the widest status label of the tab, which
/// the labels are padded to.
fn plugin_item(
    row: &Row,
    catalog: &Catalog,
    with_market: bool,
    width: usize,
    toggle_free: bool,
    s: &Strings,
) -> Item {
    let status = format!("{:<width$}", s.status(row.status));
    let market = market_label(row.product, &row.marketplace, s);
    let mut parts = vec![status.clone()];
    if with_market {
        parts.push(market.clone());
    }
    parts.extend(row.description.clone());
    let description = parts.join(s.dot);

    let installed = catalog
        .row(&row.id)
        .is_some_and(|(_, plugin)| plugin.installed);
    let can_view = row.can_view();
    let tail = match row.toggle {
        Toggle::Switch(on) if toggle_free => {
            let word = if on { s.word_disable } else { s.word_enable };
            if can_view {
                s.with(s.selected_toggle, word)
            } else {
                s.with(s.selected_toggle_only, word)
            }
        }
        _ if (row.toggle == Toggle::Blocked || installed) && can_view => {
            s.selected_view.to_string()
        }
        _ if row.toggle == Toggle::Blocked || installed => s.selected_unavailable.to_string(),
        _ if can_view => s.selected_install.to_string(),
        _ => s.selected_remote.to_string(),
    };
    let selected = s.fmt(
        s.selected,
        [
            rtwk::l10n::Arg::Text(&status),
            rtwk::l10n::Arg::Text(&tail),
        ],
    );
    let search = format!(
        "{} {} {} {} {} {}",
        row.name,
        row.plugin,
        row.plugin_name,
        market,
        row.description.as_deref().unwrap_or_default(),
        row.keywords.join(" ")
    );
    let (toggle, mark) = match row.toggle {
        Toggle::Switch(on) => (Some(on), None),
        Toggle::Blocked => (None, Some(s.mark_blocked)),
        Toggle::Unavailable => (None, Some(s.mark_unavailable)),
    };
    let unavailable = !can_view && !installed;
    Item {
        id: row.id.clone(),
        name: row.name.clone(),
        description: Some(description),
        selected_description: Some(selected),
        search_value: Some(search),
        toggle,
        prefix_spans: mark
            .map(|text| Span {
                text: text.to_string(),
                role: Role::Plain,
            })
            .into_iter()
            .collect(),
        disabled: unavailable,
        disabled_reason: unavailable.then(|| s.unavailable_reason.to_string()),
        ..Item::default()
    }
}

// ------------------------------------------------------------------------- the detail

/// `plugin_detail_popup_params`.
fn detail_page(detail: &Detail, s: &Strings) -> List {
    let plugin = &detail.plugin;
    let market = market_label(detail.product, &detail.marketplace, s);
    let mut header = vec![vec![Span {
        text: s.fmt(
            s.detail_head,
            [
                rtwk::l10n::Arg::Text(&plugin.display_name),
                rtwk::l10n::Arg::Text(s.detail_status(plugin.detail_status())),
                rtwk::l10n::Arg::Text(&market),
            ],
        ),
        role: Role::Strong,
    }]];
    if detail.disclosure() {
        let run = |text: &str, role: Role| Span {
            text: text.to_string(),
            role,
        };
        header.push(vec![
            run(s.disclosure_start, Role::Plain),
            run(s.disclosure_terms, Role::Strong),
            run(s.disclosure_and, Role::Plain),
            run(s.disclosure_privacy, Role::Strong),
            run(s.disclosure_stop, Role::Plain),
            run(s.disclosure_link, Role::Info),
            run(s.disclosure_end, Role::Plain),
        ]);
    }
    if let Some(description) = &detail.description {
        header.push(dim(description.clone()));
    }

    let mut items = vec![row(plugins::BACK, s.back, s.back_description)];
    items.push(match detail.action() {
        Action::InstalledByAdmin => {
            disabled(s.admin_installed, s.admin_installed_description)
        }
        Action::Uninstall { .. } => row(plugins::UNINSTALL, s.uninstall, s.uninstall_description),
        Action::UninstallUnavailable => disabled(s.uninstall, s.uninstall_missing),
        Action::DisabledByAdmin => disabled(s.install, s.install_disabled),
        Action::NotInstallable => disabled(s.install, s.install_not_available),
        Action::Install(_) => row(plugins::INSTALL, s.install, s.install_description),
        Action::InstallUnavailable => disabled(s.install, s.install_no_location),
    });
    items.push(disabled(s.meta_source, &source_summary(detail, s)));
    items.push(disabled(
        s.meta_auth,
        match plugin.auth {
            AuthPolicy::OnInstall => s.auth_on_install,
            AuthPolicy::OnUse => s.auth_on_use,
        },
    ));
    if let Some(version) = version_summary(detail, s) {
        items.push(disabled(s.meta_version, &version));
    }
    if let Some(share) = &plugin.share {
        items.push(disabled(s.meta_sharing, &share_summary(share, s)));
    }
    let names = |list: &[String], none: &'static str| {
        if list.is_empty() {
            none.to_string()
        } else {
            list.join(s.list_join)
        }
    };
    items.push(disabled(
        s.meta_skills,
        &names(&detail.skills, s.none_skills),
    ));
    let hooks: Vec<String> = detail
        .hooks
        .iter()
        .map(|(event, count)| {
            s.fmt(
                s.hook,
                [
                    rtwk::l10n::Arg::Text(event),
                    rtwk::l10n::Arg::Int(u64::try_from(*count).unwrap_or(u64::MAX)),
                ],
            )
        })
        .collect();
    items.push(disabled(s.meta_hooks, &names(&hooks, s.none_hooks)));
    items.push(disabled(s.meta_apps, &names(&detail.apps, s.none_apps)));
    items.push(disabled(
        s.meta_mcp,
        &names(&detail.mcp_servers, s.none_mcp),
    ));
    List {
        header,
        footer_hint: Some(s.hint_close.to_string()),
        items,
        ..base(s)
    }
}

/// `plugin_source_summary`.
fn source_summary(detail: &Detail, s: &Strings) -> String {
    use rtwk::l10n::Arg::Text;
    match &detail.plugin.source {
        Source::Local => s.source_local.to_string(),
        Source::Git {
            url,
            reference: Some(reference),
        } => s.fmt(s.source_git_ref, [Text(url), Text(reference)]),
        Source::Git { url, reference: None } => s.with(s.source_git, url),
        Source::Npm {
            package,
            version: Some(version),
        } => s.fmt(s.source_npm_version, [Text(package), Text(version)]),
        Source::Npm {
            package,
            version: None,
        } => s.with(s.source_npm, package),
        Source::Remote => {
            let by_name = Product::of_name(&detail.marketplace);
            s.with(
                s.source_remote,
                &market_label(by_name, &detail.marketplace, s),
            )
        }
    }
}

/// `plugin_version_summary`.
fn version_summary(detail: &Detail, s: &Strings) -> Option<String> {
    let plugin = &detail.plugin;
    let mut parts = Vec::new();
    if let Some(local) = &plugin.local_version {
        parts.push(s.with(s.version_local, local));
    }
    if let Some(remote) = plugin
        .share
        .as_ref()
        .and_then(|share| share.remote_version.as_deref())
    {
        parts.push(s.with(s.version_remote, remote));
    }
    (!parts.is_empty()).then(|| parts.join(s.dot))
}

/// `plugin_share_context_summary`.
fn share_summary(share: &plugins::catalog::Share, s: &Strings) -> String {
    use rtwk::l10n::Arg::Text;
    let mut parts = Vec::new();
    if let Some(kind) = share.discoverability {
        parts.push(s.discoverability(kind).to_string());
    }
    match (share.creator_name.as_deref(), share.creator_id.as_deref()) {
        (Some(name), Some(id)) => parts.push(s.fmt(s.share_creator_account, [Text(name), Text(id)])),
        (Some(name), None) => parts.push(s.with(s.share_creator, name)),
        (None, Some(id)) => parts.push(s.with(s.share_account, id)),
        (None, None) => {}
    }
    if let Some(principals) = &share.principals {
        parts.push(match principals.as_slice() {
            [] => s.share_no_principals.to_string(),
            [one] => s.with(s.share_one_principal, one),
            many => s.with_count(s.share_principals, many.len()),
        });
    }
    if let Some(url) = share.url.as_deref().filter(|url| !url.trim().is_empty()) {
        parts.push(url.to_string());
    }
    if parts.is_empty() {
        s.with(s.share_remote_id, &share.remote_id)
    } else {
        parts.join(s.dot)
    }
}

// ---------------------------------------------------------------------------- the setup

/// `plugin_install_auth_popup_params`: one app the installed plugin needs signed in.
fn auth_page(step: &AuthStep, s: &Strings) -> List {
    let header = vec![
        vec![Span {
            text: s.with(s.auth_installed, &step.plugin),
            role: Role::Strong,
        }],
        dim(s.fmt(
            s.auth_step,
            [
                rtwk::l10n::Arg::Int(u64::try_from(step.current).unwrap_or(u64::MAX)),
                rtwk::l10n::Arg::Int(u64::try_from(step.total).unwrap_or(u64::MAX)),
                rtwk::l10n::Arg::Text(&step.app.name),
            ],
        )),
        dim(if step.installed {
            s.auth_already
        } else {
            s.auth_needed
        }),
    ];
    let mut items = Vec::new();
    if step.app.url.is_some() {
        let mut open = row(
            plugins::OPEN,
            if step.installed {
                s.auth_manage
            } else {
                s.auth_install
            },
            s.auth_open_description,
        );
        open.selected_description = Some(s.auth_open_selected.to_string());
        items.push(open);
    } else {
        items.push(disabled(s.auth_no_link, s.auth_no_link_description));
    }
    let mut next = if step.installed {
        row(
            plugins::NEXT,
            s.auth_continue,
            s.auth_continue_description,
        )
    } else {
        row(plugins::NEXT, s.auth_done, s.auth_done_description)
    };
    next.selected_description = Some(
        if step.installed {
            s.auth_continue_selected
        } else {
            s.auth_done_selected
        }
        .to_string(),
    );
    items.push(next);
    let mut skip = row(plugins::SKIP, s.auth_skip, s.auth_skip_description);
    skip.selected_description = Some(s.auth_skip_selected.to_string());
    items.push(skip);
    List {
        header,
        footer_hint: Some(s.hint_close.to_string()),
        items,
        ..base(s)
    }
}
