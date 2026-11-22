//! Repeat expansion: one stored row becomes the occurrences inside a window, in code, never as
//! pre-made rows. Pure and bounded: a hostile or silly series cannot hang the room.
//!
//! Times are wall-clock ("a 9am series stays 9am"): nothing here converts between zones, so a
//! clock change never moves an event and a time that does not exist on a clock-change night is
//! kept as written.
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// The most occurrences one event may give for one request.
pub const MAX_OCCURRENCES: usize = 2000;
/// The most steps the generator may take for one event, whatever the rule says.
const MAX_STEPS: usize = 100_000;
/// The longest an occurrence may stretch over days; longer is cut when shown.
pub const MAX_SPAN_DAYS: i64 = 14;
/// Years the calendar accepts; the rest is refused so date maths cannot overflow.
pub const FIRST_YEAR: i32 = 1970;
pub const LAST_YEAR: i32 = 2200;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Repeat {
    /// day | week | month | year
    pub every: String,
    #[serde(default = "one")]
    pub interval: u32,
    /// ISO weekdays, 1 = Monday .. 7 = Sunday; only for "week"
    #[serde(default)]
    pub weekdays: Vec<u32>,
    /// last day a repeat may fall on, YYYY-MM-DD
    #[serde(default)]
    pub until: Option<String>,
    /// how many occurrences in all, the first included
    #[serde(default)]
    pub count: Option<u32>,
}
fn one() -> u32 {
    1
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Skip,
    Move(NaiveDateTime),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exception {
    pub date: NaiveDate,
    pub change: Change,
}

#[derive(Clone, Debug)]
pub struct Series {
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub repeat: Option<Repeat>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Occurrence {
    /// the day this occurrence would have had without a move; the key of its exception
    pub original: NaiveDate,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
}

impl Occurrence {
    pub fn last_day(&self) -> NaiveDate {
        let last = self.end.map(|e| e.date()).unwrap_or_else(|| self.start.date());
        last.max(self.start.date()).min(self.start.date() + Duration::days(MAX_SPAN_DAYS - 1))
    }
}

pub fn in_range(date: NaiveDate) -> bool {
    (FIRST_YEAR..=LAST_YEAR).contains(&date.year())
}

/// Check a rule on its own; `start` is the event's first day.
pub fn validate(r: &Repeat, start: NaiveDate) -> Result<(), String> {
    if !["day", "week", "month", "year"].contains(&r.every.as_str()) {
        return Err("Repeat every day, week, month or year.".into());
    }
    if !(1..=99).contains(&r.interval) {
        return Err("Repeat every 1 to 99.".into());
    }
    if r.every != "week" && !r.weekdays.is_empty() {
        return Err("Weekdays only go with a weekly repeat.".into());
    }
    if r.weekdays.iter().any(|d| !(1..=7).contains(d)) || r.weekdays.len() > 7 {
        return Err("Weekdays are Monday to Sunday.".into());
    }
    if let Some(n) = r.count {
        if !(1..=1000).contains(&n) {
            return Err("Repeat 1 to 1000 times.".into());
        }
    }
    if let Some(u) = &r.until {
        let u = NaiveDate::parse_from_str(u, "%Y-%m-%d").map_err(|_| "The end of the repeat is not a date.".to_string())?;
        if u < start {
            return Err("The repeat ends before it starts.".into());
        }
        if !in_range(u) {
            return Err("The end of the repeat is too far away.".into());
        }
    }
    Ok(())
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (y, m) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(y, m, 1).map(|d| (d - Duration::days(1)).day()).unwrap_or(28)
}

fn clamped(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day.min(days_in_month(year, month)))
}

/// Every date of the series up to `limit`, in order, honouring interval, until and count.
/// Skipped and moved days are still in this list: they count toward `count`.
pub fn dates(start: NaiveDate, r: &Repeat, limit: NaiveDate) -> Vec<NaiveDate> {
    let mut out: Vec<NaiveDate> = Vec::new();
    let stop = match r.until.as_deref().and_then(|u| NaiveDate::parse_from_str(u, "%Y-%m-%d").ok()) {
        Some(u) => u.min(limit),
        None => limit,
    };
    let wanted = r.count.map(|n| n as usize).unwrap_or(usize::MAX);
    let step = r.interval.max(1) as i64;
    let push = |out: &mut Vec<NaiveDate>, d: NaiveDate| -> bool {
        if d > stop || out.len() >= wanted || out.len() >= MAX_STEPS {
            return false;
        }
        out.push(d);
        true
    };
    match r.every.as_str() {
        "day" => {
            for i in 0..MAX_STEPS as i64 {
                let Some(d) = start.checked_add_signed(Duration::days(i * step)) else { break };
                if !push(&mut out, d) {
                    break;
                }
            }
        }
        "week" => {
            let mut days = r.weekdays.clone();
            if days.is_empty() {
                days.push(start.weekday().number_from_monday());
            }
            days.sort_unstable();
            days.dedup();
            let anchor = start - Duration::days(start.weekday().num_days_from_monday() as i64);
            'weeks: for w in 0..20_000i64 {
                let first = anchor + Duration::days(w * step * 7);
                if first > stop {
                    break;
                }
                for wd in &days {
                    let d = first + Duration::days(*wd as i64 - 1);
                    if d < start {
                        continue;
                    }
                    if !push(&mut out, d) {
                        break 'weeks;
                    }
                }
            }
        }
        "month" => {
            for i in 0..MAX_STEPS as i64 {
                let months = start.month0() as i64 + i * step;
                let year = start.year() as i64 + months.div_euclid(12);
                if year > LAST_YEAR as i64 + 1 {
                    break;
                }
                let Some(d) = clamped(year as i32, months.rem_euclid(12) as u32 + 1, start.day()) else { break };
                if !push(&mut out, d) {
                    break;
                }
            }
        }
        "year" => {
            for i in 0..MAX_STEPS as i64 {
                let year = start.year() as i64 + i * step;
                if year > LAST_YEAR as i64 + 1 {
                    break;
                }
                let Some(d) = clamped(year as i32, start.month(), start.day()) else { break };
                if !push(&mut out, d) {
                    break;
                }
            }
        }
        _ => {}
    }
    out
}

