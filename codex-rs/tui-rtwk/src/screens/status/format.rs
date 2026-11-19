//! Number and time formatting of the card, carried from the oracle
//! (`status/rate_limits.rs`:
//! 12-hour reset times; the limit bar is `card::limit_bar`).

/// An hour of the 24-hour clock on a 12-hour one: `(hour, pm)`.
pub fn twelve_hour(hour: u8) -> (u8, bool) {
    let pm = hour >= 12;
    let h = hour % 12;
    (if h == 0 { 12 } else { h }, pm)
}
