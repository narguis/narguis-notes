use chrono::{FixedOffset, TimeZone};
use notes_planner_desktop::civil_date::CivilDate;

#[test]
fn rejects_non_canonical_and_impossible_calendar_date_inputs() {
    // Given: impossible dates and strings that carry a time zone or UTC instant
    let invalid_inputs = [
        "2026-02-30",
        "2026-07-30T00:00:00Z",
        "2026-07-30T00:00:00+01:00",
        "2026-7-30",
    ];

    // When: each input is parsed as a planner identity
    let results = invalid_inputs.map(CivilDate::parse);

    // Then: no instant or non-canonical string becomes a civil page key
    assert!(results.into_iter().all(|result| result.is_err()));
}

#[test]
fn derives_the_same_civil_key_from_a_fixed_local_midnight_fixture() {
    // Given: the first local instant on the European DST transition date
    let offset = FixedOffset::east_opt(3_600).expect("one-hour offset should be valid");
    let local_midnight = offset
        .with_ymd_and_hms(2026, 3, 29, 0, 0, 0)
        .single()
        .expect("fixed-offset midnight should be unambiguous");

    // When: the planner derives its key from that local clock reading
    let today =
        CivilDate::from_local_datetime(local_midnight).expect("local fixture should derive");

    // Then: the page identity remains the local civil date, not a UTC date
    assert_eq!(today.as_str(), "2026-03-29");
}

#[test]
fn navigates_by_calendar_days_across_leap_month_year_and_dst_boundaries() {
    // Given: dates adjacent to calendar and daylight-saving boundaries
    let leap_day = CivilDate::parse("2024-02-29").expect("leap date should parse");
    let year_end = CivilDate::parse("2026-12-31").expect("year end should parse");
    let dst_start = CivilDate::parse("2026-03-29").expect("DST date should parse");

    // When: navigation moves by civil calendar days
    let after_leap_day = leap_day
        .next_day()
        .expect("next leap-day date should exist");
    let after_year_end = year_end.next_day().expect("next year date should exist");
    let before_dst_start = dst_start
        .previous_day()
        .expect("previous DST date should exist");
    let after_dst_start = dst_start.next_day().expect("next DST date should exist");

    // Then: civil arithmetic preserves the correct date boundaries without elapsed-hour math
    assert_eq!(after_leap_day.as_str(), "2024-03-01");
    assert_eq!(after_year_end.as_str(), "2027-01-01");
    assert_eq!(before_dst_start.as_str(), "2026-03-28");
    assert_eq!(after_dst_start.as_str(), "2026-03-30");
}
