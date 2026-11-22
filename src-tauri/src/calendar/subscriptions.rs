//! CALENDAR phase 7: calendars the member subscribes to, such as public holidays. Read only: Castle
//! fetches the file once a day, keeps a copy in its own database and shows it; nothing is ever sent
//! about the member, and nothing from the file can become an event, a reminder or an instruction.
//!
//! The one network rule set: HTTPS only, no address typed as an IP number, no login or cookie, at most
//! 2 MB, 30 seconds, and a host that resolves to a private or local address is refused. A failed
//! refresh keeps the last good copy and says so quietly.
use super::ics;
use super::repeat::{self, Exception, Series};
use super::Item;
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use mysql::{prelude::Queryable, Conn};
use serde::Serialize;
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration as Wait, Instant};
use tauri::{AppHandle, Manager};

/// Holidays in Malaysia, from Google Calendar's public calendar. It lists public holidays and other
/// observances together, nationwide and by state, so Castle shows what the file says and no more.
pub const MALAYSIA_URL: &str = "https://calendar.google.com/calendar/ical/en.malaysia%23holiday%40group.v.calendar.google.com/public/basic.ics";
pub const PRESETS: [(&str, &str, &str); 1] = [("malaysia", "Holidays in Malaysia", MALAYSIA_URL)];
pub const MALAYSIA_STATES: [&str; 16] = ["Johor", "Kedah", "Kelantan", "Kuala Lumpur", "Labuan", "Malacca", "Negeri Sembilan", "Pahang", "Penang", "Perak", "Perlis", "Putrajaya", "Sabah", "Sarawak", "Selangor", "Terengganu"];
/// The most calendars one member may subscribe to.
pub const MAX_SUBS: u64 = 5;
const MAX_ROWS: usize = 3000;
const MAX_URL: usize = 2048;
/// How far back and ahead a refresh keeps entries.
const KEEP_BACK_DAYS: i64 = 366;
const KEEP_AHEAD_DAYS: i64 = 731;
/// A refresh is tried once a day; after a failure again after six hours.
const REFRESH_AFTER_HOURS: i64 = 24;
const RETRY_AFTER_HOURS: i64 = 6;
const TIMEOUT: Wait = Wait::from_secs(30);

// ------------------------------------------------------------------------------ what may be fetched

/// An address Castle may fetch, or why not. Pure.
pub fn check_url(text: &str) -> Result<reqwest::Url, String> {
    let bad = || "Use a complete https:// address of a calendar file, with no login in it.".to_string();
    if text.len() > MAX_URL || text.contains(['{', '}', '\r', '\n', ' ']) {
        return Err(bad());
    }
    let url = reqwest::Url::parse(text).map_err(|_| bad())?;
    let host = url.host_str().ok_or_else(bad)?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() || url.port().is_some_and(|p| p != 443) {
        return Err(bad());
    }
    // a name, not a number: an address typed as 192.168.1.5 or [::1] is never a public calendar
    if host.starts_with('[') || host.parse::<IpAddr>().is_ok() || !host.contains('.') || host.eq_ignore_ascii_case("localhost") {
        return Err("Use the calendar's web address with a name in it, not an IP number.".into());
    }
    Ok(url)
}

/// May Castle talk to this address? Loopback, private, link-local and similar are never a public calendar.
pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => !(v.is_loopback() || v.is_private() || v.is_link_local() || v.is_unspecified() || v.is_broadcast() || v.is_documentation() || v.octets()[0] == 0 || (v.octets()[0] == 100 && (64..128).contains(&v.octets()[1]))),
        IpAddr::V6(v) => {
            if let Some(mapped) = v.to_ipv4_mapped() {
                return public_ip(IpAddr::V4(mapped));
            }
            let s = v.segments();
            !(v.is_loopback() || v.is_unspecified() || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80 || (s[0] & 0xff00) == 0xff00)
        }
    }
}

fn host_is_public(url: &reqwest::Url) -> Result<(), String> {
    let host = url.host_str().ok_or("That address has no name.")?;
    let found: Vec<IpAddr> = (host, 443u16).to_socket_addrs().map_err(|_| "That address could not be found. Check the internet connection and the address.".to_string())?.map(|a| a.ip()).collect();
    if found.is_empty() || found.iter().any(|ip| !public_ip(*ip)) {
        return Err("That address points to this computer or a private network, so Castle will not fetch it.".into());
    }
    Ok(())
}

