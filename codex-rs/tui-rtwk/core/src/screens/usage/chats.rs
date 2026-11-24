//! The Chats report: recent local chats with a backend estimate (a business
//! account: lifetime credits per chat, oracle `chats.rs` + `chat_panel.rs`)
//! or with recorded task usage (a consumer account: allowance percentages and
//! balance debits, oracle `tasks.rs` + `task_panel.rs`). `ChatList`/`TaskList`
//! are what the host answers with (the backend's `ThreadUsage`/`TaskUsage`
//! reduced to what is shown); `chat_view`/`task_view` are the rows the page
//! lays out: every choice that is not the room's (which rows, which groups,
//! the sort, the selection, the details) is made here; the words are the
//! view's (`Title`, `Metric` and the lines name text, they hold none).

use std::cmp::Ordering;

use super::data::credits;
use super::{Model, Section, View};

/// How many rows the consumer card and a truncated listing keep (oracle:
/// `tasks.rs` `truncate(100)`, `task_panel.rs` `take(5)`).
pub const CARD_ROWS: usize = 5;

/// The widest a report's rows go (oracle: `clamp` in `chat_lines` and
/// `task_lines`); the text wraps one cell short of it.
pub const BUSINESS_ROW_MAX: usize = 97;
pub const CONSUMER_ROW_MAX: usize = 111;

/// From this width the breakdown of a business chat is a table, below it
/// each group takes two rows; the dollar column stacks under it as well.
pub const TABLE_FROM: usize = 70;

/// The fixed columns of a business chat's rows.
pub const USD_COLUMN: usize = 12;
pub const CREDITS_COLUMN: usize = 16;
pub const TAG_COLUMN: usize = 12;

/// The room consumer metrics need beside the title before they all show.
pub const LEFT_ROOM: usize = 24;

// ---- the business account's chats ----

/// One model/effort/speed group of a chat's estimate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct GroupData {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub speed: Option<String>,
    pub credits_micros: i64,
}

/// The backend's estimate of a chat (millionths of credits and dollars).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Estimate {
    pub credits_micros: i64,
    #[serde(default)]
    pub usd_micros: Option<i64>,
    #[serde(default)]
    pub groups: Vec<GroupData>,
}

/// A recent root chat; the estimate is missing when the backend gave none.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ChatRow {
    pub title: String,
    #[serde(default)]
    pub estimate: Option<Estimate>,
}

/// The listing, sorted by lifetime credits (the host sorts, as the oracle
/// does before it shows).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ChatList {
    pub rows: Vec<ChatRow>,
}

// ---- the consumer account's tasks ----

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Available,
    Partial,
    Unavailable,
}

/// A decimal the backend sent as text: the digits are kept for showing and
/// for sorting (oracle: the backend client's `TaskCredits`, carried with its
/// order).
#[derive(Clone, Debug, Eq, serde::Deserialize)]
#[serde(transparent)]
pub struct Decimal(String);