/// The occurrences that touch the window `from..=to` (a multi-day one counts on every day it
/// covers), with skips removed and moves applied, in time order, at most MAX_OCCURRENCES.
pub fn occurrences(s: &Series, exceptions: &[Exception], from: NaiveDate, to: NaiveDate) -> Vec<Occurrence> {
    let length = s.end.map(|e| e - s.start).filter(|d| *d >= Duration::zero());
    let make = |original: NaiveDate, start: NaiveDateTime| Occurrence { original, start, end: length.map(|l| start + l) };
    let touches = |o: &Occurrence| o.start.date() <= to && o.last_day() >= from;
    let mut out: Vec<Occurrence> = Vec::new();
    match &s.repeat {
        None => {
            let o = make(s.start.date(), s.start);
            if touches(&o) {
                out.push(o);
            }
        }
        Some(r) => {
            // A move can bring an occurrence from beyond the window into it, so look as far
            // as the latest changed day.
            let limit = exceptions.iter().map(|e| e.date).max().map_or(to, |m| m.max(to));
            for d in dates(s.start.date(), r, limit) {
                let moved = exceptions.iter().find(|e| e.date == d);
                let start = match moved.map(|e| &e.change) {
                    Some(Change::Skip) => continue,
                    Some(Change::Move(to_when)) => *to_when,
                    None => d.and_time(s.start.time()),
                };
                let o = make(d, start);
                if touches(&o) {
                    out.push(o);
                }
                if out.len() >= MAX_OCCURRENCES {
                    break;
                }
            }
        }
    }
    out.sort_by_key(|o| (o.start, o.original));
    out.truncate(MAX_OCCURRENCES);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }
    fn rule(every: &str) -> Repeat {
        Repeat { every: every.into(), interval: 1, weekdays: vec![], until: None, count: None }
    }
    fn series(start: &str, r: Option<Repeat>) -> Series {
        Series { start: dt(start), end: None, repeat: r }
    }
    fn days(o: &[Occurrence]) -> Vec<String> {
        o.iter().map(|o| o.start.format("%Y-%m-%d").to_string()).collect()
    }

    #[test]
    fn a_single_event_shows_only_inside_its_window() {
        let s = series("2026-11-22 09:00", None);
        assert_eq!(days(&occurrences(&s, &[], d("2026-11-01"), d("2026-11-30"))), vec!["2026-11-22"]);
        assert!(occurrences(&s, &[], d("2026-12-01"), d("2026-12-31")).is_empty());
        assert_eq!(occurrences(&s, &[], d("2026-11-22"), d("2026-11-22")).len(), 1, "both ends are inclusive");
    }

    #[test]
    fn daily_with_an_interval() {
        let mut r = rule("day");
        r.interval = 3;
        let got = occurrences(&series("2026-11-01 08:00", Some(r)), &[], d("2026-11-01"), d("2026-11-14"));
        assert_eq!(days(&got), vec!["2026-11-01", "2026-11-04", "2026-11-07", "2026-11-10", "2026-11-13"]);
    }

    #[test]
    fn weekly_on_several_weekdays_starts_from_the_first_day_and_not_before() {
        let mut r = rule("week");
        r.weekdays = vec![1, 3]; // Monday, Wednesday
        // 2026-11-04 is a Wednesday: the Monday of that week is before the start and is not an occurrence.
        let got = occurrences(&series("2026-11-04 18:30", Some(r)), &[], d("2026-11-01"), d("2026-11-17"));
        assert_eq!(days(&got), vec!["2026-11-04", "2026-11-09", "2026-11-11", "2026-11-16"]);
    }

    #[test]
    fn weekly_without_weekdays_uses_the_start_day_and_the_interval() {
        let mut r = rule("week");
        r.interval = 2;
        let got = occurrences(&series("2026-11-02 10:00", Some(r)), &[], d("2026-11-01"), d("2026-12-31"));
        assert_eq!(days(&got), vec!["2026-11-02", "2026-11-16", "2026-11-30", "2026-12-14", "2026-12-28"]);
    }

    #[test]
    fn monthly_on_the_31st_falls_to_the_month_end_and_does_not_drift() {
        let got = occurrences(&series("2026-01-31 12:00", Some(rule("month"))), &[], d("2026-01-01"), d("2026-05-31"));
        assert_eq!(days(&got), vec!["2026-01-31", "2026-02-28", "2026-03-31", "2026-04-30", "2026-05-31"]);
    }

    #[test]
    fn monthly_crosses_the_year() {
        let got = occurrences(&series("2026-11-15 12:00", Some(rule("month"))), &[], d("2026-11-01"), d("2027-02-28"));
        assert_eq!(days(&got), vec!["2026-11-15", "2026-12-15", "2027-01-15", "2027-02-15"]);
    }

    #[test]
    fn yearly_on_29_february_uses_the_28th_in_other_years() {
        let got = occurrences(&series("2028-02-29 00:00", Some(rule("year"))), &[], d("2028-01-01"), d("2032-12-31"));
        assert_eq!(days(&got), vec!["2028-02-29", "2029-02-28", "2030-02-28", "2031-02-28", "2032-02-29"]);
    }

    #[test]
    fn until_is_inclusive_and_count_includes_the_first() {
        let mut r = rule("day");
        r.until = Some("2026-11-03".into());
        assert_eq!(occurrences(&series("2026-11-01 09:00", Some(r)), &[], d("2026-11-01"), d("2026-12-31")).len(), 3);
        let mut r = rule("day");
        r.count = Some(4);
        assert_eq!(occurrences(&series("2026-11-01 09:00", Some(r)), &[], d("2026-11-01"), d("2026-12-31")).len(), 4);
        let mut r = rule("day");
        r.count = Some(10);
        r.until = Some("2026-11-02".into());
        assert_eq!(occurrences(&series("2026-11-01 09:00", Some(r)), &[], d("2026-11-01"), d("2026-12-31")).len(), 2, "whichever ends first");
    }

    #[test]
    fn a_window_in_the_middle_still_counts_from_the_start() {
        let mut r = rule("day");
        r.count = Some(5);
        let got = occurrences(&series("2026-11-01 09:00", Some(r)), &[], d("2026-11-04"), d("2026-11-30"));
        assert_eq!(days(&got), vec!["2026-11-04", "2026-11-05"], "five in all, three already past");
    }

    #[test]
    fn a_skipped_occurrence_is_gone_but_still_counts() {
        let mut r = rule("day");
        r.count = Some(3);
        let skip = Exception { date: d("2026-11-02"), change: Change::Skip };
        let got = occurrences(&series("2026-11-01 09:00", Some(r)), &[skip], d("2026-11-01"), d("2026-11-30"));
        assert_eq!(days(&got), vec!["2026-11-01", "2026-11-03"]);
    }

    #[test]
    fn a_moved_occurrence_shows_at_its_new_time_and_keeps_its_length() {
        let mut s = series("2026-11-02 09:00", Some(rule("week")));
        s.end = Some(dt("2026-11-02 10:30"));
        let mv = Exception { date: d("2026-11-09"), change: Change::Move(dt("2026-11-10 14:00")) };
        let got = occurrences(&s, &[mv], d("2026-11-01"), d("2026-11-16"));
        assert_eq!(got.len(), 3);
        assert_eq!(got[1].original, d("2026-11-09"));
        assert_eq!(got[1].start, dt("2026-11-10 14:00"));
        assert_eq!(got[1].end, Some(dt("2026-11-10 15:30")));
        assert_eq!(got[2].start, dt("2026-11-16 09:00"));
    }

    #[test]
    fn a_move_from_outside_the_window_into_it_is_found() {
        let s = series("2026-11-02 09:00", Some(rule("week")));
        let mv = Exception { date: d("2026-12-07"), change: Change::Move(dt("2026-11-05 09:00")) };
        let got = occurrences(&s, &[mv], d("2026-11-03"), d("2026-11-06"));
        assert_eq!(days(&got), vec!["2026-11-05"]);
    }

    #[test]
    fn a_move_for_a_day_that_is_not_in_the_series_does_nothing() {
        let s = series("2026-11-02 09:00", Some(rule("week")));
        let mv = Exception { date: d("2026-11-04"), change: Change::Move(dt("2026-11-05 09:00")) };
        assert_eq!(occurrences(&s, &[mv], d("2026-11-03"), d("2026-11-06")).len(), 0);
    }

    #[test]
    fn wall_clock_times_stay_put_across_a_clock_change() {
        // Europe, the clocks went forward on 2027-03-28 and 02:30 did not exist that night:
        // the series keeps 02:30 as written; nothing is shifted to 03:30.
        let got = occurrences(&series("2027-03-27 02:30", Some(rule("day"))), &[], d("2027-03-27"), d("2027-03-29"));
        assert!(got.iter().all(|o| o.start.format("%H:%M").to_string() == "02:30"));
        assert_eq!(got.len(), 3);
        // and again when they went back and 01:30 happened twice
        let got = occurrences(&series("2026-10-24 01:30", Some(rule("day"))), &[], d("2026-10-24"), d("2026-10-26"));
        assert!(got.iter().all(|o| o.start.format("%H:%M").to_string() == "01:30"));
    }

    #[test]
    fn an_all_day_event_stays_on_its_date_and_a_multi_day_one_covers_each_day() {
        let mut s = series("2026-12-31 00:00", None);
        s.end = Some(dt("2027-01-02 00:00"));
        let got = occurrences(&s, &[], d("2027-01-01"), d("2027-01-01"));
        assert_eq!(got.len(), 1, "the middle day of a three-day event touches the window");
        assert_eq!(got[0].last_day(), d("2027-01-02"));
        assert!(occurrences(&s, &[], d("2027-01-03"), d("2027-01-04")).is_empty());
    }

    #[test]
    fn a_very_long_event_is_cut_to_the_span_limit() {
        let mut s = series("2026-01-01 00:00", None);
        s.end = Some(dt("2026-12-31 00:00"));
        let got = occurrences(&s, &[], d("2026-01-01"), d("2026-01-31"));
        assert_eq!(got[0].last_day(), d("2026-01-14"));
    }

    #[test]
    fn the_occurrence_cap_holds_whatever_the_rule_says() {
        let got = occurrences(&series("2026-01-01 00:00", Some(rule("day"))), &[], d("2026-01-01"), d("2027-02-04"));
        assert_eq!(got.len(), 400);
        let mut r = rule("day");
        r.interval = 1;
        // a window of years is never asked for by the command, but the engine alone is still bounded
        let big = occurrences(&series("1970-01-01 00:00", Some(r)), &[], d("1970-01-01"), d("2200-12-31"));
        assert!(big.len() <= MAX_OCCURRENCES);
    }

    #[test]
    fn a_year_is_cheap_and_the_far_future_does_not_overflow() {
        let s = series("2026-11-22 09:00", Some(rule("week")));
        assert_eq!(occurrences(&s, &[], d("2027-11-01"), d("2027-11-30")).len(), 4);
        let far = series("2199-12-31 09:00", Some(rule("year")));
        let _ = occurrences(&far, &[], d("2199-01-01"), d("2200-12-31"));
    }

    #[test]
    fn validation_refuses_what_it_cannot_expand() {
        let start = d("2026-11-01");
        assert!(validate(&rule("week"), start).is_ok());
        assert!(validate(&rule("fortnight"), start).is_err());
        let mut r = rule("day");
        r.interval = 0;
        assert!(validate(&r, start).is_err());
        r.interval = 100;
        assert!(validate(&r, start).is_err());
        let mut r = rule("day");
        r.weekdays = vec![1];
        assert!(validate(&r, start).is_err(), "weekdays only for weekly");
        let mut r = rule("week");
        r.weekdays = vec![0];
        assert!(validate(&r, start).is_err());
        r.weekdays = vec![8];
        assert!(validate(&r, start).is_err());
        let mut r = rule("day");
        r.count = Some(0);
        assert!(validate(&r, start).is_err());
        r.count = Some(1001);
        assert!(validate(&r, start).is_err());
        let mut r = rule("day");
        r.until = Some("2026-10-31".into());
        assert!(validate(&r, start).is_err(), "ends before it starts");
        r.until = Some("not a date".into());
        assert!(validate(&r, start).is_err());
        r.until = Some("2300-01-01".into());
        assert!(validate(&r, start).is_err());
    }

    #[test]
    fn a_rule_with_an_unknown_field_is_refused_when_read() {
        assert!(serde_json::from_str::<Repeat>(r#"{"every":"day","rrule":"FREQ=DAILY"}"#).is_err());
        let r: Repeat = serde_json::from_str(r#"{"every":"week","weekdays":[1,3]}"#).unwrap();
        assert_eq!(r.interval, 1);
    }
}
