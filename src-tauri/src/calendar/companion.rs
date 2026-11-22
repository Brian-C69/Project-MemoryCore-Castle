//! The companion's door to the calendar: `castle-memory calendar list | show | propose`.
//! She may read what is on the calendar and suggest an event; the suggestion waits, doing nothing, for
//! her person's yes in CALENDAR. There is no verb to approve, edit, cancel or delete: those do not
//! exist here, so nothing she writes can make an event real, ring a reminder or leave in an export.
use super::repeat::Repeat;
use super::store::{self, Event};
use super::{merge, read_items, window, MAX_ITEMS};
use crate::door::Ask;
use chrono::{Duration, Local, NaiveDate};
use mysql::Conn;

const VERBS: [&str; 3] = ["list", "show", "propose"];
/// How many lines `list` prints; the rest are counted.
const MAX_LINES: usize = 150;
/// A suggestion is for something coming up, not for the far future or the past.
const MAX_AHEAD_DAYS: i64 = 800;
const MAX_WHY: usize = 400;

pub fn door(c: &mut Conn, ask: &Ask) -> Result<String, String> {
    let verb = ask.words.get(1).map(String::as_str).unwrap_or("");
    if !VERBS.contains(&verb) {
        return Err("Calendar verbs: list, show, propose. Approving, editing and cancelling belong to your person in CALENDAR.".into());
    }
    crate::memdb::migrate(c)?;
    let today = Local::now().date_naive();
    match verb {
        "list" => list(c, ask, today),
        "show" => show(c, ask),
        _ => {
            let (event, why) = draft(ask, today)?;
            let ok = store::check(event)?;
            let id = store::propose(c, &ok, &why)?;
            Ok(format!("CALENDAR proposal {id} saved: \"{}\" on {}. It waits for your person's yes in CALENDAR and does nothing until then.", ok.event.title, ok.event.starts_at))
        }
    }
}

fn list(c: &mut Conn, ask: &Ask, today: NaiveDate) -> Result<String, String> {
    let from = ask.get("from").map(str::to_string).unwrap_or_else(|| today.to_string());
    let to = ask.get("to").map(str::to_string).unwrap_or_else(|| (today + Duration::days(13)).to_string());
    let (a, b) = window(&from, &to)?;
    let items = merge(read_items(c, a, b)?, MAX_ITEMS);
    if items.is_empty() {
        return Ok(format!("CALENDAR {a} to {b}: nothing is dated."));
    }
    let total = items.len();
    let mut out = vec![format!("CALENDAR {a} to {b}")];
    for i in items.iter().take(MAX_LINES) {
        let when = i.time.as_deref().map(str::to_string).unwrap_or_else(|| "all day".into());
        let tag = match (i.kind.as_str(), i.state.as_str()) {
            ("event", "proposed") => format!("proposed event #{} - not confirmed", i.id),
            ("event", _) => format!("event #{}{}", i.id, if i.repeating { ", repeats" } else { "" }),
            (kind, state) => format!("{kind}, {state}"),
        };
        out.push(format!("{} {} {} [{}]", i.date, when, i.title, tag));
    }
    if total > MAX_LINES {
        out.push(format!("... and {} more; ask for a shorter range with --from and --to.", total - MAX_LINES));
    }
    Ok(out.join("\n"))
}

fn show(c: &mut Conn, ask: &Ask) -> Result<String, String> {
    let id: u64 = ask.get("id").map(|v| v.trim_start_matches('#')).and_then(|v| v.parse().ok()).ok_or("--id is a number, like 7.")?;
    let e = store::get(c, id)?;
    let repeat = e.repeat.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default()).unwrap_or_else(|| "no".into());
    Ok(format!(
        "EVENT #{} {}\nstate: {}{}\nstarts: {}\nends: {}\nall day: {}\nrepeats: {}\nreminder: {}\nnote: {}",
        e.id,
        e.title,
        e.state,
        if e.state == "proposed" { " (waiting for your person's yes; does nothing yet)" } else { "" },
        e.starts_at,
        e.ends_at.as_deref().unwrap_or("-"),
        if e.all_day { "yes" } else { "no" },
        repeat,
        e.remind_before_min.map(|m| format!("{m} minutes before")).unwrap_or_else(|| "none".into()),
        if e.note.is_empty() { "-" } else { e.note.as_str() },
    ))
}

fn number(ask: &Ask, name: &str) -> Result<Option<u32>, String> {
    match ask.get(name) {
        None => Ok(None),
        Some(v) => v.trim().parse::<u32>().map(Some).map_err(|_| format!("--{name} is a whole number.")),
    }
}