impl Decimal {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parts(&self) -> (bool, i64, String) {
        let (mantissa, exponent) = self.0.split_once(['e', 'E']).unwrap_or((&self.0, "0"));
        let negative = mantissa.starts_with('-');
        let unsigned = mantissa.trim_start_matches(['-', '+']);
        let fraction = unsigned
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        let digits = unsigned.replace('.', "");
        let digits = digits.trim_start_matches('0');
        let power =
            exponent.parse::<i64>().unwrap_or_default() - fraction as i64 + digits.len() as i64;
        (negative && !digits.is_empty(), power, digits.to_string())
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let (negative, power, digits) = self.parts();
        let (other_negative, other_power, other_digits) = other.parts();
        let magnitude = match (digits.is_empty(), other_digits.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => power.cmp(&other_power).then_with(|| {
                let width = digits.len().max(other_digits.len());
                digits
                    .bytes()
                    .chain(std::iter::repeat(b'0'))
                    .take(width)
                    .cmp(
                        other_digits
                            .bytes()
                            .chain(std::iter::repeat(b'0'))
                            .take(width),
                    )
            }),
        };
        other_negative.cmp(&negative).then(if negative {
            magnitude.reverse()
        } else {
            magnitude
        })
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

/// The three amounts a task or a group reports (the limit percentages are
/// finite: the host takes none else).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct Amounts {
    #[serde(default)]
    pub weekly_percent: Option<f64>,
    #[serde(default)]
    pub five_hour_percent: Option<f64>,
    #[serde(default)]
    pub balance: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct TaskGroup {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub speed: Option<String>,
    pub product: Option<String>,
    #[serde(default)]
    pub amounts: Amounts,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Task {
    pub status: Status,
    #[serde(default)]
    pub amounts: Amounts,
    #[serde(default)]
    pub groups: Vec<TaskGroup>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct TaskRow {
    pub title: String,
    #[serde(default)]
    pub task: Option<Task>,
}

/// The listing with what the listing left out and when the data is from.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct TaskList {
    pub rows: Vec<TaskRow>,
    #[serde(default)]
    pub truncated: bool,
    /// Unix seconds, UTC; none when any batch had no freshness.
    #[serde(default)]
    pub updated_at: Option<i64>,
}

// The percentages were checked finite where the host made them.
impl Eq for Amounts {}
impl Eq for TaskGroup {}
impl Eq for Task {}
impl Eq for TaskRow {}
impl Eq for TaskList {}

/// What a task row is sorted and shown by (oracle: `METRICS`); the index is
/// `Model::chat_metric`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Weekly,
    FiveHour,
    Balance,
}

impl Metric {
    pub const ALL: [Self; 3] = [Self::Weekly, Self::FiveHour, Self::Balance];

    pub fn index(self) -> usize {
        self as usize
    }
}

impl Amounts {
    fn percent(&self, metric: Metric) -> Option<f64> {
        match metric {
            Metric::Weekly => self.weekly_percent,
            _ => self.five_hour_percent,
        }
    }

    /// The amount as shown (oracle: `task_panel::amount`): none is missing.
    pub fn shown(&self, metric: Metric) -> Option<String> {
        if metric == Metric::Balance {
            return self.balance.as_ref().map(|value| balance_text(value.as_str()));
        }
        self.percent(metric).map(|value| format!("{value:.1}%"))
    }
}

/// A balance in credits with two decimals, from the digits the backend sent;
/// what is too small or too large to round keeps its own text (oracle:
/// `task_panel::amount`).
pub fn balance_text(value: &str) -> String {
    let (mantissa, exponent) = value.split_once(['e', 'E']).unwrap_or((value, "0"));
    let fraction = mantissa
        .split_once('.')
        .map_or("", |(_, fraction)| fraction);
    let digits = mantissa.trim_start_matches(['+', '-']).replace('.', "");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return "0.00".to_string();
    }
    let cent_digits = digits.len() as i64 + exponent.parse::<i32>().unwrap_or_default() as i64
        - fraction.len() as i64
        + 2;
    // Retain tiny adjustments and bound the integer conversion before padding.
    if !(1..=18).contains(&cent_digits) {
        return value.to_string();
    }
    let mut cents = digits
        .bytes()
        .chain(std::iter::repeat(b'0'))
        .take(cent_digits as usize)
        .fold(0_i64, |cents, digit| cents * 10 + i64::from(digit - b'0'));
    if digits
        .as_bytes()
        .get(cent_digits as usize)
        .is_some_and(|digit| *digit >= b'5')
    {
        cents += 1;
    }
    cents
        .checked_mul(10_000)
        .map(|micros| credits(if mantissa.starts_with('-') { -micros } else { micros }))
        .unwrap_or_else(|| value.to_string())
}

fn compare(a: Option<&Amounts>, b: Option<&Amounts>, metric: Metric) -> Ordering {
    if metric == Metric::Balance {
        a.and_then(|a| a.balance.as_ref())
            .cmp(&b.and_then(|b| b.balance.as_ref()))
    } else {
        a.and_then(|a| a.percent(metric))
            .partial_cmp(&b.and_then(|b| b.percent(metric)))
            .unwrap_or(Ordering::Equal)
    }
}

impl TaskRow {
    /// The amounts, unless the backend said they are unavailable (then the
    /// title proves no ownership either).
    pub fn available(&self) -> Option<&Amounts> {
        self.task
            .as_ref()
            .filter(|task| task.status != Status::Unavailable)
            .map(|task| &task.amounts)
    }

