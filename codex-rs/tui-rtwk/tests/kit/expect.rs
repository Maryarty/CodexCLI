//! What an oracle test asserts that is not a snapshot row: the turns the app started, the
//! requests it made, the words on the screen however they wrapped. A shard names none of core's
//! types and writes no `assert!`; each of these is the oracle's own expectation, spelled once.

use std::ops::Range;
use std::path::{Path, PathBuf};

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::effects::{Ask as Effect, Done};
use codex_tui_core::app::{AppEv, BackendEv, Command};
use codex_tui_core::data::mention::ToolMention;
use codex_tui_core::screens::select::review::Commit;
use codex_tui_core::screens::{ScreenCommand, agents, chat, select};
use codex_tui_core::threads::{ReviewTarget, ThreadAsk};
use serde_json::json;

use super::Screen;

/// What a command the app asked is, as the shards name it (`Screen::asked_*`).
pub type Ask = Command;

fn squash(text: &str) -> String {
    text.split_whitespace().collect()
}

/// The turn the oracle expects the app to start: only what is set is compared.
#[derive(Default)]
pub struct Turn {
    text: Option<String>,
    remote: Option<Vec<String>>,
    images: Option<Vec<(PathBuf, Option<Range<usize>>)>>,
    skills: Option<Vec<(String, String)>>,
    mentions: Option<Vec<(String, String)>>,
}

impl Turn {
    /// A turn whose text is `text`.
    pub fn text(text: &str) -> Turn {
        Turn {
            text: Some(text.to_owned()),
            ..Turn::default()
        }
    }

    /// A turn of any text.
    pub fn any() -> Turn {
        Turn::default()
    }

    /// The remote images (URLs) it carries.
    pub fn remote_images(mut self, urls: &[&str]) -> Turn {
        self.remote = Some(urls.iter().map(|u| (*u).to_owned()).collect());
        self
    }

    /// It carries no local image.
    pub fn no_images(mut self) -> Turn {
        self.images = Some(Vec::new());
        self
    }

    /// It carries these local images and no other, the placeholders' ranges in its text as given.
    pub fn images(mut self, images: &[(&Path, Range<usize>)]) -> Turn {
        self.images = Some(
            images
                .iter()
                .map(|(path, range)| (path.to_path_buf(), Some(range.clone())))
                .collect(),
        );
        self
    }

    /// It carries these local images and no other (whatever their placeholders' ranges).
    pub fn image_paths(mut self, paths: &[&Path]) -> Turn {
        self.images = Some(
            paths
                .iter()
                .map(|path| (path.to_path_buf(), None))
                .collect(),
        );
        self
    }

    /// It binds these skills (`name`, `path` of the skill's file) and nothing else.
    pub fn skills(mut self, skills: &[(&str, &str)]) -> Turn {
        self.skills = Some(
            skills
                .iter()
                .map(|(n, p)| ((*n).to_owned(), (*p).to_owned()))
                .collect(),
        );
        self
    }
}

impl Turn {
    /// It binds these tool mentions (`name`, `path`: `app://...`, `plugin://...`) and nothing else.
    pub fn mentions(mut self, mentions: &[(&str, &str)]) -> Turn {
        self.mentions = Some(
            mentions
                .iter()
                .map(|(n, p)| ((*n).to_owned(), (*p).to_owned()))
                .collect(),
        );
        self
    }
}

impl Screen {
    // ---- the words on the screen -------------------------------------------------------

    /// The screen reads `text`, however the panel wrapped it (white space aside).
    pub fn assert_reads(&self, text: &str) {
        assert!(
            squash(&self.text()).contains(&squash(text)),
            "the screen does not read `{text}`:\n{}",
            self.text()
        );
    }

    /// The screen reads no `text`, however it is wrapped.
    pub fn assert_not_reads(&self, text: &str) {
        assert!(
            !squash(&self.text()).contains(&squash(text)),
            "the screen reads `{text}`:\n{}",
            self.text()
        );
    }

    /// The screen holds `text` exactly when `shown`.
    pub fn assert_present(&self, text: &str, shown: bool) {
        if shown {
            self.assert_contains(text);
        } else {
            self.assert_absent(text);
        }
    }

