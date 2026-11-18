//! The `Tokens:` and `Est. usage:` lines of the details (oracle:
//! `agents_overview_usage.rs` `usage_lines`) with the cost formatters of the
//! oracle's `status/thread_usage.rs` (`format_credit_micros`,
//! `format_estimated_usd_micros`). The model's figures come from core
//! (`Usage::token_counts`, the estimate's fields); the words are `strings.rs`.

use rtwk::l10n::NumSyms;

use codex_tui_core::screens::agents::{Usage, format_tokens_compact};

use super::strings::Strings;

/// One line of the usage block: its label and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageLine {
    pub label: &'static str,
    pub value: String,
}

/// The lines, in the oracle's order. `ent26`: the plan is billed directly in
/// dollars (no credits, the native cost).
pub fn lines(strings: &Strings, num: &NumSyms, usage: &Usage, ent26: bool) -> Vec<UsageLine> {
    let (input, output) = usage.token_counts();
    let mut lines = Vec::new();
    let tokens = strings.tokens_line(
        input.map(format_tokens_compact).as_deref(),
        output.map(format_tokens_compact).as_deref(),
        num,
    );
    if !tokens.is_empty() {
        lines.push(UsageLine {
            label: strings.tokens,
            value: tokens,
        });
    }
    if usage.has_estimate() {
        let mut values = Vec::new();
        if !ent26 && usage.credits_micros().is_some_and(|micros| micros >= 0) {
            values.push(strings.credits(
                &format_credit_micros(usage.credits_micros().unwrap_or_default()),
                num,
            ));
        }
        let cost = if ent26 {
            usage.native_usd_micros()
        } else {
            usage.usd_micros()
        };
        if let Some(cost) = cost.and_then(format_estimated_usd_micros) {
            values.push(cost);
        }
        if !values.is_empty() {
            lines.push(UsageLine {
                label: strings.est_usage,
                value: strings.join_values(&values),
            });
        }
    }
    lines
}

/// Credits as `1.5K`, `12M` (oracle: `format_credit_micros`).
pub fn format_credit_micros(micros: i64) -> String {
    let micros = i128::from(micros.max(0));
    let (unit_micros, suffix) = [
        (1_000_000_000_000_000_000_i128, "T"),
        (1_000_000_000_000_000_i128, "B"),
        (1_000_000_000_000_i128, "M"),
        (1_000_000_000_i128, "K"),
    ]
    .into_iter()
    .find(|(unit, _)| micros >= *unit)
    .unwrap_or((1_000_000, ""));
    let tenths = (micros * 10 + unit_micros / 2) / unit_micros;
    let whole = tenths / 10;
    let fraction = tenths % 10;
    if fraction == 0 {
        format!("{whole}{suffix}")
    } else {
        format!("{whole}.{fraction}{suffix}")
    }
}

/// Dollars as `~$0.0123` (oracle: `format_estimated_usd_micros`).
pub fn format_estimated_usd_micros(micros: i64) -> Option<String> {
    if micros < 0 {
        return None;
    }
    if micros > 0 && micros < 100 {
        return Some(format!("~$0.{micros:06}"));
    }
    if micros > 0 && micros < 10_000 {
        let ten_thousandths = micros.saturating_add(50) / 100;
        return Some(format!("~$0.{ten_thousandths:04}"));
    }
    let cents = micros.saturating_add(5_000) / 10_000;
    Some(format!("~${}.{:02}", cents / 100, cents % 100))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::agents::strings::EN;

    /// `incomplete_billing_groups_do_not_display_partial_token_totals`
    /// (oracle: `agents_overview_usage_tests.rs:302`).
    #[test]
    fn incomplete_billing_groups_do_not_display_partial_token_totals() {
        let estimate = serde_json::from_value(serde_json::json!({
            "threadId": "thread", "estimatedUsageCreditsMicros": 0, "estimatedUsageUsdMicros": null,
            "groups": [
                {"estimatedUsageCreditsMicros": 0, "inputTokens": 100, "outputTokens": 10},
                {"estimatedUsageCreditsMicros": 0, "inputTokens": null, "outputTokens": 20}
            ]
        }))
        .unwrap();
        let usage = Usage {
            estimate: Some(estimate),
            ..Usage::default()
        };
        let text: Vec<String> = lines(&EN, &NumSyms::ENGLISH, &usage, false)
            .into_iter()
            .map(|line| format!("{}{}", line.label, line.value))
            .collect();
        assert_eq!(text, vec!["Tokens: 30 out", "Est. usage: 0 credits"]);
    }

    #[test]
    fn credits_and_dollars_read_as_the_oracle_writes_them() {
        assert_eq!(format_credit_micros(1_500_000_000), "1.5K");
        assert_eq!(format_credit_micros(12_000_000), "12");
        assert_eq!(format_estimated_usd_micros(50), Some("~$0.000050".into()));
        assert_eq!(
            format_estimated_usd_micros(1_234_500),
            Some("~$1.23".into())
        );
        assert_eq!(format_estimated_usd_micros(-1), None);
    }
}