    fn partial(&self) -> bool {
        self.task
            .as_ref()
            .is_some_and(|task| task.status == Status::Partial)
    }

    fn title(&self) -> Title {
        if self.available().is_some() {
            Title::Text(self.title.clone())
        } else {
            Title::Unavailable
        }
    }
}

impl ChatRow {
    fn title(&self) -> Title {
        if self.estimate.is_some() {
            Title::Text(self.title.clone())
        } else {
            Title::Unavailable
        }
    }
}

/// A chat's title as shown: local history can span accounts, so a title
/// shows only where the account's estimate came back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Title {
    Text(String),
    Unavailable,
}

// ---- where the keys are ----

/// The cursor row and the row whose details are open. Moving off a row
/// closes it (oracle: `handle_key`), so an open row is the one under the
/// cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pos {
    pub cursor: usize,
    pub detail: Option<usize>,
    /// `a`: the details list the groups that cost nothing as well.
    pub zero_groups: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatStep {
    Up,
    Down,
    Top,
    Bottom,
    /// Enter and Right: open or close the details of the row.
    Toggle,
    /// Left: close the details.
    Collapse,
    /// Esc: close the details, else the page.
    Back,
}

impl Pos {
    /// The cursor kept inside `rows` (the listing may have changed).
    pub fn at(&self, rows: usize) -> usize {
        self.cursor.min(rows.saturating_sub(1))
    }

    /// One key; whether the page closes. `has_details`: the row under the
    /// cursor has an estimate to open.
    pub fn step(&mut self, step: ChatStep, rows: usize, has_details: bool) -> bool {
        let before = self.at(rows);
        self.cursor = before;
        let last = rows.max(1) - 1;
        match step {
            ChatStep::Up => self.cursor = before.saturating_sub(1),
            ChatStep::Down => self.cursor = (before + 1).min(last),
            ChatStep::Top => self.cursor = 0,
            ChatStep::Bottom => self.cursor = last,
            ChatStep::Toggle => {
                if !has_details {
                    return false;
                }
                self.detail = if self.detail == Some(before) {
                    None
                } else {
                    Some(before)
                };
            }
            ChatStep::Collapse => self.detail = None,
            ChatStep::Back => return self.detail.take().is_none(),
        }
        if self.cursor != before {
            self.detail = None;
        }
        false
    }

    /// A tap on a row: the first moves the cursor to it, the next opens its
    /// details, the one after closes them.
    pub fn pick(&mut self, index: usize, rows: usize, has_details: bool) {
        if index >= rows {
            return;
        }
        if self.at(rows) == index {
            self.step(ChatStep::Toggle, rows, has_details);
        } else {
            self.cursor = index;
            self.detail = None;
        }
    }
}

// ---- the rows the page lays out ----

