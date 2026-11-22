//! Five local-time cron fields. Restricted day-of-month and weekday use OR semantics.
//! Walk wall-clock minutes; ambiguous minutes choose their first occurrence, gaps are skipped.
use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, TimeZone, Timelike};
use serde_json::Value;
fn field(s: &str, lo: u32, hi: u32) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let (base, step) = match part.split_once('/') {
            Some((b, n)) => (b, n.parse::<u32>().ok()?),
            None => (part, 1),
        };
        if step == 0 || step > hi + 1 {
            return None;
        }
        let (a, b) = if base == "*" {
            (lo, hi)
        } else if let Some((a, b)) = base.split_once('-') {
            (a.parse().ok()?, b.parse().ok()?)
        } else {
            let a = base.parse().ok()?;
            (a, if part.contains('/') { hi } else { a })
        };
        if a < lo || b > hi || a > b {
            return None;
        }
        out.extend((a..=b).step_by(step as usize));
    }
    (!out.is_empty()).then_some(out)
}
pub fn valid(expr: &str) -> bool {
    parse(expr).is_some()
}
fn parse(expr: &str) -> Option<Vec<Vec<u32>>> {
    let f: Vec<_> = expr.split_whitespace().collect();
    if f.len() != 5 {
        return None;
    }
    [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)]
        .iter()
        .enumerate()
        .map(|(i, (a, b))| field(f[i], *a, *b))
        .collect()
}
pub fn next_after(expr: &str, from: DateTime<Local>) -> Option<DateTime<Local>> {
    next_in(expr, from)
}
fn next_in<T: TimeZone>(expr: &str, from: DateTime<T>) -> Option<DateTime<T>> {
    let zone = from.timezone();
    let f = parse(expr)?;
    let raw: Vec<_> = expr.split_whitespace().collect();
    let mut t = from.naive_local().with_second(0)?.with_nanosecond(0)? + Duration::minutes(1);
    let end = t + Duration::days(366 * 8);
    while t < end {
        let wd = t.weekday().num_days_from_sunday();
        let dom = f[2].contains(&t.day());
        let dow = f[4].iter().any(|v| *v % 7 == wd);
        let day = if raw[2] == "*" {
            dow
        } else if raw[4] == "*" {
            dom
        } else {
            dom || dow
        };
        if f[3].contains(&t.month())
            && day
            && f[1].contains(&t.hour())
            && f[0].contains(&t.minute())
        {
            if let Some(real) = zone
                .from_local_datetime(&t)
                .earliest()
                .filter(|r| *r > from)
            {
                return Some(real);
            }
        }
        t += Duration::minutes(1);
    }
    None
}
pub fn next(trigger: &Value, from: DateTime<Local>) -> Option<DateTime<Local>> {
    if let Some(expr) = trigger["cron"].as_str() {
        next_after(expr, from)
    } else if let Some(n) = trigger["every_minutes"]
        .as_i64()
        .filter(|n| (1..=525600).contains(n))
    {
        Some(from + Duration::minutes(n))
    } else {
        let t = NaiveDateTime::parse_from_str(trigger["at"].as_str()?, "%Y-%m-%dT%H:%M").ok()?;
        Local
            .from_local_datetime(&t)
            .earliest()
            .filter(|t| *t > from)
    }
}
pub fn stamp(t: DateTime<Local>) -> String {
    t.format("%Y-%m-%d %H:%M:%S").to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn date(s: &str) -> DateTime<Local> {
        Local
            .from_local_datetime(&NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap())
            .earliest()
            .unwrap()
    }
    #[test]
    fn fields_and_invalid() {
        for x in ["* * * * *", "*/15 8-17 * * 1-5", "0,30 8 * 1,6 0,7"] {
            assert!(valid(x));
        }
        for x in [
            "* * * *",
            "60 * * * *",
            "*/0 * * * *",
            "0 25 * * *",
            "0 0 0 * *",
            "0 0 * 13 *",
            "0 0 * * 8",
            "1-0 * * * *",
        ] {
            assert!(!valid(x), "{x}");
        }
    }
    #[test]
    fn calendar() {
        assert_eq!(
            stamp(next_after("0 0 29 2 *", date("2025-03-01 00:00")).unwrap()),
            "2028-02-29 00:00:00"
        );
        assert_eq!(
            stamp(next_after("0 0 1 * *", date("2026-01-31 23:59")).unwrap()),
            "2026-02-01 00:00:00"
        );
        assert_eq!(
            stamp(next_after("*/15 8-17 * * 1-5", date("2026-11-20 17:45")).unwrap()),
            "2026-11-23 08:00:00"
        );
    }
    #[test]
    fn day_fields_are_or() {
        assert_eq!(
            stamp(next_after("0 0 1 * 1", date("2026-11-01 00:00")).unwrap()),
            "2026-11-02 00:00:00"
        );
    }
    #[test]
    fn strict_future() {
        let t = date("2026-11-21 12:00");
        assert!(next(&serde_json::json!({"at":"2026-11-21T12:00"}), t).is_none());
        assert_eq!(
            next(&serde_json::json!({"every_minutes":30}), t).unwrap() - t,
            Duration::minutes(30)
        );
    }
}