/// Download one calendar file. No cookies, no login, a generic name, at most 2 MB.
pub fn fetch(address: &str) -> Result<String, String> {
    let url = check_url(address)?;
    host_is_public(&url)?;
    let follow = reqwest::redirect::Policy::custom(|attempt| {
        let ok = attempt.previous().len() < 3 && check_url(attempt.url().as_str()).is_ok();
        if ok { attempt.follow() } else { attempt.stop() }
    });
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("MemoryCore-Castle/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .redirect(follow)
        .timeout(TIMEOUT)
        .connect_timeout(Wait::from_secs(10))
        .build()
        .map_err(|_| "The internet connection could not be prepared.".to_string())?;
    let response = client.get(url).header("Accept", "text/calendar, text/plain;q=0.5").send().map_err(|_| "The calendar could not be reached. Castle will try again later.".to_string())?;
    if !response.status().is_success() {
        return Err(format!("The calendar address answered with an error ({}).", response.status().as_u16()));
    }
    if response.content_length().is_some_and(|n| n as usize > ics::MAX_BYTES) {
        return Err("That calendar file is bigger than 2 MB.".into());
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    response.take(ics::MAX_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|_| "The calendar download was interrupted.".to_string())?;
    if bytes.len() > ics::MAX_BYTES {
        return Err("That calendar file is bigger than 2 MB.".into());
    }
    Ok(String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string())
}

// ------------------------------------------------------------------------------------- reading it

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub uid: String,
    pub day: NaiveDate,
    pub end_day: Option<NaiveDate>,
    pub title: String,
    /// `|Selangor|Sabah|`, or empty for the whole country
    pub regions: String,
    pub tentative: bool,
}

/// "Observance in Kelantan, Terengganu" -> `|Kelantan|Terengganu|`; "Observance" alone -> nationwide.
pub fn regions_of(note: &str) -> (String, bool) {
    let first = note.lines().next().unwrap_or("");
    let tentative = note.to_ascii_lowercase().contains("tentative");
    let list = first.split_once(" in ").map(|(_, rest)| rest).unwrap_or("");
    let names: Vec<&str> = list.split(',').map(str::trim).filter(|n| !n.is_empty() && n.len() <= 40).collect();
    let regions = if names.is_empty() { String::new() } else { format!("|{}|", names.join("|")) };
    (regions, tentative)
}

/// The days this calendar file puts in the window: repeats are worked out here, once, so reading is cheap.
pub fn rows_from(text: &str, today: NaiveDate, to_local: &dyn Fn(NaiveDateTime) -> NaiveDateTime) -> (Vec<Row>, Vec<String>) {
    let parsed = ics::parse(text, to_local);
    let (from, to) = (today - Duration::days(KEEP_BACK_DAYS), today + Duration::days(KEEP_AHEAD_DAYS));
    let mut rows: Vec<Row> = Vec::new();
    'all: for item in &parsed.events {
        let e = &item.event;
        let (Ok(start), regions) = (NaiveDateTime::parse_from_str(&e.starts_at, "%Y-%m-%d %H:%M"), regions_of(&e.note)) else { continue };
        let end = e.ends_at.as_deref().and_then(|t| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M").ok());
        let series = Series { start, end, repeat: e.repeat.clone() };
        let skips: Vec<Exception> = item.skips.iter().map(|d| Exception { date: *d, change: repeat::Change::Skip }).collect();
        for o in repeat::occurrences(&series, &skips, from, to) {
            let last = o.last_day();
            rows.push(Row { uid: item.uid.clone(), day: o.start.date(), end_day: (last > o.start.date()).then_some(last), title: e.title.clone(), regions: regions.0.clone(), tentative: regions.1 });
            if rows.len() >= MAX_ROWS {
                break 'all;
            }
        }
    }
    (rows, parsed.warnings)
}

// ---------------------------------------------------------------------------------- the database

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sub {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub preset: String,
    pub enabled: bool,
    pub region: String,
    pub last_try_at: Option<String>,
    pub last_ok_at: Option<String>,
    pub last_error: String,
    pub entries: u64,
}

fn sql(what: &str) -> impl Fn(mysql::Error) -> String + '_ {
    move |_| format!("{what} failed. Start the memory database in Settings > Database.")
}

