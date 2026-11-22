//! Quick add: one line of plain words becomes an event, parsed here with no model call.
//! "Dentist fri 3pm", "Pay rent 1 dec every month", "Gym every mon 18:30 remind 30m".
//! The member always sees what was understood before anything is saved, so a wrong guess is
//! visible. Only English words are understood; anything else stays in the title.
use super::repeat::Repeat;
use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Quick {
    pub title: String,
    /// YYYY-MM-DD when a date was understood
    pub date: Option<String>,
    /// HH:MM when a time was understood; none means the event is all day
    pub time: Option<String>,
    pub repeat: Option<Repeat>,
    pub remind_min: Option<u32>,
    /// a short line saying what was understood, for the member to check
    pub summary: String,
    pub has_date: bool,
}

const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
const MONTH_NAMES: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn weekday(word: &str) -> Option<u32> {
    Some(match word {
        "mon" | "monday" => 1,
        "tue" | "tues" | "tuesday" => 2,
        "wed" | "weds" | "wednesday" => 3,
        "thu" | "thur" | "thurs" | "thursday" => 4,
        "fri" | "friday" => 5,
        "sat" | "saturday" => 6,
        "sun" | "sunday" => 7,
        _ => return None,
    })
}

fn month(word: &str) -> Option<u32> {
    let w = word.trim_end_matches('.');
    let long = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
    if let Some(i) = long.iter().position(|m| *m == w) {
        return Some(i as u32 + 1);
    }
    if w == "sept" {
        return Some(9);
    }
    MONTHS.iter().position(|m| *m == w).map(|i| i as u32 + 1)
}

fn day_number(word: &str) -> Option<u32> {
    let w = word.trim_end_matches(',');
    let digits = w.trim_end_matches("st").trim_end_matches("nd").trim_end_matches("rd").trim_end_matches("th");
    if digits.is_empty() || digits.len() > 2 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: u32 = digits.parse().ok()?;
    (1..=31).contains(&n).then_some(n)
}

fn year_number(word: &str) -> Option<i32> {
    let w = word.trim_end_matches(',');
    (w.len() == 4 && w.chars().all(|c| c.is_ascii_digit())).then(|| w.parse().ok()).flatten().filter(|y| (1970..=2200).contains(y))
}

/// "3pm", "3:30pm", "15:30" -> (hour, minute)
fn clock(word: &str) -> Option<(u32, u32)> {
    let w = word.trim_end_matches(',');
    let (body, pm) = if let Some(b) = w.strip_suffix("pm") {
        (b, Some(true))
    } else if let Some(b) = w.strip_suffix("am") {
        (b, Some(false))
    } else {
        (w, None)
    };
    let (h, m) = match body.split_once(':') {
        Some((h, m)) => (h, m),
        None if pm.is_some() => (body, "0"),
        None => return None,
    };
    if h.is_empty() || h.len() > 2 || m.is_empty() || m.len() > 2 || !h.chars().chain(m.chars()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (mut h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    if m > 59 {
        return None;
    }
    match pm {
        Some(p) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            h %= 12;
            if p {
                h += 12;
            }
        }
        None => {
            if h > 23 || body.split_once(':').map(|(a, _)| a.len()).unwrap_or(0) == 0 {
                return None;
            }
        }
    }
    Some((h, m))
}

/// "30m", "45min", "1h", "2hr", "1d" -> minutes
fn lead(word: &str) -> Option<u32> {
    let w = word.trim_end_matches(',');
    let at = w.find(|c: char| !c.is_ascii_digit())?;
    let (n, unit) = w.split_at(at);
    let n: u32 = n.parse().ok().filter(|n| (1..=999).contains(n))?;
    let per = match unit {
        "m" | "min" | "mins" => 1,
        "h" | "hr" | "hrs" | "hour" | "hours" => 60,
        "d" | "day" | "days" => 1440,
        _ => return None,
    };
    let total = n * per;
    (total <= 14 * 1440).then_some(total)
}

fn unit_of(word: &str) -> Option<&'static str> {
    match word.trim_end_matches('s') {
        "day" => Some("day"),
        "week" => Some("week"),
        "month" => Some("month"),
        "year" => Some("year"),
        _ => None,
    }
}

fn after(today: NaiveDate, wd: u32) -> NaiveDate {
    let now = today.weekday().number_from_monday();
    today + Duration::days(((wd + 7 - now - 1) % 7 + 1) as i64)
}