#[cfg(test)]
mod dst_tests {
    use super::*;
    use chrono::{FixedOffset, LocalResult, NaiveDate, Offset};
    #[derive(Clone)]
    struct TestZone;
    impl TimeZone for TestZone {
        type Offset = FixedOffset;
        fn from_offset(_: &Self::Offset) -> Self {
            Self
        }
        fn offset_from_local_date(&self, d: &NaiveDate) -> LocalResult<Self::Offset> {
            self.offset_from_local_datetime(&d.and_hms_opt(12, 0, 0).unwrap())
        }
        fn offset_from_local_datetime(&self, t: &NaiveDateTime) -> LocalResult<Self::Offset> {
            let standard = FixedOffset::west_opt(5 * 3600).unwrap();
            let summer = FixedOffset::west_opt(4 * 3600).unwrap();
            if t.month() == 3 && t.day() == 8 && t.hour() == 2 {
                LocalResult::None
            } else if t.month() == 11 && t.day() == 1 && t.hour() == 1 {
                LocalResult::Ambiguous(summer, standard)
            } else if t.month() == 11 && t.day() == 1 && t.hour() == 0
                || t.month() > 3 && t.month() < 11
                || t.month() == 3 && (t.day() > 8 || t.day() == 8 && t.hour() > 2)
            {
                LocalResult::Single(summer)
            } else {
                LocalResult::Single(standard)
            }
        }
        fn offset_from_utc_date(&self, _: &NaiveDate) -> Self::Offset {
            FixedOffset::west_opt(5 * 3600).unwrap()
        }
        fn offset_from_utc_datetime(&self, t: &NaiveDateTime) -> Self::Offset {
            if t.month() == 11 && t.day() == 1 && t.hour() < 6 {
                FixedOffset::west_opt(4 * 3600).unwrap()
            } else {
                self.offset_from_local_datetime(t)
                    .single()
                    .unwrap_or(FixedOffset::west_opt(5 * 3600).unwrap())
            }
        }
    }
    #[test]
    fn nonexistent_time_is_skipped() {
        let from = TestZone.with_ymd_and_hms(2026, 3, 8, 1, 59, 0).unwrap();
        let next = next_in("30 2 * * *", from).unwrap();
        assert_eq!((next.day(), next.hour(), next.minute()), (9, 2, 30));
    }
    #[test]
    fn repeated_minute_runs_once() {
        let from = TestZone.with_ymd_and_hms(2026, 11, 1, 0, 59, 0).unwrap();
        let first = next_in("30 1 * * *", from).unwrap();
        assert_eq!(first.offset().fix().local_minus_utc(), -4 * 3600);
        let next = next_in("30 1 * * *", first).unwrap();
        assert_eq!(next.day(), 2);
    }
}