pub fn list(c: &mut Conn) -> Result<Vec<Sub>, String> {
    let rows: Vec<(u64, String, String, String, bool, String, Option<String>, Option<String>, String, u64)> = c
        .query("SELECT id,name,url,preset,enabled,region,DATE_FORMAT(last_try_at,'%Y-%m-%d %H:%i'),DATE_FORMAT(last_ok_at,'%Y-%m-%d %H:%i'),last_error,entries FROM calendar_subscriptions ORDER BY id")
        .map_err(sql("Reading calendars"))?;
    Ok(rows.into_iter().map(|(id, name, url, preset, enabled, region, last_try_at, last_ok_at, last_error, entries)| Sub { id, name, url, preset, enabled, region, last_try_at, last_ok_at, last_error, entries }).collect())
}

pub fn add(c: &mut Conn, name: &str, url: &str, preset: &str) -> Result<u64, String> {
    let name: String = name.chars().filter(|ch| !ch.is_control()).collect::<String>().trim().chars().take(80).collect();
    if name.is_empty() {
        return Err("Give the calendar a name.".into());
    }
    let url = check_url(url.trim())?.to_string();
    let have: Option<u64> = c.query_first("SELECT COUNT(*) FROM calendar_subscriptions").map_err(sql("Adding the calendar"))?;
    if have.unwrap_or(0) >= MAX_SUBS {
        return Err(format!("At most {MAX_SUBS} calendars can be subscribed to."));
    }
    let same: Option<u64> = c.exec_first("SELECT COUNT(*) FROM calendar_subscriptions WHERE url=?", (&url,)).map_err(sql("Adding the calendar"))?;
    if same.unwrap_or(0) > 0 {
        return Err("That calendar is already added.".into());
    }
    c.exec_drop("INSERT INTO calendar_subscriptions (name,url,preset) VALUES (?,?,?)", (name, url, preset)).map_err(sql("Adding the calendar"))?;
    Ok(c.last_insert_id())
}

pub fn set(c: &mut Conn, id: u64, enabled: bool, region: &str) -> Result<(), String> {
    let preset: Option<String> = c.exec_first("SELECT preset FROM calendar_subscriptions WHERE id=?", (id,)).map_err(sql("Saving"))?;
    let preset = preset.ok_or("That calendar no longer exists.")?;
    let region = region.trim();
    // only the Malaysia calendar knows its states; a made-up name would hide everything regional
    if !region.is_empty() && !(preset == "malaysia" && MALAYSIA_STATES.contains(&region)) {
        return Err("Choose one of the listed states.".into());
    }
    c.exec_drop("UPDATE calendar_subscriptions SET enabled=?,region=? WHERE id=?", (enabled, region, id)).map_err(sql("Saving"))
}

pub fn remove(c: &mut Conn, id: u64) -> Result<(), String> {
    c.exec_drop("DELETE FROM calendar_subscriptions WHERE id=?", (id,)).map_err(sql("Removing the calendar"))?;
    if c.affected_rows() == 0 {
        return Err("That calendar no longer exists.".into());
    }
    Ok(())
}

/// Replace a calendar's copy with fresh rows, all or nothing.
pub fn store_rows(c: &mut Conn, id: u64, rows: &[Row], now: NaiveDateTime) -> Result<(), String> {
    c.query_drop("START TRANSACTION").map_err(sql("Saving the calendar"))?;
    let done = (|| -> Result<(), String> {
        c.exec_drop("DELETE FROM calendar_holidays WHERE sub_id=?", (id,)).map_err(sql("Saving the calendar"))?;
        for chunk in rows.chunks(200) {
            let marks = vec!["(?,?,?,?,?,?,?)"; chunk.len()].join(",");
            let mut params: Vec<mysql::Value> = Vec::new();
            for r in chunk {
                params.extend([id.into(), r.uid.clone().into(), r.day.to_string().into(), r.end_day.map(|d| d.to_string()).into(), r.title.clone().into(), r.regions.clone().into(), r.tentative.into()]);
            }
            c.exec_drop(format!("INSERT IGNORE INTO calendar_holidays (sub_id,uid,day,end_day,title,regions,tentative) VALUES {marks}"), params).map_err(sql("Saving the calendar"))?;
        }
        c.exec_drop("UPDATE calendar_subscriptions SET last_try_at=?,last_ok_at=?,last_error='',entries=? WHERE id=?", (now.format("%Y-%m-%d %H:%M:%S").to_string(), now.format("%Y-%m-%d %H:%M:%S").to_string(), rows.len() as u64, id)).map_err(sql("Saving the calendar"))?;
        Ok(())
    })();
    match done {
        Ok(()) => c.query_drop("COMMIT").map_err(|_| {
            let _ = c.query_drop("ROLLBACK");
            "The calendar could not be saved.".to_string()
        }),
        Err(e) => {
            let _ = c.query_drop("ROLLBACK");
            Err(e)
        }
    }
}