fn next_week(today: NaiveDate, wd: u32) -> NaiveDate {
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    monday + Duration::days(7 + wd as i64 - 1)
}

fn month_day(today: NaiveDate, m: u32, d: u32, year: Option<i32>) -> Option<NaiveDate> {
    if let Some(y) = year {
        return NaiveDate::from_ymd_opt(y, m, d);
    }
    (0..5).filter_map(|k| NaiveDate::from_ymd_opt(today.year() + k, m, d)).find(|date| *date >= today)
}

pub fn parse(text: &str, today: NaiveDate) -> Quick {
    let words: Vec<&str> = text.split_whitespace().collect();
    let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    let mut used = vec![false; words.len()];
    let (mut date, mut time, mut repeat, mut remind): (Option<NaiveDate>, Option<(u32, u32)>, Option<Repeat>, Option<u32>) = (None, None, None, None);
    let mut i = 0;
    while i < words.len() {
        let w = lower[i].as_str().trim_end_matches(',');
        let next = lower.get(i + 1).map(|s| s.trim_end_matches(',')).unwrap_or("");
        let take = |n: usize, used: &mut Vec<bool>| {
            for k in 0..n {
                used[i + k] = true;
            }
        };
        if remind.is_none() && w == "remind" {
            if let Some(m) = lead(next) {
                remind = Some(m);
                take(2, &mut used);
                i += 2;
                continue;
            }
        }
        if repeat.is_none() && w == "every" {
            // every [N] unit | every weekday
            if let Some(wd) = weekday(next) {
                repeat = Some(Repeat { every: "week".into(), interval: 1, weekdays: vec![wd], until: None, count: None });
                if date.is_none() {
                    date = Some(after(today, wd));
                }
                take(2, &mut used);
                i += 2;
                continue;
            }
            if let Some(u) = unit_of(next) {
                repeat = Some(Repeat { every: u.into(), interval: 1, weekdays: vec![], until: None, count: None });
                take(2, &mut used);
                i += 2;
                continue;
            }
            if let Ok(n) = next.parse::<u32>() {
                if let Some(u) = lower.get(i + 2).and_then(|s| unit_of(s.trim_end_matches(','))) {
                    if (1..=99).contains(&n) {
                        repeat = Some(Repeat { every: u.into(), interval: n, weekdays: vec![], until: None, count: None });
                        take(3, &mut used);
                        i += 3;
                        continue;
                    }
                }
            }
        }
        if repeat.is_none() {
            let every = match w {
                "daily" => Some("day"),
                "weekly" => Some("week"),
                "monthly" => Some("month"),
                "yearly" | "annually" => Some("year"),
                _ => None,
            };
            if let Some(u) = every {
                repeat = Some(Repeat { every: u.into(), interval: 1, weekdays: vec![], until: None, count: None });
                take(1, &mut used);
                i += 1;
                continue;
            }
        }
        if date.is_none() {
            if w == "today" || w == "tomorrow" {
                date = Some(if w == "today" { today } else { today + Duration::days(1) });
                take(1, &mut used);
                i += 1;
                continue;
            }
            if w == "next" {
                if let Some(wd) = weekday(next) {
                    date = Some(next_week(today, wd));
                    take(2, &mut used);
                    i += 2;
                    continue;
                }
            }
            if let Some(wd) = weekday(w) {
                date = Some(after(today, wd));
                take(1, &mut used);
                i += 1;
                continue;
            }
            if let Ok(d) = NaiveDate::parse_from_str(w, "%Y-%m-%d") {
                date = Some(d);
                take(1, &mut used);
                i += 1;
                continue;
            }
            // 25 dec [2026]  |  dec 25 [2026]
            let pair = if let (Some(d), Some(m)) = (day_number(w), month(next)) {
                Some((d, m))
            } else if let (Some(m), Some(d)) = (month(w), day_number(next)) {
                Some((d, m))
            } else {
                None
            };
            if let Some((d, m)) = pair {
                let year = lower.get(i + 2).and_then(|s| year_number(s));
                if let Some(found) = month_day(today, m, d, year) {
                    date = Some(found);
                    let n = if year.is_some() { 3 } else { 2 };
                    take(n, &mut used);
                    i += n;
                    continue;
                }
            }
        }
        if time.is_none() {
            if w == "at" {
                if let Some(t) = clock(next) {
                    time = Some(t);
                    take(2, &mut used);
                    i += 2;
                    continue;
                }
            }
            if let Some(t) = clock(w) {
                time = Some(t);
                take(1, &mut used);
                i += 1;
                continue;
            }
        }
        i += 1;
    }
    let title = words.iter().zip(&used).filter(|(_, u)| !**u).map(|(w, _)| *w).collect::<Vec<_>>().join(" ");
    let has_date = date.is_some();
    let time_text = time.map(|(h, m)| format!("{h:02}:{m:02}"));
    let mut parts: Vec<String> = Vec::new();
    match date {
        Some(d) => parts.push(format!("{} {} {}", DAY_NAMES[d.weekday().num_days_from_monday() as usize], d.day(), MONTH_NAMES[d.month0() as usize])),
        None => parts.push("no date understood".into()),
    }
    parts.push(time_text.clone().unwrap_or_else(|| "all day".into()));
    if let Some(r) = &repeat {
        let unit = format!("{}s", r.every);
        parts.push(if r.interval > 1 { format!("repeats every {} {}", r.interval, unit) } else { format!("repeats every {}", r.every) });
    }
    if let Some(m) = remind {
        parts.push(format!("remind {m} min before"));
    }
    let summary = if title.is_empty() { format!("(no title yet) - {}", parts.join(", ")) } else { format!("{} - {}", title, parts.join(", ")) };
    Quick { title, date: date.map(|d| d.to_string()), time: time_text, repeat, remind_min: remind, summary, has_date }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Sunday 22 November 2026
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()
    }
    fn q(text: &str) -> Quick {
        parse(text, today())
    }

    #[test]
    fn a_weekday_and_a_time() {
        let r = q("Dentist fri 3pm");
        assert_eq!(r.title, "Dentist");
        assert_eq!(r.date.as_deref(), Some("2026-11-27"));
        assert_eq!(r.time.as_deref(), Some("15:00"));
        assert_eq!(r.summary, "Dentist - Fri 27 Nov, 15:00");
    }

    #[test]
    fn a_bare_weekday_is_the_next_one_after_today_and_never_today() {
        assert_eq!(q("x sun").date.as_deref(), Some("2026-11-29"), "today is Sunday");
        assert_eq!(q("x mon").date.as_deref(), Some("2026-11-23"));
        assert_eq!(q("x Tuesday").date.as_deref(), Some("2026-11-24"));
    }

    #[test]
    fn next_weekday_means_the_following_week() {
        // Sunday 22 Nov still belongs to the week that began on Monday 16 Nov.
        assert_eq!(q("Review next tue").date.as_deref(), Some("2026-11-24"));
        assert_eq!(q("Review next mon").date.as_deref(), Some("2026-11-23"));
        assert_eq!(q("Review next sun").date.as_deref(), Some("2026-11-29"));
    }

    #[test]
    fn today_and_tomorrow() {
        assert_eq!(q("Call mum today").date.as_deref(), Some("2026-11-22"));
        assert_eq!(q("Call mum tomorrow 9:15").date.as_deref(), Some("2026-11-23"));
        assert_eq!(q("Call mum tomorrow 9:15").time.as_deref(), Some("09:15"));
    }

    #[test]
    fn day_and_month_in_either_order_and_with_a_year() {
        assert_eq!(q("Party 25 dec").date.as_deref(), Some("2026-12-25"));
        assert_eq!(q("Party dec 25").date.as_deref(), Some("2026-12-25"));
        assert_eq!(q("Party 25th December").date.as_deref(), Some("2026-12-25"));
        assert_eq!(q("Party 25 dec 2027").date.as_deref(), Some("2027-12-25"));
        assert_eq!(q("Party 25 dec 2027").title, "Party");
        assert_eq!(q("Birthday 3 jan").date.as_deref(), Some("2027-01-03"), "a date already passed this year means next year");
        assert_eq!(q("Birthday 22 nov").date.as_deref(), Some("2026-11-22"), "today itself is not past");
    }

    #[test]
    fn an_iso_date() {
        let r = q("Flight 2026-12-25 06:45");
        assert_eq!(r.date.as_deref(), Some("2026-12-25"));
        assert_eq!(r.time.as_deref(), Some("06:45"));
        assert_eq!(r.title, "Flight");
        assert!(q("Odd 2026-02-30").date.is_none(), "an impossible date is not guessed");
    }

    #[test]
    fn times_in_every_form() {
        assert_eq!(q("a 3pm").time.as_deref(), Some("15:00"));
        assert_eq!(q("a 3:30pm").time.as_deref(), Some("15:30"));
        assert_eq!(q("a 12am").time.as_deref(), Some("00:00"));
        assert_eq!(q("a 12pm").time.as_deref(), Some("12:00"));
        assert_eq!(q("a 9am").time.as_deref(), Some("09:00"));
        assert_eq!(q("a 15:30").time.as_deref(), Some("15:30"));
        assert_eq!(q("a at 7:05pm").time.as_deref(), Some("19:05"));
        assert_eq!(q("a at 7:05pm").title, "a");
    }

    #[test]
    fn a_bare_number_is_not_a_time_and_bad_times_are_not_guessed() {
        let r = q("Meet 5 people");
        assert_eq!(r.time, None);
        assert_eq!(r.title, "Meet 5 people");
        assert_eq!(q("a 25:00").time, None);
        assert_eq!(q("a 13pm").time, None);
        assert_eq!(q("a 9:75").time, None);
        assert_eq!(q("a 0pm").time, None);
    }

    #[test]
    fn repeats() {
        let r = q("Standup daily 9:30");
        assert_eq!(r.repeat.as_ref().map(|r| (r.every.as_str(), r.interval)), Some(("day", 1)));
        assert_eq!(q("Rent 1 dec every month").repeat.unwrap().every, "month");
        assert_eq!(q("Water plants every week").repeat.unwrap().every, "week");
        assert_eq!(q("Passport yearly 3 mar").repeat.unwrap().every, "year");
        let two = q("Clinic every 2 weeks fri").repeat.unwrap();
        assert_eq!((two.every.as_str(), two.interval), ("week", 2));
        assert_eq!(q("Clinic every 2 weeks fri").title, "Clinic");
        assert!(q("a every 0 weeks").repeat.is_none());
    }

    #[test]
    fn every_weekday_repeats_weekly_on_that_day_and_fills_the_date() {
        let r = q("Gym every mon 18:30 remind 30m");
        assert_eq!(r.title, "Gym");
        let rule = r.repeat.unwrap();
        assert_eq!((rule.every.as_str(), rule.weekdays.clone()), ("week", vec![1]));
        assert_eq!(r.date.as_deref(), Some("2026-11-23"));
        assert_eq!(r.time.as_deref(), Some("18:30"));
        assert_eq!(r.remind_min, Some(30));
    }

    #[test]
    fn remind_lead_times() {
        assert_eq!(q("a remind 1h").remind_min, Some(60));
        assert_eq!(q("a remind 45min").remind_min, Some(45));
        assert_eq!(q("a remind 1d").remind_min, Some(1440));
        assert_eq!(q("a remind 30 minutes").remind_min, None, "only the short forms");
        assert_eq!(q("a remind 99d").remind_min, None, "too far ahead");
        assert_eq!(q("Remind me later").title, "Remind me later");
    }

    #[test]
    fn text_with_no_date_stays_the_title_and_says_so() {
        let r = q("Buy milk");
        assert_eq!(r.title, "Buy milk");
        assert!(!r.has_date);
        assert_eq!(r.date, None);
        assert_eq!(r.summary, "Buy milk - no date understood, all day");
    }

    #[test]
    fn no_time_means_all_day() {
        let r = q("Holiday 24 dec");
        assert_eq!(r.time, None);
        assert!(r.summary.ends_with("all day"));
    }

    #[test]
    fn other_languages_are_not_guessed() {
        let r = q("Mesyuarat 25 disember");
        assert_eq!(r.date, None, "a non-English month name is left alone");
        assert_eq!(r.title, "Mesyuarat 25 disember");
    }

    #[test]
    fn the_first_of_each_kind_wins_and_the_rest_stay_in_the_title() {
        let r = q("Lunch fri sat");
        assert_eq!(r.date.as_deref(), Some("2026-11-27"));
        assert_eq!(r.title, "Lunch sat");
    }

    #[test]
    fn empty_and_odd_input_never_panics() {
        assert_eq!(q("").title, "");
        assert!(q("").summary.starts_with("(no title yet)"));
        for text in ["every", "next", "at", "remind", "25", "dec", "every 2", "::", "3pm 3pm", "\u{0}\u{1f600}", &"x ".repeat(5000)] {
            let _ = q(text);
        }
    }

    #[test]
    fn only_a_title_left_when_everything_else_is_understood() {
        assert_eq!(q("fri 3pm").title, "");
    }
}