/// Turn the flags of `propose` into an event and the reason she gives. Nothing here touches the
/// database; every limit of a member's own event applies as well, in `store::check`.
pub fn draft(ask: &Ask, today: NaiveDate) -> Result<(Event, String), String> {
    let title = ask.get("title").map(str::trim).filter(|t| !t.is_empty()).ok_or("--title is needed.")?;
    let start = ask.get("start").map(str::trim).filter(|t| !t.is_empty()).ok_or("--start is needed, like \"2026-12-04 15:00\" (a date alone means all day).")?;
    let all_day = !start.contains(' ');
    let day = NaiveDate::parse_from_str(start.split(' ').next().unwrap_or(""), "%Y-%m-%d").map_err(|_| "--start is not a date, use 2026-12-04 or \"2026-12-04 15:00\".".to_string())?;
    if day < today {
        return Err("That date has already passed. Propose something that is still to come.".into());
    }
    if day > today + Duration::days(MAX_AHEAD_DAYS) {
        return Err("That date is too far away to propose.".into());
    }
    let repeat = match ask.get("repeat") {
        None => None,
        Some(every) => Some(Repeat {
            every: every.trim().to_lowercase(),
            interval: number(ask, "interval")?.unwrap_or(1),
            weekdays: match ask.get("weekdays") {
                None => vec![],
                Some(list) => list.split(',').map(|d| d.trim().parse::<u32>().map_err(|_| "--weekdays is numbers 1 (Monday) to 7 (Sunday), like 1,3.".to_string())).collect::<Result<Vec<_>, _>>()?,
            },
            until: ask.get("until").map(|u| u.trim().to_string()),
            count: number(ask, "count")?,
        }),
    };
    if repeat.is_none() && ["interval", "weekdays", "until", "count"].iter().any(|f| ask.get(f).is_some()) {
        return Err("--interval, --weekdays, --until and --count go with --repeat.".into());
    }
    let why: String = ask.get("why").unwrap_or("").chars().filter(|c| !c.is_control()).take(MAX_WHY).collect();
    let event = Event {
        id: 0,
        title: title.to_string(),
        note: String::new(),
        starts_at: if all_day { format!("{start} 00:00") } else { start.to_string() },
        ends_at: ask.get("end").map(|e| e.trim().to_string()).filter(|e| !e.is_empty()),
        all_day,
        repeat,
        remind_before_min: number(ask, "remind-min")?,
        colour: "accent".into(),
        automation_job_id: None,
        automation_lead_min: None,
        state: "proposed".into(),
        created_by: "companion".into(),
    };
    Ok((event, why))
}