/// A refresh failed: keep the last copy and say so quietly.
pub fn record_failure(c: &mut Conn, id: u64, why: &str, now: NaiveDateTime) {
    let why: String = why.chars().filter(|ch| !ch.is_control()).take(300).collect();
    let _ = c.exec_drop("UPDATE calendar_subscriptions SET last_try_at=?,last_error=? WHERE id=?", (now.format("%Y-%m-%d %H:%M:%S").to_string(), why, id));
}

fn to_local(t: NaiveDateTime) -> NaiveDateTime {
    Local.from_utc_datetime(&t).naive_local()
}

/// Fetch, read and store one calendar now. Returns how many entries it holds.
pub fn refresh(c: &mut Conn, id: u64, today: NaiveDate, now: NaiveDateTime) -> Result<usize, String> {
    let url: Option<String> = c.exec_first("SELECT url FROM calendar_subscriptions WHERE id=?", (id,)).map_err(sql("Refreshing"))?;
    let url = url.ok_or("That calendar no longer exists.")?;
    let result = (|| {
        let text = fetch(&url)?;
        let (rows, _) = rows_from(&text, today, &to_local);
        if rows.is_empty() {
            return Err("That address did not give a calendar with any entries in the coming years.".to_string());
        }
        store_rows(c, id, &rows, now)?;
        Ok(rows.len())
    })();
    if let Err(e) = &result {
        record_failure(c, id, e, now);
    }
    result
}

/// Which calendars are due: never tried, or not tried for a day (six hours after a failure).
pub fn due(c: &mut Conn, now: NaiveDateTime) -> Result<Vec<u64>, String> {
    let rows: Vec<(u64, Option<String>, Option<String>)> = c
        .query("SELECT id,DATE_FORMAT(last_try_at,'%Y-%m-%d %H:%i:%s'),DATE_FORMAT(last_ok_at,'%Y-%m-%d %H:%i:%s') FROM calendar_subscriptions WHERE enabled=1")
        .map_err(sql("Reading calendars"))?;
    let parse = |s: &Option<String>| s.as_deref().and_then(|t| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S").ok());
    Ok(rows
        .iter()
        .filter(|(_, tried, ok)| {
            let (tried, ok) = (parse(tried), parse(ok));
            match (tried, ok) {
                (None, _) => true,
                (Some(t), Some(o)) if o >= t => now - t >= Duration::hours(REFRESH_AFTER_HOURS),
                (Some(t), _) => now - t >= Duration::hours(RETRY_AFTER_HOURS),
            }
        })
        .map(|(id, _, _)| *id)
        .collect())
}

/// The holidays inside a window, as items: one per day each covers. A calendar with a state chosen shows
/// the nationwide days and that state's; with none chosen, the nationwide days only.
pub fn items(c: &mut Conn, from: NaiveDate, to: NaiveDate) -> Result<Vec<Item>, String> {
    let rows: Vec<(u64, String, String, Option<String>, String, bool, String, String)> = c
        .exec(
            "SELECT h.id,h.title,DATE_FORMAT(h.day,'%Y-%m-%d'),DATE_FORMAT(h.end_day,'%Y-%m-%d'),h.regions,h.tentative,s.name,s.region FROM calendar_holidays h JOIN calendar_subscriptions s ON s.id=h.sub_id WHERE s.enabled=1 AND h.day<=? AND COALESCE(h.end_day,h.day)>=? AND (h.regions='' OR (s.region<>'' AND h.regions LIKE CONCAT('%|',s.region,'|%'))) ORDER BY h.day,h.id LIMIT 1500",
            (to.to_string(), from.to_string()),
        )
        .map_err(sql("Reading holidays"))?;
    let mut out = Vec::new();
    for (id, title, day, end_day, regions, tentative, name, _region) in rows {
        let (Ok(first), last) = (NaiveDate::parse_from_str(&day, "%Y-%m-%d"), end_day.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())) else { continue };
        let last = last.unwrap_or(first).min(first + Duration::days(13));
        let place = if regions.is_empty() { "nationwide".to_string() } else { regions.trim_matches('|').replace('|', ", ") };
        let detail = format!("{name} · {place}{}", if tentative { " · date may change" } else { "" });
        let mut d = first;
        while d <= last {
            if d >= from && d <= to {
                let mut item = Item::plain("holiday", id, &title, d.to_string(), None, "ok", "calendar", detail.clone());
                item.all_day = true;
                item.first = d == first;
                out.push(item);
            }
            d += Duration::days(1);
        }
    }
    Ok(out)
}