    /// `text` stands `n` times on the screen.
    pub fn assert_text_count(&self, text: &str, n: usize) {
        pretty_assertions::assert_eq!(
            self.text().matches(text).count(),
            n,
            "`{text}` on the screen:\n{}",
            self.text()
        );
    }

    /// `n` rows contain `text`.
    pub fn assert_rows_with(&self, text: &str, n: usize) {
        let got = self.rows().iter().filter(|row| row.contains(text)).count();
        pretty_assertions::assert_eq!(got, n, "rows with `{text}`:\n{}", self.text());
    }

    /// Some row reads `text` once its blanks at both ends are cut.
    pub fn assert_has_row_trimmed(&self, text: &str) {
        assert!(
            self.rows().iter().any(|row| row.trim() == text.trim()),
            "no row reads `{text}`:\n{}",
            self.text()
        );
    }

    /// The first row that holds `anchor` also holds every text of `holds` and none of `lacks`.
    pub fn assert_row_of(&self, anchor: &str, holds: &[&str], lacks: &[&str]) {
        let rows = self.rows();
        let Some(row) = rows.iter().find(|row| row.contains(anchor)) else {
            panic!("no row holds `{anchor}`:\n{}", self.text());
        };
        for text in holds {
            assert!(
                row.contains(text),
                "the row of `{anchor}` lacks `{text}`: {row}"
            );
        }
        for text in lacks {
            assert!(
                !row.contains(text),
                "the row of `{anchor}` holds `{text}`: {row}"
            );
        }
    }

    /// The first row that is not empty reads `text` (blanks at both ends cut).
    pub fn assert_first_row(&self, text: &str) {
        let rows = self.rows();
        let first = rows.iter().find(|row| !row.is_empty());
        pretty_assertions::assert_eq!(
            first.map(|row| row.trim()),
            Some(text),
            "the first row:\n{}",
            self.text()
        );
    }

    /// No row is wider than `width` characters.
    pub fn assert_rows_fit(&self, width: usize) {
        assert!(
            self.rows().iter().all(|row| row.chars().count() <= width),
            "a row is wider than {width}:\n{}",
            self.text()
        );
    }

    /// `n` rows start with `prefix` once their leading blanks are cut.
    pub fn assert_rows_starting(&self, prefix: &str, n: usize) {
        let got = self
            .rows()
            .iter()
            .filter(|row| row.trim_start().starts_with(prefix))
            .count();
        pretty_assertions::assert_eq!(got, n, "rows starting `{prefix}`:\n{}", self.text());
    }

    /// `n` rows read exactly `text` (trailing blanks aside).
    pub fn assert_row_count(&self, text: &str, n: usize) {
        let got = self
            .rows()
            .iter()
            .filter(|row| *row == text.trim_end())
            .count();
        pretty_assertions::assert_eq!(got, n, "rows reading `{text}`:\n{}", self.text());
    }

    /// The row that reads `row` is followed by exactly the rows `following`, in the rows the
    /// oracle's Markdown renderer speaks of (the agent cell's gutter cut off).
    pub fn assert_row_then(&self, row: &str, following: &[&str]) {
        row_then(&self.grid().rows(false), &self.text(), row, following);
    }

    /// `assert_row_then` in the screen's rows as they are (the raw render has no gutter).
    pub fn assert_screen_row_then(&self, row: &str, following: &[&str]) {
        row_then(&self.rows(), &self.text(), row, following);
    }

    /// The first row of the oracle snapshot `name` and the rows under it equal ours, from the
    /// screen's top, trailing blanks cut.
    pub fn assert_snap_head(&self, name: &str) {
        let want: Vec<String> = super::snap_rows(name, "")
            .iter()
            .map(|row| row.trim_end().to_string())
            .collect();
        let have: Vec<String> = self.rows().into_iter().take(want.len()).collect();
        pretty_assertions::assert_eq!(
            have,
            want,
            "{name}: ours (left) differs from the oracle (right)"
        );
    }

