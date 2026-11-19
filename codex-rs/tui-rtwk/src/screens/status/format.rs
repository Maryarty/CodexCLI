//! Number and time formatting of the card, carried from the oracle
//! (`status/helpers.rs`: `format_tokens_compact`; `status/rate_limits.rs`:
//! 12-hour reset times; the limit bar is `card::limit_bar`).

/// `format_tokens_compact` (status/helpers.rs:106): `1_900` is `1.9K`.
pub fn tokens_compact(value: i64) -> String {
    let value = value.max(0);
    if value < 1_000 {
        return value.to_string();
    }
    let value_f64 = value as f64;
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

/// An hour of the 24-hour clock on a 12-hour one: `(hour, pm)`.
pub fn twelve_hour(hour: u8) -> (u8, bool) {
    let pm = hour >= 12;
    let h = hour % 12;
    (if h == 0 { 12 } else { h }, pm)
}
