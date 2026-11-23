use codex_tui_rtwk::widgets::age::*;

#[test]
fn ages_follow_the_oracle() {
    assert_eq!(age(100, None), Age::Missing);
    assert_eq!(age(100, Some(100)), Age::Now);
    assert_eq!(age(100, Some(58)), Age::Seconds(42));
    assert_eq!(age(10_000, Some(10_000 - 35 * 60)), Age::Minutes(35));
    assert_eq!(age(100_000, Some(100_000 - 7200)), Age::Hours(2));
    assert_eq!(age(1_000_000, Some(0)), Age::Days(11));
}