    /// The rows of frame `frame` of the oracle snapshot `name` (`assert_snap_section`'s header)
    /// equal ours, trailing blanks cut, but for the rows in `except` (the port's own words: a
    /// row of the oracle's test cell the thread's items spell otherwise).
    pub fn assert_snap_frame_except(&self, name: &str, frame: &str, except: &[usize]) {
        let ours = self.rows();
        for (at, want) in super::snap_rows(name, frame).iter().enumerate() {
            if except.contains(&at) {
                continue;
            }
            let got = ours.get(at).map_or("", |row| row.trim_end());
            pretty_assertions::assert_eq!(
                got,
                want.trim_end(),
                "{name} / {frame}: row {at}\n{}",
                self.text()
            );
        }
    }

    /// The rows of the oracle snapshot `name` equal ours from screen row `row`, trailing blanks
    /// cut (no cell style is checked).
    pub fn assert_snap_rows_at(&self, name: &str, row: usize) {
        let ours = self.rows();
        assert!(
            row < ours.len(),
            "{name}: the popup is not on the screen:\n{}",
            self.text()
        );
        for (at, want) in super::snap_rows(name, "").iter().enumerate() {
            let got = ours.get(row + at).map_or("", |r| r.trim_end());
            pretty_assertions::assert_eq!(got, want.trim_end(), "{name}: row {at}");
        }
    }

    /// The rows, each trimmed and all joined without a separator, hold `text` (a text the
    /// width wrapped stays whole in them).
    pub fn assert_joined_rows_hold(&self, text: &str) {
        let joined: String = self
            .grid()
            .rows(false)
            .iter()
            .map(|row| row.trim())
            .collect();
        assert!(joined.contains(text), "`{text}` is lost: {joined}");
    }

    /// Every row that ends with `suffix` has `text` at column `x`, and `n` rows do.
    pub fn assert_rows_ending_have_text_at(&self, suffix: &str, x: usize, text: &str, n: usize) {
        let mut seen = 0;
        for (y, row) in self.rows().iter().enumerate() {
            if row.trim_end().ends_with(suffix) {
                self.assert_text_at(x, y, text);
                seen += 1;
            }
        }
        assert_eq!(seen, n, "rows ending `{suffix}`:\n{}", self.text());
    }

    // ---- the window title ------------------------------------------------------------

    /// The window title the app wrote last ends with `tail`.
    pub fn assert_title_ends_with(&self, tail: &str) {
        let title = self.title().expect("a title");
        assert!(
            title.ends_with(tail),
            "the title is {title:?}, not ..{tail}"
        );
    }

    /// The window title the app wrote last holds no `part`.
    pub fn assert_title_lacks(&self, part: &str) {
        let title = self.title().expect("a title");
        assert!(
            !title.contains(part),
            "the title is {title:?}, with ..{part}.."
        );
    }

    /// The window title the app wrote last holds `part`.
    pub fn assert_title_has(&self, part: &str) {
        let title = self.title().expect("a title");
        assert!(
            title.contains(part),
            "the title is {title:?}, not ..{part}.."
        );
    }

    // ---- links ------------------------------------------------------------------------