/// For tests and the room: what a draft looks like once it has passed every check.
#[cfg(test)]
pub fn checked(ask: &Ask, today: NaiveDate) -> Result<store::Checked, String> {
    store::check(draft(ask, today)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ask(args: &[&str]) -> Ask {
        crate::door::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()
    }

    #[test]
    fn a_timed_proposal_becomes_a_checked_event() {
        let ok = checked(&ask(&["calendar", "propose", "--title", "Dentist", "--start", "2026-11-27 15:00", "--end", "2026-11-27 16:00", "--why", "You mentioned it"]), today()).unwrap();
        assert_eq!(ok.event.title, "Dentist");
        assert_eq!(ok.event.starts_at, "2026-11-27 15:00");
        assert_eq!(ok.event.ends_at.as_deref(), Some("2026-11-27 16:00"));
        assert!(!ok.event.all_day);
    }

    #[test]
    fn a_date_alone_is_all_day() {
        let ok = checked(&ask(&["calendar", "propose", "--title", "Trip", "--start", "2026-12-24"]), today()).unwrap();
        assert!(ok.event.all_day);
        assert_eq!(ok.event.starts_at, "2026-12-24 00:00");
    }

    #[test]
    fn a_repeat_with_weekdays_count_and_reminder() {
        let ok = checked(&ask(&["calendar", "propose", "--title", "Gym", "--start", "2026-11-23 18:30", "--repeat", "week", "--weekdays", "1,3", "--count", "10", "--remind-min", "30"]), today()).unwrap();
        let r = ok.event.repeat.unwrap();
        assert_eq!((r.every.as_str(), r.weekdays.clone(), r.count, r.interval), ("week", vec![1, 3], Some(10), 1));
        assert_eq!(ok.event.remind_before_min, Some(30));
    }

    #[test]
    fn what_is_missing_or_wrong_is_said_plainly() {
        let bad = |args: &[&str]| draft(&ask(args), today()).err().unwrap_or_default();
        assert!(bad(&["calendar", "propose", "--start", "2026-12-01"]).contains("--title"));
        assert!(bad(&["calendar", "propose", "--title", "x"]).contains("--start"));
        assert!(bad(&["calendar", "propose", "--title", "x", "--start", "tomorrow"]).contains("not a date"));
        assert!(bad(&["calendar", "propose", "--title", "x", "--start", "2026-11-21"]).contains("passed"));
        assert!(bad(&["calendar", "propose", "--title", "x", "--start", "2030-01-01"]).contains("too far"));
        assert!(bad(&["calendar", "propose", "--title", "x", "--start", "2026-12-01", "--count", "3"]).contains("go with --repeat"));
        assert!(bad(&["calendar", "propose", "--title", "x", "--start", "2026-12-01", "--repeat", "week", "--weekdays", "mon"]).contains("1 (Monday)"));
        assert!(bad(&["calendar", "propose", "--title", "x", "--start", "2026-12-01", "--repeat", "week", "--count", "lots"]).contains("whole number"));
    }

    #[test]
    fn the_member_limits_still_apply_to_a_proposal() {
        let long = "x".repeat(200);
        assert!(checked(&ask(&["calendar", "propose", "--title", &long, "--start", "2026-12-01"]), today()).is_err());
        assert!(checked(&ask(&["calendar", "propose", "--title", "x", "--start", "2026-12-01", "--repeat", "fortnight"]), today()).is_err());
        assert!(checked(&ask(&["calendar", "propose", "--title", "x", "--start", "2026-12-01 10:00", "--end", "2026-12-01 09:00"]), today()).is_err());
        assert!(checked(&ask(&["calendar", "propose", "--title", "x", "--start", "2026-12-01", "--remind-min", "99999"]), today()).is_err());
    }

    #[test]
    fn a_proposal_is_always_marked_as_hers() {
        let (event, _) = draft(&ask(&["calendar", "propose", "--title", "x", "--start", "2026-12-01"]), today()).unwrap();
        assert_eq!((event.state.as_str(), event.created_by.as_str()), ("proposed", "companion"));
    }

    #[test]
    fn the_reason_is_cleaned_and_bounded() {
        let (_, why) = draft(&ask(&["calendar", "propose", "--title", "x", "--start", "2026-12-01", "--why", &format!("a\u{0}b{}", "y".repeat(900))]), today()).unwrap();
        assert!(why.starts_with("ab") && why.chars().count() == MAX_WHY);
    }

    /// Opt-in isolated database: the generated test database and user are created and dropped here.
    /// `CASTLE_TEST_DB=port:admin:password`, or the installed database's port and stored root key.
    #[test]
    #[ignore]
    fn live_database_proposals_wait_for_a_yes_and_the_companion_cannot_decide() {
        use crate::calendar::notices;
        use mysql::prelude::Queryable;
        let config = std::env::var("CASTLE_TEST_DB").unwrap_or_else(|_| {
            let folder = std::env::var("APPDATA").unwrap() + "/com.memorycore.castle/settings.json";
            let port = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(folder).unwrap()).unwrap()["dbPort"].as_u64().unwrap();
            format!("{port}:root:{}", crate::key::db_root().unwrap_or_default())
        });
        let mut parts = config.splitn(3, ':');
        let port = parts.next().unwrap().parse().unwrap();
        let (user, pass) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &user, &pass, &format!("Proposals Test {}", std::process::id())).unwrap();
        let mut c = crate::memdb::open(&access).unwrap();
        crate::memdb::migrate(&mut c).unwrap();
        let day = Local::now().date_naive() + Duration::days(3);
        let when = |h: &str| format!("{day} {h}");
        let propose = |c: &mut Conn, title: &str, at: &str, extra: &[&str]| {
            let mut args = vec!["calendar", "propose", "--title", title, "--start", at];
            args.extend_from_slice(extra);
            door(c, &ask(&args))
        };

        // she proposes with a reminder; it is saved as hers and waiting
        let said = propose(&mut c, "Dentist", &when("15:00"), &["--remind-min", "30", "--why", "You said Friday"]).unwrap();
        assert!(said.contains("waits for your person's yes"), "{said}");
        let waiting = store::proposals(&mut c).unwrap();
        assert_eq!(waiting.len(), 1);
        let id = waiting[0].id;
        assert_eq!((waiting[0].state.as_str(), waiting[0].created_by.as_str()), ("proposed", "companion"));
        assert_eq!(waiting[0].note, "Why: You said Friday");

        // she can read it, marked as not confirmed, and cannot decide anything
        let text = door(&mut c, &ask(&["calendar", "list"])).unwrap();
        assert!(text.contains("Dentist") && text.contains(&format!("proposed event #{id} - not confirmed")), "{text}");
        assert!(door(&mut c, &ask(&["calendar", "show", "--id", &id.to_string()])).unwrap().contains("waiting for your person's yes"));
        for verb in ["approve", "edit", "cancel", "delete", "dismiss"] {
            let refused = door(&mut c, &ask(&["calendar", verb, "--id", &id.to_string()])).unwrap_err();
            assert!(refused.contains("belong to your person"), "{verb}: {refused}");
        }
        assert_eq!(store::get(&mut c, id).unwrap().state, "proposed", "nothing she tried changed it");

        // a proposal is not a commitment: no reminder at its time, no notice row, not in the reminders list
        let start = chrono::NaiveDateTime::parse_from_str(&when("15:00"), "%Y-%m-%d %H:%M").unwrap();
        let ringing = notices::collect(&mut c, start - Duration::minutes(29)).unwrap();
        assert!(ringing.shown.is_empty() && ringing.counted == 0, "a proposal never rings");
        assert!(store::with_reminders(&mut c).unwrap().is_empty());
        // and it cannot be moved or skipped until approved
        assert!(store::move_whole(&mut c, id, start).is_err());
        assert!(store::change_day(&mut c, id, day, None).is_err());

        // the same suggestion twice is refused, and so is a flood
        assert!(propose(&mut c, "Dentist", &when("15:00"), &[]).unwrap_err().contains("already"));
        for n in 1..store::MAX_PROPOSED {
            propose(&mut c, &format!("Idea {n}"), &when("10:00"), &[]).unwrap();
        }
        assert!(propose(&mut c, "One too many", &when("11:00"), &[]).unwrap_err().contains("Too many"));

        // the member approves it with an edit: it is now an ordinary event, and its reminder works
        let mut edited = store::get(&mut c, id).unwrap();
        edited.title = "Dentist (moved)".into();
        edited.starts_at = format!("{day} 16:00");
        store::decide(&mut c, id, true, Some(&store::check(edited).unwrap())).unwrap();
        let now_real = store::get(&mut c, id).unwrap();
        assert_eq!((now_real.state.as_str(), now_real.title.as_str(), now_real.created_by.as_str()), ("active", "Dentist (moved)", "companion"));
        let later = chrono::NaiveDateTime::parse_from_str(&format!("{day} 16:00"), "%Y-%m-%d %H:%M").unwrap();
        assert_eq!(notices::collect(&mut c, later - Duration::minutes(29)).unwrap().shown.len(), 1, "once approved it reminds");
        assert!(store::decide(&mut c, id, true, None).is_err(), "it is no longer waiting");

        // turning one down: it is gone from the list, and the same suggestion is not made again for a while
        let other = store::proposals(&mut c).unwrap()[0].clone();
        store::decide(&mut c, other.id, false, None).unwrap();
        assert_eq!(store::get(&mut c, other.id).map(|e| e.state).unwrap_or_default(), "", "a cancelled proposal is not shown");
        assert!(propose(&mut c, &other.title, &other.starts_at, &[]).unwrap_err().contains("turned down"), "turned down lately");
        assert_eq!(notices::home(&mut c, chrono::Local::now().naive_local()).unwrap().proposed, store::MAX_PROPOSED - 2);

        c.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
        let mut root = Conn::new(mysql::OptsBuilder::new().ip_or_hostname(Some("127.0.0.1")).tcp_port(port).user(Some(user.as_str())).pass(Some(pass.as_str()))).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root.query_drop(format!("DROP USER IF EXISTS '{}'@'{host}'", access.user)).unwrap();
        }
    }

    #[test]
    fn she_has_no_verb_to_approve_edit_cancel_or_delete() {
        // the door refuses before it ever opens the database, so a throwaway connection is not needed:
        // the verb list is the whole of what she can do
        assert_eq!(VERBS, ["list", "show", "propose"]);
        for verb in ["approve", "edit", "update", "cancel", "delete", "remove", "dismiss", "decide", "move", "skip", "save", "set"] {
            assert!(!VERBS.contains(&verb), "{verb} must not be a calendar verb");
        }
    }
}