/// What a business chat's row shows at its right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Amount {
    /// The credits (none: the estimate is missing).
    Credits(Option<String>),
    /// A dollar figure beside the credits (some chat has one).
    Both {
        usd: Option<String>,
        credits: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatLine {
    /// "Active in past 30 days, sorted by lifetime credits".
    Blurb,
    /// The load's own note while the listing is not there.
    Message(super::Note),
    NoRecent,
    /// Chats without an estimate.
    Missing(usize),
    Blank,
    /// The column heads; `usd`: the dollar column is there.
    Head {
        usd: bool,
    },
    Row {
        index: usize,
        selected: bool,
        title: Title,
        amount: Amount,
    },
    /// The line under an open row.
    Rule,
    /// The open row's full title (the page shows it where the row cut it).
    LongTitle(Title),
    GroupHead,
    Group {
        model: Option<String>,
        effort: Option<String>,
        speed: Option<String>,
        credits: String,
    },
    NoBreakdown,
    Zeros {
        count: usize,
        shown: bool,
    },
    /// "Showing a-b of n chats".
    Showing {
        first: usize,
        last: usize,
        count: usize,
    },
    Coverage,
    /// The card's first line.
    CardBlurb,
    /// A chat on the card: its title and its credits (none: missing).
    CardRow {
        title: Title,
        credits: Option<String>,
    },
    CardCoverage,
    /// Some chat has no estimate.
    CardMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskLine {
    Blurb(Metric),
    /// "Credits debited from the balance" or "Recorded usage / current limit".
    Basis {
        balance: bool,
    },
    /// The listing is not there: the load's note, or the plain empty text.
    Message(Option<super::Note>),
    NoRecent,
    Partial {
        partial: usize,
        missing: usize,
    },
    Blank,
    /// The columns offered, the sort metric among them.
    Head {
        metrics: Vec<Metric>,
        metric: Metric,
        /// The widest cell of each metric offered over every row (the
        /// card shows the first rows only, the columns fit all of them).
        widest: Vec<usize>,
    },
    Row {
        index: usize,
        selected: bool,
        title: Title,
        partial: bool,
        /// The cell of each metric offered; none is missing.
        cells: Vec<Option<String>>,
    },
    Rule,
    /// The open row's title.
    DetailTitle(Title),
    Cell {
        metric: Metric,
        value: Option<String>,
    },
    GroupHead(Metric),
    Group {
        model: Option<String>,
        effort: Option<String>,
        speed: Option<String>,
        product: Option<String>,
        value: Option<String>,
    },
    Unavailable,
    /// "Showing top n of m chats" (the card).
    ShowingTop {
        shown: usize,
        count: usize,
    },
    Showing {
        first: usize,
        last: usize,
        count: usize,
    },
    Coverage,
    /// Only the 100 most recent chats were asked.
    Truncated,
    Updated(i64),
}

impl Model {
    pub fn business_chats(&self) -> Option<&ChatList> {
        self.chats.ready().map(|list| &**list)
    }

    /// The metrics a consumer account's rows can be sorted by: the balance
    /// always, a percentage once some row reports it.
    pub fn task_metrics(&self) -> Vec<Metric> {
        Metric::ALL
            .into_iter()
            .filter(|metric| {
                *metric == Metric::Balance
                    || self.tasks.ready().is_some_and(|list| {
                        list.rows.iter().any(|row| {
                            row.available()
                                .is_some_and(|amounts| amounts.percent(*metric).is_some())
                        })
                    })
            })
            .collect()
    }

    /// The rows the keys move over in the Chats report.
    pub fn chat_row_count(&self) -> usize {
        if self.business() {
            self.business_chats().map_or(0, |list| list.rows.len())
        } else {
            self.tasks.ready().map_or(0, |list| list.rows.len())
        }
    }
}

impl View<'_> {
    /// The rows by the sort metric, the largest first (oracle: `task_rows`).
    pub fn task_rows(&self) -> Vec<&TaskRow> {
        let mut rows = self
            .tasks
            .ready()
            .map(|list| list.rows.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let metric = self.task_metric();
        rows.sort_by(|a, b| compare(b.available(), a.available(), metric));
        rows
    }

    /// The row under the cursor has details to open (oracle:
    /// `chat_has_details`).
    pub fn chat_has_details(&self, pos: &Pos) -> bool {
        let at = pos.at(self.chat_row_count());
        if self.business() {
            self.business_chats()
                .and_then(|list| list.rows.get(at))
                .is_some_and(|row| row.estimate.is_some())
        } else {
            self.task_rows()
                .get(at)
                .is_some_and(|row| row.available().is_some())
        }
    }

    /// The business account's report (oracle: `chat_lines`).
    pub fn chat_view(&self, pos: &Pos) -> Vec<ChatLine> {
        let mut lines = vec![ChatLine::Blurb];
        if let Some(note) = self.chats.note() {
            lines.push(ChatLine::Message(note));
        }
        let Some(list) = self.business_chats() else {
            return lines;
        };
        if list.rows.is_empty() {
            lines.push(ChatLine::NoRecent);
            return lines;
        }
        let missing = list.rows.iter().filter(|row| row.estimate.is_none()).count();
        if missing > 0 {
            lines.push(ChatLine::Missing(missing));
        }
        lines.push(ChatLine::Blank);
        let usd = list
            .rows
            .iter()
            .any(|row| row.estimate.as_ref().is_some_and(|e| e.usd_micros.is_some()));
        lines.push(ChatLine::Head { usd });
        let cursor = pos.at(list.rows.len());
        for (index, row) in list.rows.iter().enumerate() {
            let selected = self.sel.section == Section::Chats && index == cursor;
            let text = row
                .estimate
                .as_ref()
                .map(|estimate| credits(estimate.credits_micros));
            let amount = if usd {
                Amount::Both {
                    usd: row
                        .estimate
                        .as_ref()
                        .and_then(|estimate| estimate.usd_micros)
                        .map(|micros| format!("${:.2}", micros as f64 / 1_000_000.0)),
                    credits: text,
                }
            } else {
                Amount::Credits(text)
            };
            lines.push(ChatLine::Row {
                index,
                selected,
                title: row.title(),
                amount,
            });
            if pos.detail == Some(index)
                && let Some(estimate) = &row.estimate
            {
                self.chat_details(row, estimate, pos.zero_groups, &mut lines);
            }
        }
        lines.push(ChatLine::Blank);
        lines.push(ChatLine::Showing {
            first: 1,
            last: list.rows.len(),
            count: list.rows.len(),
        });
        lines.push(ChatLine::Coverage);
        lines
    }

    /// The business account's card (oracle: the dashboard's Chats card): the
    /// first rows by lifetime credits.
    pub fn chat_card(&self) -> Vec<ChatLine> {
        let mut lines = vec![ChatLine::CardBlurb, ChatLine::Blank];
        if let Some(list) = self.business_chats() {
            if list.rows.is_empty() {
                lines.push(ChatLine::NoRecent);
            }
            for row in list.rows.iter().take(CARD_ROWS) {
                lines.push(ChatLine::CardRow {
                    title: row.title(),
                    credits: row
                        .estimate
                        .as_ref()
                        .map(|estimate| credits(estimate.credits_micros)),
                });
            }
            lines.push(ChatLine::Blank);
            lines.push(ChatLine::CardCoverage);
            if list.rows.iter().any(|row| row.estimate.is_none()) {
                lines.push(ChatLine::CardMissing);
            }
        } else if let Some(note) = self.chats.note() {
            lines.push(ChatLine::Message(note));
        }
        lines
    }

    fn chat_details(
        &self,
        row: &ChatRow,
        estimate: &Estimate,
        zero_groups: bool,
        lines: &mut Vec<ChatLine>,
    ) {
        lines.push(ChatLine::Rule);
        lines.push(ChatLine::LongTitle(row.title()));
        let zeros = estimate
            .groups
            .iter()
            .filter(|group| group.credits_micros == 0)
            .count();
        let mut groups = estimate
            .groups
            .iter()
            .filter(|group| zero_groups || group.credits_micros != 0)
            .collect::<Vec<_>>();
        groups.sort_by_key(|group| std::cmp::Reverse(group.credits_micros));
        if !groups.is_empty() {
            lines.push(ChatLine::GroupHead);
        }
        for group in groups {
            lines.push(ChatLine::Group {
                model: group.model.as_deref().map(|model| self.model_name(model).to_owned()),
                effort: group.effort.clone(),
                speed: group.speed.clone(),
                credits: credits(group.credits_micros),
            });
        }
        if estimate.groups.is_empty() {
            lines.push(ChatLine::NoBreakdown);
        }
        if zeros > 0 {
            lines.push(ChatLine::Zeros {
                count: zeros,
                shown: zero_groups,
            });
        }
        lines.push(ChatLine::Blank);
    }

    /// The consumer account's report, or its card (`zoomed` false: the first
    /// rows only, no details; oracle: `task_lines`).
    pub fn task_view(&self, pos: &Pos, zoomed: bool) -> Vec<TaskLine> {
        let Some(list) = self.tasks.ready() else {
            return vec![TaskLine::Message(self.tasks.note())];
        };
        let metric = self.task_metric();
        let mut lines = vec![
            TaskLine::Blurb(metric),
            TaskLine::Basis {
                balance: metric == Metric::Balance,
            },
        ];
        if list.rows.is_empty() {
            lines.push(TaskLine::NoRecent);
            return lines;
        }
        let missing = list
            .rows
            .iter()
            .filter(|row| row.available().and_then(|a| a.shown(metric)).is_none())
            .count();
        let partial = list.rows.iter().filter(|row| row.partial()).count();
        if missing + partial > 0 {
            lines.push(TaskLine::Partial { partial, missing });
        }
        let metrics = self.task_metrics();
        lines.push(TaskLine::Blank);
        let rows = self.task_rows();
        let widest = metrics
            .iter()
            .map(|metric| {
                rows.iter()
                    .map(|row| {
                        row.available()
                            .and_then(|amounts| amounts.shown(*metric))
                            .map_or(1, |cell| cell.chars().count())
                    })
                    .max()
                    .unwrap_or(1)
            })
            .collect();
        lines.push(TaskLine::Head {
            metrics: metrics.clone(),
            metric,
            widest,
        });
        let count = rows.len();
        let cursor = pos.at(count);
        let mut body = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            body.push(TaskLine::Row {
                index,
                selected: self.sel.section == Section::Chats && index == cursor,
                title: row.title(),
                partial: row.partial(),
                cells: metrics
                    .iter()
                    .map(|metric| row.available().and_then(|a| a.shown(*metric)))
                    .collect(),
            });
            if zoomed && pos.detail == Some(index) {
                self.task_details(row, &metrics, metric, &mut body);
            }
            // The card keeps its first rows only.
            if !zoomed && index + 1 == CARD_ROWS {
                break;
            }
        }
        if zoomed {
            lines.extend(body);
            lines.push(TaskLine::Blank);
            lines.push(TaskLine::Showing {
                first: 1,
                last: count,
                count,
            });
        } else {
            lines.extend(body);
            lines.push(TaskLine::Blank);
            lines.push(TaskLine::ShowingTop {
                shown: count.min(CARD_ROWS),
                count,
            });
        }
        lines.push(TaskLine::Coverage);
        if list.truncated {
            lines.push(TaskLine::Truncated);
        }
        if let Some(at) = list.updated_at {
            lines.push(TaskLine::Updated(at));
        }
        lines
    }

    fn task_details(
        &self,
        row: &TaskRow,
        metrics: &[Metric],
        metric: Metric,
        lines: &mut Vec<TaskLine>,
    ) {
        lines.push(TaskLine::Rule);
        lines.push(TaskLine::DetailTitle(row.title()));
        for shown in metrics {
            lines.push(TaskLine::Cell {
                metric: *shown,
                value: row.available().and_then(|a| a.shown(*shown)),
            });
        }
        match &row.task {
            Some(task) => {
                if task.status != Status::Unavailable && !task.groups.is_empty() {
                    let mut groups = task.groups.iter().collect::<Vec<_>>();
                    groups.sort_by(|a, b| compare(Some(&b.amounts), Some(&a.amounts), metric));
                    lines.push(TaskLine::Blank);
                    lines.push(TaskLine::GroupHead(metric));
                    for group in groups {
                        lines.push(TaskLine::Group {
                            model: group
                                .model
                                .as_deref()
                                .map(|model| self.model_name(model).to_owned()),
                            effort: group.effort.clone(),
                            speed: group.speed.clone(),
                            product: group.product.clone(),
                            value: group.amounts.shown(metric),
                        });
                    }
                }
            }
            None => lines.push(TaskLine::Unavailable),
        }
        lines.push(TaskLine::Blank);
    }
}