    /// The links on the screen point to exactly these targets (each once, sorted).
    pub fn assert_link_targets(&self, targets: &[&str]) {
        let grid = self.grid();
        let found: std::collections::BTreeSet<String> = (0..grid.height)
            .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| grid.cell(x, y).link.as_deref().map(str::to_string))
            .collect();
        let got: Vec<&str> = found.iter().map(String::as_str).collect();
        pretty_assertions::assert_eq!(got, targets, "link targets:\n{}", self.text());
    }

    /// The cells that carry a link, read in order, spell `url`, and every link goes to it.
    pub fn assert_links_spell(&self, url: &str) {
        let grid = self.grid();
        let mut spelled = String::new();
        let mut destinations = Vec::new();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let cell = grid.cell(x, y);
                if let Some(link) = &cell.link {
                    if !cell.tail {
                        spelled.push_str(&cell.text);
                    }
                    destinations.push(link.clone());
                }
            }
        }
        pretty_assertions::assert_eq!(spelled, url, "the linked cells:\n{}", grid.text());
        assert!(
            !destinations.is_empty() && destinations.iter().all(|d| d == url),
            "links {destinations:?}"
        );
    }

    /// `len` cells from `from` columns after the start of `anchor` (the first row that holds it)
    /// are all linked to `url`, or all not (`linked`).
    pub fn assert_span_linked(
        &self,
        anchor: &str,
        from: usize,
        len: usize,
        url: &str,
        linked: bool,
    ) {
        let grid = self.grid();
        let (x, _, y) = grid
            .locate(anchor)
            .unwrap_or_else(|| panic!("no `{anchor}` row:\n{}", self.text()));
        let all = (x + from..x + from + len).all(|c| grid.cell(c, y).link.as_deref() == Some(url));
        assert_eq!(
            all,
            linked,
            "the {len} cells {from} after `{anchor}` are {}linked to {url}",
            if linked { "not " } else { "" }
        );
    }

    // ---- the wire ---------------------------------------------------------------------

    /// `n` requests of `method` have gone to the server.
    pub fn assert_requests_of(&self, method: &str, n: usize) {
        pretty_assertions::assert_eq!(
            self.requests_of(method),
            n,
            "requests of `{method}`\n{}",
            self.text()
        );
    }

    /// No request of `method` has gone to the server.
    pub fn assert_no_request(&self, method: &str) {
        self.assert_requests_of(method, 0);
    }

    /// Some request of `method` carries `needle` in its params (their JSON text).
    pub fn assert_request_mentions(&self, method: &str, needle: &str) {
        let params = self.request_params(method);
        assert!(
            params.iter().any(|p| p.to_string().contains(needle)),
            "no `{method}` mentions `{needle}`: {params:?}"
        );
    }

    /// The last request of `method` carries `needle`.
    pub fn assert_last_request_has(&self, method: &str, needle: &str) {
        let sent = self
            .request_params(method)
            .last()
            .expect("the shortcut asks the server")
            .to_string();
        assert!(sent.contains(needle), "{method} sent {sent}");
    }

    /// The last request of `method` mentions `needle` exactly when `mentioned`.
    pub fn assert_last_request_mentions(&self, method: &str, needle: &str, mentioned: bool) {
        let sent = self
            .request_params(method)
            .last()
            .expect("a request")
            .to_string();
        assert_eq!(sent.contains(needle), mentioned, "{method}: {sent}");
    }

    /// The history pages the app has asked for number `n`.
    pub fn assert_page_requests(&self, n: usize) {
        pretty_assertions::assert_eq!(
            self.page_requests().len(),
            n,
            "page requests {:?}",
            self.page_requests()
        );
    }

    /// `n` add-credits nudges went out.
    pub fn assert_nudges(&self, n: usize) {
        pretty_assertions::assert_eq!(self.nudges().len(), n, "nudges {:?}", self.nudges());
    }

    /// The pages the app opened in the browser are `urls`.
    pub fn assert_opened_urls(&self, urls: &[&str]) {
        pretty_assertions::assert_eq!(self.opened_urls(), urls);
    }

    // ---- the turns --------------------------------------------------------------------

    /// The app started exactly one turn, and it is `want`.
    pub fn assert_sole_turn(&self, want: Turn) {
        let turns = self.turns();
        assert_eq!(turns.len(), 1, "{:?}", self.commands());
        let turn = &turns[0];
        if let Some(text) = &want.text {
            pretty_assertions::assert_eq!(&turn.text, text, "the turn's text");
        }
        if let Some(remote) = &want.remote {
            pretty_assertions::assert_eq!(&turn.remote_images, remote, "the turn's remote images");
        }
        if let Some(images) = &want.images {
            assert_eq!(turn.images.len(), images.len(), "{turn:?}");
            for (got, (path, range)) in turn.images.iter().zip(images) {
                pretty_assertions::assert_eq!(&got.path, path);
                if let Some(range) = range {
                    pretty_assertions::assert_eq!(&got.range, range);
                }
            }
        }
        if let Some(skills) = &want.skills {
            let got: Vec<(String, String)> = turn
                .mentions
                .iter()
                .map(|m| match m {
                    ToolMention::Skill { name, path } => (name.clone(), path.clone()),
                    other => panic!("not a skill: {other:?}"),
                })
                .collect();
            pretty_assertions::assert_eq!(&got, skills, "the turn's skills");
        }
        if let Some(mentions) = &want.mentions {
            let got: Vec<(String, String)> = turn
                .mentions
                .iter()
                .map(|m| match m {
                    ToolMention::Mention { name, path } => (name.clone(), path.clone()),
                    other => panic!("not a mention: {other:?}"),
                })
                .collect();
            pretty_assertions::assert_eq!(&got, mentions, "the turn's mentions");
        }
    }

    // ---- what the app asked ------------------------------------------------------------

    fn screen_commands(&self) -> Vec<ScreenCommand> {
        self.commands()
            .into_iter()
            .filter_map(|c| match c {
                Command::Screen { command, .. } => Some(command),
                _ => None,
            })
            .collect()
    }

    /// The `app/list` requests the app has made, oldest first.
    pub fn asked_apps(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Apps { .. }),
                    ..
                }
            )
        })
    }

    /// The configuration-requirements reads `/permissions` has made, oldest first.
    pub fn asked_requirements(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Requirements { .. }),
                    ..
                }
            )
        })
    }

    /// The pages of permission profiles `/permissions` has asked for, oldest first.
    pub fn asked_profiles(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Profiles { .. }),
                    ..
                }
            )
        })
    }

    /// The server answers the last `/permissions` read: the `requirements` (the JSON of the
    /// requirements, null for none), then the one page of `profiles` (their JSON list).
    pub fn answer_permission_catalog(
        &mut self,
        requirements: serde_json::Value,
        profiles: serde_json::Value,
    ) {
        let reads = self.asked_requirements();
        let Some(read) = reads.last() else {
            panic!("the requirements were not asked:\n{}", self.text());
        };
        self.answer_request(read, json!({ "requirements": requirements }));
        let pages = self.asked_profiles();
        let Some(page) = pages.last() else {
            panic!("no page of profiles was asked:\n{}", self.text());
        };
        self.answer_request(page, json!({ "data": profiles, "nextCursor": null }));
    }

    /// The permission profiles the app asked the thread to change to are `profiles`, in order
    /// (the oracle's `UpdatePermissionProfile` events).
    pub fn assert_permission_updates(&self, profiles: &[&str]) {
        let got: Vec<String> = self
            .screen_commands()
            .into_iter()
            .filter_map(|c| match c {
                ScreenCommand::Select(select::Command::Settings { patch, .. }) => patch.permissions,
                _ => None,
            })
            .collect();
        pretty_assertions::assert_eq!(got, profiles, "the permission profiles asked of the thread");
    }

    /// The app asked the thread to change its settings once, to the reviewer
    /// (`approvals_reviewer` as the wire spells it: `auto_review` or `user`) and the permission profile `profile` (the
    /// oracle's override turn context).
    pub fn assert_one_settings_patch(&self, reviewer: &str, profile: &str) {
        let patches: Vec<_> = self
            .screen_commands()
            .into_iter()
            .filter_map(|c| match c {
                ScreenCommand::Select(select::Command::Settings { patch, .. }) => Some(patch),
                _ => None,
            })
            .collect();
        pretty_assertions::assert_eq!(patches.len(), 1, "{patches:?}");
        pretty_assertions::assert_eq!(
            serde_json::to_value(patches[0].approvals_reviewer).ok(),
            Some(json!(reviewer)),
            "the approvals reviewer of the patch"
        );
        pretty_assertions::assert_eq!(patches[0].permissions.as_deref(), Some(profile));
    }

    /// No turn with the text `text` has been started.
    pub fn assert_no_turn_text(&self, text: &str) {
        let started = self.turn_texts();
        assert!(
            !started.iter().any(|started| started == text),
            "the turn `{text}` was started: {started:?}"
        );
    }

    /// The `model/list` requests the app has made, oldest first.
    pub fn asked_models(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Models { .. }),
                    ..
                }
            )
        })
    }

    /// The git-log asks of the review picker the host has not answered, oldest first.
    pub fn asked_git_logs(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Effect {
                    ask: Effect::GitLog { .. },
                    ..
                }
            )
        })
    }

    /// The git-branches asks of the review picker the host has not answered, oldest first.
    pub fn asked_git_branches(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Effect {
                    ask: Effect::GitBranches { .. },
                    ..
                }
            )
        })
    }

    /// The `turn/steer` requests the app has made and the server has not answered, oldest first.
    pub fn asked_steers(&self) -> Vec<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Chat(chat::Command::Steer { .. }),
                    ..
                }
            )
        })
    }

    /// The title suggestion `/rename` asked of the structured-turn door: the run's first request
    /// (the MCP servers of the config), none when the conversation had nothing to name the
    /// thread by.
    pub fn asked_title(&self) -> Option<Ask> {
        self.thread_request(|ask| matches!(ask, ThreadAsk::McpServers { .. }))
    }

    /// The first request of the thread asks `which` picks.
    fn thread_request(&self, which: impl Fn(&ThreadAsk) -> bool) -> Option<Ask> {
        self.asked_of(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                    ..
                } if which(ask)
            )
        })
        .into_iter()
        .next()
    }

    /// The host's answer to the latest export write `/export` asked for: `Ok(path)` saved,
    /// `Err(why)` failed.
    pub fn answer_export(&mut self, result: Result<&str, &str>) {
        let done = Done::Export(result.map(str::to_owned).map_err(str::to_owned));
        self.answer_latest_effect(|ask| matches!(ask, Effect::Export { .. }), done);
    }

    /// The host's answer to the latest clipboard copy asked for: `Ok(())` confirmed,
    /// `Err(why)` refused.
    pub fn answer_copy(&mut self, result: Result<(), &str>) {
        let done = Done::Copy(
            result
                .map(|()| codex_tui_core::app::effects::CopyStatus::Confirmed)
                .map_err(str::to_owned),
        );
        self.answer_latest_effect(|ask| matches!(ask, Effect::Copy { .. }), done);
    }

    fn answer_latest_effect(&mut self, pick: impl Fn(&Effect) -> bool, done: Done) {
        let command = self
            .commands()
            .into_iter()
            .rev()
            .find(|c| matches!(c, Command::Effect { ask, .. } if pick(ask)))
            .expect("the effect was asked");
        self.answer_effect(&command, done);
    }

    /// The skills (`name`, `path`) turn `n` binds, its other mentions left out.
    pub fn turn_skills(&self, n: usize) -> Vec<(String, String)> {
        self.turns()[n]
            .mentions
            .iter()
            .filter_map(|m| match m {
                ToolMention::Skill { name, path } => Some((name.clone(), path.clone())),
                _ => None,
            })
            .collect()
    }

    /// The results the app answered server requests with, oldest first.
    pub fn resolved_requests(&self) -> Vec<serde_json::Value> {
        self.commands()
            .into_iter()
            .filter_map(|c| match c {
                Command::ResolveRequest { result, .. } => Some(result),
                _ => None,
            })
            .collect()
    }

    fn asked_of(&self, which: impl Fn(&Command) -> bool) -> Vec<Ask> {
        self.commands().into_iter().filter(|c| which(c)).collect()
    }

    /// The server walks the title suggestion `/rename` asked for to its end (the oracle's
    /// `generate_thread_title`): the config is read, the temporary thread starts read-only, its
    /// turn starts, and the model says `title` (none: an answer that is no title). A thread with
    /// no conversation asked for nothing, and `None` is the only answer it takes.
    pub fn answer_title(&mut self, title: Option<&str>) {
        let Some(config) = self.asked_title() else {
            assert!(title.is_none(), "the rename prompt asked for a title");
            return;
        };
        self.answer_request(&config, json!({"config": {}, "origins": {}}));
        let start = self
            .thread_request(|ask| matches!(ask, ThreadAsk::StartTemporary(_)))
            .expect("the temporary thread starts once the config is read");
        let thread = json!({
            "id": "title-1", "environments": null, "extra": null, "sessionId": "title-1",
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": true, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
            "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/tmp/project",
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
            "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
            "turns": []
        });
        self.answer_request(
            &start,
            json!({"thread": thread, "model": "gpt-5", "modelProvider": "openai",
                "serviceTier": null, "cwd": "/tmp/project", "instructionSources": [],
                "approvalPolicy": "never", "approvalsReviewer": "user",
                "sandbox": {"type": "readOnly", "networkAccess": false},
                "reasoningEffort": null}),
        );
        let turn = self
            .thread_request(|ask| matches!(ask, ThreadAsk::StructuredTurn { .. }))
            .expect("the turn starts on the temporary thread");
        self.answer_request(
            &turn,
            json!({"turn": {"id": "title-turn", "items": [], "itemsView": "summary",
                "status": "inProgress", "error": null}}),
        );
        let said = title.map_or_else(
            || "no title".to_owned(),
            |t| json!({"title": t}).to_string(),
        );
        self.notify(json!({"method": "item/completed", "params": {
            "threadId": "title-1", "turnId": "title-turn", "completedAtMs": 0,
            "startedAtMs": 0,
            "item": {"type": "agentMessage", "id": "m1", "text": said, "phase": null}}}));
        self.notify(json!({"method": "turn/completed", "params": {
            "threadId": "title-1", "turn": {"id": "title-turn", "items": [],
            "itemsView": "summary", "status": "completed", "error": null}}}));
    }

    /// The server's stream says `message` (a notification's JSON).
    fn notify(&mut self, message: serde_json::Value) {
        let notification: ServerNotification =
            serde_json::from_value(message).expect("a notification");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(
            epoch,
            AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        );
        self.h.advance(100);
    }

    /// The host answers the git log `ask` with the commits `(sha, subject)`.
    pub fn answer_git_log(&mut self, ask: &Ask, commits: &[(&str, &str)]) {
        let log = commits
            .iter()
            .map(|(sha, subject)| Commit {
                sha: (*sha).into(),
                subject: (*subject).into(),
            })
            .collect();
        self.answer_effect(ask, Done::GitLog(log));
    }

    /// The host answers the git branches `ask` with the branch checked out (none: detached HEAD,
    /// or no repository) and the local branches.
    pub fn answer_git_branches(&mut self, ask: &Ask, current: Option<&str>, branches: &[&str]) {
        self.answer_effect(
            ask,
            Done::GitBranches {
                current: current.map(Into::into),
                branches: branches.iter().map(|b| (*b).into()).collect(),
            },
        );
    }

    /// `n` commands have been given so far (a mark for `assert_no_models_asked_since`).
    pub fn command_count(&self) -> usize {
        self.commands().len()
    }

    /// No model list was asked of the server after the first `mark` commands.
    pub fn assert_no_models_asked_since(&self, mark: usize) {
        let new = self.commands().split_off(mark);
        assert!(
            new.iter().all(|c| !matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Models { .. }),
                    ..
                }
            )),
            "a model list was asked: {new:?}"
        );
    }

    /// The model the app asked the thread to switch to, in order.
    pub fn assert_model_updates(&self, models: &[&str]) {
        let got: Vec<String> = self
            .screen_commands()
            .into_iter()
            .filter_map(|c| match c {
                ScreenCommand::Select(select::Command::Settings { patch, .. }) => patch.model,
                _ => None,
            })
            .collect();
        pretty_assertions::assert_eq!(got, models);
    }

    /// No shell command and no compaction was sent by the chat.
    pub fn assert_no_shell_or_compact(&self) {
        let sent = self.screen_commands().into_iter().any(|c| {
            matches!(
                c,
                ScreenCommand::Chat(chat::Command::Shell(_) | chat::Command::Compact)
            )
        });
        assert!(
            !sent,
            "a shell or compact command went out: {:?}",
            self.commands()
        );
    }

    /// No command of the chat whose Debug text holds `Shell` went out.
    pub fn assert_no_shell_command(&self) {
        assert!(
            !self
                .commands()
                .iter()
                .any(|c| format!("{c:?}").contains("Shell")),
            "{:?}",
            self.commands()
        );
    }

    /// The chat asked the thread to clean its background terminals.
    pub fn assert_clean_terminals_sent(&self) {
        assert!(
            self.screen_commands()
                .iter()
                .any(|c| matches!(c, ScreenCommand::Chat(chat::Command::CleanTerminals))),
            "the cleanup went out: {:?}",
            self.commands()
        );
    }

    /// `n` archive asks have gone to the agents backend.
    pub fn assert_archives(&self, n: usize) {
        let got = self
            .screen_commands()
            .iter()
            .filter(|c| matches!(c, ScreenCommand::Agents(agents::Command::Archive { .. })))
            .count();
        pretty_assertions::assert_eq!(got, n, "archives asked");
    }

    /// `n` delete asks have gone to the agents backend.
    pub fn assert_deletes(&self, n: usize) {
        let got = self
            .screen_commands()
            .iter()
            .filter(|c| matches!(c, ScreenCommand::Agents(agents::Command::Delete { .. })))
            .count();
        pretty_assertions::assert_eq!(got, n, "deletes asked");
    }

    /// The chat was asked to review exactly this commit, once.
    pub fn assert_review_of_commit(&self, sha: &str, title: &str) {
        let got: Vec<ReviewTarget> = self
            .screen_commands()
            .into_iter()
            .filter_map(|c| match c {
                ScreenCommand::Chat(chat::Command::Review(target)) => Some(target),
                _ => None,
            })
            .collect();
        pretty_assertions::assert_eq!(
            got,
            [ReviewTarget::Commit {
                sha: sha.into(),
                title: Some(title.into())
            }]
        );
    }

    /// `n` `app/list` requests have been made.
    pub fn assert_apps_asked(&self, n: usize) {
        let asked = self.asked_apps();
        assert_eq!(asked.len(), n, "{asked:?}");
    }

    /// `n` git-log asks of the review picker have been made.
    pub fn assert_git_logs_asked(&self, n: usize) {
        let asked = self.asked_git_logs();
        assert_eq!(asked.len(), n, "{asked:?}");
    }

    /// The names the app asked the server to give the thread (`Op::SetThreadName`) are `names`.
    pub fn assert_set_names(&self, names: &[&str]) {
        let got: Vec<String> = self
            .commands_named("Feature")
            .iter()
            .filter_map(|command| {
                let rest = command.split("SetName(\"").nth(1)?;
                Some(rest.split("\")").next()?.to_owned())
            })
            .collect();
        pretty_assertions::assert_eq!(got, names);
    }

    // ---- the composer ------------------------------------------------------------------

    /// Pastes one character more than the composer takes (`MAX_INPUT_CHARS`).
    pub fn paste_over_the_input_limit(&mut self) -> &mut Screen {
        let text = "x".repeat(codex_tui_core::screens::composer::MAX_INPUT_CHARS + 1);
        self.paste(&text)
    }
}

