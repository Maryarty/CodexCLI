use codex_tui_core::screens::usage::date::*;
use std::str::FromStr;

#[test]
fn civil_days_round_trip_and_weekdays() {
    let date = NaiveDate::from_ymd_opt(2026, 9, 9).unwrap();
    assert_eq!(date.ymd(), (2026, 9, 9));
    assert_eq!(date.to_string(), "2026-09-09");
    // 2026-09-09 is a Wednesday.
    assert_eq!(date.weekday_from_sunday(), 3);
    assert_eq!(date.minus_days(9).to_string(), "2026-08-31");
    assert_eq!(
        NaiveDate::from_ymd_opt(2024, 2, 29)
            .unwrap()
            .plus_days(1)
            .to_string(),
        "2024-03-01"
    );
    assert!(NaiveDate::from_ymd_opt(2026, 2, 29).is_none());
    assert_eq!(
        "2026-01-09".parse::<NaiveDate>(),
        Ok(NaiveDate::from_ymd_opt(2026, 1, 9).unwrap())
    );
    assert!("2026-1-9".parse::<NaiveDate>().is_err());
}

#[test]
fn rfc3339_instants() {
    assert_eq!(rfc3339_unix("1970-01-02T00:00:00Z"), Some(86_400));
    assert_eq!(
        rfc3339_unix("2026-01-09T10:20:30.5Z"),
        rfc3339_unix("2026-01-09T12:20:30+02:00")
    );
    assert_eq!(rfc3339_unix("2026-01-09"), None);
}
