//! The details pane.

use std::borrow::Cow;
use std::sync::Arc;

use codex_tui_core::app::facts::{Rendering, TuiSettings};
use codex_tui_core::data::links::web_destination;
use rtwk::provider::Provider;
use rtwk_rich::markdown::{Glyphs, LinkDestinations, markdown_blocks_lines_with_destinations};

use crate::app::theme::Tokens;
use crate::pages::session::cells::markdown::{keep_tables, plain_lists, preferred};
use crate::widgets::markdown;

use super::list::status;
use super::*;

/// The rows a text of the details takes at the pane's width.
fn flow(seg: Measure, text: &str, style: &Style) -> Vec<Line> {
    soft::rows(seg, text, DETAILS_WIDTH)
        .into_iter()
        .map(|row| vec![span(row, style)])
        .collect()
}

/// Native Markdown rows let the pane replace a whole row at its clipping
/// boundary and keep the destinations of the surviving spans.
/// A formula is drawn when the user's `tui.rendering.math` says so (`rendering.math`).
fn markdown_lines(
    cx: &mut BuildCx<'_>,
    source: &str,
    cwd: &str,
    rendering: Rendering,
) -> Vec<Line> {
    let (lists, math) = (rendering.lists, rendering.math);
    let mut stream = markdown::stream(source, math);
    let blocks = stream.blocks().to_vec();
    let definitions = stream.definitions();
    let blocks = if lists { blocks } else { plain_lists(&blocks) };
    markdown_blocks_lines_with_destinations(
        &segmenter_of(cx),
        &markdown::prepared(&blocks, definitions, Some(cwd)),
        definitions,
        DETAILS_WIDTH,
        &Tokens::of(cx).transcript_markdown(cx),
        &Glyphs::UNICODE,
        true,
        // The Agent Center uses the oracle's width-and-cwd renderer, which
        // always writes destinations even when the terminal supports OSC 8.
        LinkDestinations::Suffix,
    )
    .into_iter()
    .map(|line| {
        line.into_iter()
            .map(|rich| {
                let mut style = rich.style;
                if let Some(link) = style.link.as_deref() {
                    style.link = web_destination(link).map(Arc::from);
                }
                Span::styled(rich.text, style)
            })
            .collect()
    })
    .collect()
}

/// Whether the account's plan is billed directly in dollars (oracle:
/// `PlanType::Ent26`), which hides the credits and shows the native cost.
fn bills_in_dollars(model: &Model) -> bool {
    model.plan_bills_in_dollars()
}

/// The details of the highlighted row (oracle: `render_details`): one row
/// per line, the prompt kept to two rows and the activity to the rows left.
pub fn details(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    center: &Rc<Center>,
    agents: &Shared<Model>,
) -> AnyWidget {
    let (look, strings, num, center, agents) =
        (look.clone(), *strings, *num, center.clone(), agents.clone());
    LayoutBuilder::new(move |cx, constraints| {
        let Some(row) = center.selected_row() else {
            return AnyWidget::default();
        };
        let height = usize::try_from(constraints.max_height).unwrap_or(0);
        let lines = detail_lines(cx, &look, &strings, &num, row, &agents, height);
        top(stacked(lines.into_iter().map(one).collect()))
    })
    .into()
}

fn detail_lines(
    cx: &mut BuildCx<'_>,
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    row: &RowData,
    agents: &Shared<Model>,
    height: usize,
) -> Vec<Line> {
    let seg = segmenter_of(cx);
    let model = &**agents;
    let rendering = Provider::<TuiSettings>::maybe_of(cx, true)
        .map_or_else(Rendering::default, |settings| settings.rendering);
    let (status_text, dot) = status(row, strings, look);
    let pair =
        |label: &'static str, value: &str| vec![span(label, &look.dim), span(value, &look.plain)];
    let mut head: Vec<Line> = vec![
        vec![span(strings.details, &look.bold)],
        Vec::new(),
        vec![span(row.title(), &look.thread(&look.bold, row.id()))],
        vec![dot, span(" ", &look.plain), span(status_text, &look.plain)],
        Vec::new(),
        pair(strings.model, row.model()),
        pair(
            strings.reasoning,
            row.reasoning_effort.as_deref().unwrap_or(strings.unknown),
        ),
        Vec::new(),
        vec![span(strings.project, &look.dim)],
    ];
    head.extend(flow(seg, &row.cwd, &look.plain));
    let usage_lines = usage::lines(strings, num, &row.details.usage, bills_in_dollars(model));
    for line in &usage_lines {
        head.push(pair(line.label, &line.value));
    }
    if let Some(branch) = &row.branch {
        head.push(Vec::new());
        head.push(vec![span(strings.branch, &look.dim)]);
        head.extend(flow(seg, branch, &look.plain));
    }
    let prompt_start = head.len();
    head.push(Vec::new());
    head.push(vec![span(strings.prompt, &look.dim)]);
    let prompt_text = codex_tui_core::screens::agents::preview_markdown(
        row.details.latest_prompt.as_deref().unwrap_or(&row.preview),
    );
    let prompt_text = if prompt_text.is_empty() {
        strings.no_prompt.to_owned()
    } else {
        prompt_text
    };
    let prompt_text = if rendering.tables {
        Cow::Borrowed(prompt_text.as_str())
    } else {
        keep_tables(&prompt_text)
    };
    let mut prompt = markdown_lines(cx, &prompt_text, &row.cwd, rendering);
    if prompt.len() > 2 {
        prompt.truncate(2);
        prompt[1] = vec![span(strings.elision, &look.dim)];
    }
    let mut lines = head;
    lines.extend(prompt);
    let mut activity: Vec<Line> = Vec::new();
    for line in &row.details.lines {
        match line {
            DetailLine::Blank => activity.push(Vec::new()),
            DetailLine::Agent(name) => activity.push(pair(strings.agent_label, name)),
            DetailLine::NeedsAttention => {
                activity.push(vec![span(strings.needs_attention, &look.red)]);
            }
            DetailLine::Request(text) => activity.extend(flow(seg, text, &look.plain)),
            DetailLine::OpenToReview => activity.push(vec![span(strings.open_task, &look.dim)]),
            DetailLine::WaitingApproval => {
                activity.push(vec![span(strings.wait_approval, &look.plain)]);
            }
            DetailLine::WaitingResponse => {
                activity.push(vec![span(strings.wait_response, &look.plain)]);
            }
            DetailLine::TaskError => activity.push(vec![span(strings.task_error, &look.plain)]),
            DetailLine::LatestActivity(header) => {
                activity.push(vec![span(strings.latest_activity, &look.dim)]);
                activity.extend(flow(seg, header, &look.plain));
            }
        }
    }
    if let Some((message, cwd)) = &row.details.last_message {
        activity.push(Vec::new());
        activity.push(vec![span(strings.last_message, &look.dim)]);
        activity.extend(markdown_lines(
            cx,
            &preferred(message, rendering),
            cwd,
            rendering,
        ));
    }
    let offline = matches!(
        model.notice,
        Some(Notice::Reconnecting | Notice::ReconnectFailed)
    );
    if !offline && !activity.is_empty() {
        if !usage_lines.is_empty() && activity.len() > height.saturating_sub(lines.len()) {
            lines.truncate(prompt_start);
        }
        let available = height.saturating_sub(lines.len());
        if activity.len() > available {
            activity.truncate(available);
            if let Some(last) = activity.last_mut() {
                *last = vec![span(strings.elision, &look.dim)];
            }
        }
        let tail = lines.split_off(prompt_start);
        lines.extend(activity);
        lines.extend(tail);
    }
    lines.truncate(height);
    lines
}
