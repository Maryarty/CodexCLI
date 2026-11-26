//! Plain text formatting shared by pages, carried from the oracle.

/// The time a goal has run: `45s`, `12m`, `3h`, `3h 20m`, `2d 5h 7m`.
// oracle: ../goal_display.rs fn format_goal_elapsed_seconds
pub fn format_goal_elapsed_seconds(seconds: i64) -> String {
    let seconds = seconds.max(0) as u64;
    if seconds < 60 {
        return format!("{seconds}s");
    }

    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }

    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;
    if hours >= 24 {
        let days = hours / 24;
        let remaining_hours = hours % 24;
        return format!("{days}d {remaining_hours}h {remaining_minutes}m");
    }

    if remaining_minutes == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {remaining_minutes}m")
    }
}

// oracle: ../status/helpers.rs fn format_tokens_compact
pub fn format_tokens_compact(value: i64) -> String {
    let value = value.max(0);
    if value == 0 {
        return "0".to_string();
    }
    if value < 1_000 {
        return value.to_string();
    }

    let value_f64 = value as f64;
    // Promote values that would round to 1000 in the smaller unit.
    let (scaled, suffix) = if value >= 999_500_000_000 {
        (value_f64 / 1_000_000_000_000.0, "T")
    } else if value >= 999_500_000 {
        (value_f64 / 1_000_000_000.0, "B")
    } else if value >= 999_500 {
        (value_f64 / 1_000_000.0, "M")
    } else {
        (value_f64 / 1_000.0, "K")
    };

    let decimals = if scaled < 10.0 {
        2
    } else if scaled < 100.0 {
        1
    } else {
        0
    };

    let mut formatted = format!("{scaled:.decimals$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }

    format!("{formatted}{suffix}")
}

/// One line of a plain-text table export. A reserved label affects alignment without emitting text.
pub enum TextLine {
    Heading(String),
    Paragraph(String),
    Blank,
    Row { label: String, values: Vec<String> },
    Reserve(String),
}

/// Serializes status-like rows as plain text without terminal wrapping.
pub fn text_table(
    lines: Vec<TextLine>,
    indent: usize,
    gap: usize,
    width: impl Fn(&str) -> usize,
) -> String {
    let label_width = lines
        .iter()
        .filter_map(|line| match line {
            TextLine::Row { label, .. } | TextLine::Reserve(label) => Some(width(label)),
            TextLine::Heading(_) | TextLine::Paragraph(_) | TextLine::Blank => None,
        })
        .max()
        .unwrap_or_default();
    let value_indent = " ".repeat(indent + label_width + gap);
    let indent = " ".repeat(indent);
    let mut rows = Vec::new();
    for line in lines {
        match line {
            TextLine::Heading(text) => rows.push(text),
            TextLine::Paragraph(text) => rows.push(format!("{indent}{text}")),
            TextLine::Blank => rows.push(String::new()),
            TextLine::Reserve(_) => {}
            TextLine::Row { label, values } => {
                let padding = " ".repeat(label_width - width(&label) + gap);
                let mut values = values.into_iter();
                let first = values.next().unwrap_or_default();
                rows.push(format!("{indent}{label}{padding}{first}"));
                rows.extend(values.map(|value| format!("{value_indent}{value}")));
            }
        }
    }
    rows.iter()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

/// The plain-text form of a two-colour bar, as in the exported status rows.
pub fn text_progress_bar(filled: usize, cells: usize) -> String {
    let filled = filled.min(cells);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(cells - filled))
}