/// The glyphs of the sparkle's dots (`sparkle_field.rs`, `DOTS`).
pub const SPARKLE_DOTS: [char; 8] = ['⠁', '⠂', '⠄', '⠈', '⠐', '⠠', '⡀', '⢀'];

impl Screen {
    /// A dot of the starfield is on the screen exactly when `shown`.
    pub fn assert_dots(&self, shown: bool) {
        let on = self.text().chars().any(|c| SPARKLE_DOTS.contains(&c));
        assert_eq!(
            on,
            shown,
            "the starfield dots (want shown: {shown}):\n{}",
            self.text()
        );
    }
}

fn row_then(rows: &[String], screen: &str, row: &str, following: &[&str]) {
    let Some(at) = rows.iter().position(|r| r == row.trim_end()) else {
        panic!("no `{row}` row:\n{screen}");
    };
    for (n, want) in following.iter().enumerate() {
        let got = rows.get(at + 1 + n).map_or("", String::as_str);
        pretty_assertions::assert_eq!(
            got,
            want.trim_end(),
            "the row {} after `{row}`:\n{screen}",
            n + 1
        );
    }
}

/// The rows `got` read exactly `want` (trailing blanks aside).
pub fn assert_rows_are(got: &[String], want: &[&str]) {
    let got: Vec<&str> = got.iter().map(|row| row.trim_end()).collect();
    let want: Vec<&str> = want.iter().map(|row| row.trim_end()).collect();
    pretty_assertions::assert_eq!(got, want);
}