// -------------------------------------------------------------------------------- the daily tick

static BUSY: AtomicBool = AtomicBool::new(false);
static LAST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);

/// Called from the runner's 15 second tick. At most every five minutes it looks for a calendar that is
/// due, and does the fetching on its own thread so a slow server never delays a reminder.
pub fn tick(app: &AppHandle) {
    {
        let mut last = LAST_CHECK.lock().unwrap_or_else(|p| p.into_inner());
        if last.is_some_and(|t| t.elapsed() < Wait::from_secs(300)) {
            return;
        }
        *last = Some(Instant::now());
    }
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = (|| -> Result<(), String> {
            let settings = app.state::<crate::settings::SettingsStore>();
            if !settings.get().room_on("calendar") {
                return Ok(());
            }
            let mut c = crate::automations::room_connection(&settings)?;
            let ready: Option<String> = c.exec_first("SELECT name FROM schema_migrations WHERE name=?", ("025_calendar_subscriptions",)).map_err(|_| "preparing".to_string())?;
            if ready.is_none() {
                return Ok(());
            }
            let now = Local::now().naive_local();
            for id in due(&mut c, now)? {
                let _ = refresh(&mut c, id, now.date(), now);
            }
            Ok(())
        })();
        BUSY.store(false, Ordering::Release);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn same(t: NaiveDateTime) -> NaiveDateTime {
        t
    }

    #[test]
    fn only_a_named_https_address_with_no_login_is_fetched() {
        assert!(check_url(MALAYSIA_URL).is_ok());
        assert!(check_url("https://www.officeholidays.com/ics/malaysia").is_ok());
        for bad in [
            "http://example.com/a.ics",
            "ftp://example.com/a.ics",
            "https://user:pass@example.com/a.ics",
            "https://example.com:8443/a.ics",
            "https://127.0.0.1/a.ics",
            "https://192.168.1.5/a.ics",
            "https://[::1]/a.ics",
            "https://localhost/a.ics",
            "https://intranet/a.ics",
            "https://example.com/a.ics#frag",
            "https://example.com/{{payload}}",
            "https://exa mple.com/a.ics",
            "file:///C:/a.ics",
            "javascript:alert(1)",
            "",
            "not a url",
        ] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
        assert!(check_url(&format!("https://example.com/{}", "a".repeat(3000))).is_err());
    }

    #[test]
    fn private_and_local_hosts_are_never_public() {
        for ok in ["8.8.8.8", "142.250.4.100", "2001:4860:4860::8888"] {
            assert!(public_ip(ok.parse().unwrap()), "{ok}");
        }
        for no in ["127.0.0.1", "10.0.0.5", "172.16.3.4", "192.168.0.1", "169.254.1.1", "0.0.0.0", "100.64.0.1", "::1", "fe80::1", "fc00::1", "fd12::1", "::ffff:127.0.0.1", "::ffff:10.1.1.1", "ff02::1"] {
            assert!(!public_ip(no.parse().unwrap()), "{no}");
        }
    }

    #[test]
    fn regions_come_from_the_description_and_nationwide_has_none() {
        assert_eq!(regions_of("Observance\nTo hide observances, go to Settings"), (String::new(), false));
        assert_eq!(regions_of("Observance in Kelantan, Terengganu\nTo hide observances").0, "|Kelantan|Terengganu|");
        assert_eq!(regions_of("Observance in Kuala Lumpur, Labuan, Putrajaya").0, "|Kuala Lumpur|Labuan|Putrajaya|");
        assert_eq!(regions_of("Observance\nDate is tentative and may change."), (String::new(), true));
        assert_eq!(regions_of(""), (String::new(), false));
    }

    fn google_like() -> String {
        let one = |uid: &str, date: &str, end: &str, title: &str, desc: &str| format!("BEGIN:VEVENT\r\nDTSTART;VALUE=DATE:{date}\r\nDTEND;VALUE=DATE:{end}\r\nUID:{uid}\r\nDESCRIPTION:{desc}\r\nSUMMARY:{title}\r\nEND:VEVENT\r\n");
        format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nX-WR-CALNAME:Holidays in Malaysia\r\n{}{}{}{}{}{}END:VCALENDAR\r\n",
            one("a1", "20261231", "20270101", "New Year's Eve", "Observance\\nTo hide observances\\, go to Settings"),
            one("a2", "20260916", "20260917", "Malaysia Day", "Observance\\nTo hide observances"),
            one("a3", "20261108", "20261109", "Diwali (regional holiday)", "Observance in Selangor\\, Sabah\\nTo hide"),
            one("a4", "20270301", "20270302", "Hari Raya Puasa", "Observance\\nDate is tentative and may change."),
            one("a5", "20190101", "20190102", "Long ago", "Observance"),
            one("a6", "20301225", "20301226", "Too far ahead", "Observance"),
        )
    }

    #[test]
    fn a_google_style_file_becomes_rows_inside_the_window_only() {
        let (rows, warnings) = rows_from(&google_like(), day("2026-11-22"), &same);
        assert!(warnings.is_empty(), "{warnings:?}");
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"New Year's Eve") && titles.contains(&"Malaysia Day") && titles.contains(&"Hari Raya Puasa"));
        assert!(!titles.contains(&"Long ago") && !titles.contains(&"Too far ahead"), "{titles:?}");
        let diwali = rows.iter().find(|r| r.title.starts_with("Diwali")).unwrap();
        assert_eq!(diwali.regions, "|Selangor|Sabah|");
        assert!(rows.iter().find(|r| r.title == "Hari Raya Puasa").unwrap().tentative);
        assert_eq!(rows.iter().find(|r| r.title == "Malaysia Day").unwrap().regions, "");
    }

    #[test]
    fn a_repeating_entry_is_worked_out_once_into_days() {
        let text = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:r\r\nDTSTART;VALUE=DATE:20260101\r\nDTEND;VALUE=DATE:20260102\r\nRRULE:FREQ=YEARLY\r\nSUMMARY:New Year\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let (rows, _) = rows_from(text, day("2026-11-22"), &same);
        let days: Vec<String> = rows.iter().map(|r| r.day.to_string()).collect();
        assert_eq!(days, vec!["2026-01-01", "2027-01-01", "2028-01-01"], "a year back and two ahead: {days:?}");
    }

    #[test]
    fn a_hostile_file_cannot_make_more_than_the_row_cap() {
        let body: String = (0..MAX_ROWS + 500).map(|n| format!("BEGIN:VEVENT\r\nUID:e{n}\r\nDTSTART;VALUE=DATE:20270105\r\nSUMMARY:E{n}\r\nEND:VEVENT\r\n")).collect();
        let (rows, _) = rows_from(&format!("BEGIN:VCALENDAR\r\n{body}END:VCALENDAR\r\n"), day("2026-11-22"), &same);
        assert!(rows.len() <= ics::MAX_EVENTS);
    }

    #[test]
    fn the_malaysia_preset_and_states_are_what_the_panel_offers() {
        assert_eq!(PRESETS[0].0, "malaysia");
        assert_eq!(MALAYSIA_STATES.len(), 16);
        assert!(MALAYSIA_STATES.windows(2).all(|w| w[0] < w[1]), "kept in order so the list reads well");
        assert!(check_url(PRESETS[0].2).is_ok());
    }

    #[test]
    fn the_migration_has_the_two_tables() {
        let found = crate::memdb::statements(include_str!("../../resources/schema/025_calendar_subscriptions.sql"));
        assert_eq!(found.len(), 2);
        assert!(found[0].starts_with("CREATE TABLE calendar_subscriptions") && found[1].starts_with("CREATE TABLE calendar_holidays"));
    }
}
