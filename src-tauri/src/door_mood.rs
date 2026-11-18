//! The door's mood verbs (`castle-memory mood show | set`): the companion reads today's mood and
//! may change it itself when something truly moves it - at most mood::CHANGES_A_DAY times a day,
//! always with a short why. Castle writes the day's first mood (mood.rs).

use crate::door::Ask;
use crate::mood;
use mysql::Conn;

/// The mood verbs may run before Castle's background migration has made the table: make sure.
fn ready(conn: &mut Conn) -> Result<(), String> {
    crate::memdb::migrate(conn).map(|_| ())
}

/// "MOOD sweet, gentle, dreamy - Soft and unhurried. (daily, 08:01)"
fn line(m: &mood::Mood) -> String {
    let why = m.why.as_deref().map(|w| format!(" - why: {w}")).unwrap_or_default();
    format!("MOOD {} - {} ({}, {}){why}", m.words.join(", "), m.tone, m.source, m.at)
}

pub(crate) fn mood_show(conn: &mut Conn) -> Result<String, String> {
    ready(conn)?;
    let all = mood::today_all(conn)?;
    if all.is_empty() {
        return Ok("MOOD none yet today - Castle writes it at the day's first start".into());
    }
    let left = mood::CHANGES_A_DAY.saturating_sub(mood::changes_today(conn)?);
    Ok(format!("{}\nChanges left today: {left}", all.iter().map(line).collect::<Vec<_>>().join("\n")))
}

/// The words as the door takes them: "a, b, c" (commas, or spaces when there is no comma). Pure, tested.
pub fn split_words(text: &str) -> Vec<String> {
    let parts: Vec<&str> = if text.contains(',') { text.split(',').collect() } else { text.split_whitespace().collect() };
    parts.into_iter().map(str::trim).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

pub(crate) fn mood_set(conn: &mut Conn, ask: &Ask) -> Result<String, String> {
    ready(conn)?;
    let words = split_words(ask.get("words").ok_or("--words is needed: 3 to 7 mood words, comma separated.")?);
    let tone = ask.get("tone").ok_or("--tone is needed: one sentence on how you speak now.")?;
    let why = ask.get("why").map(str::trim).filter(|w| !w.is_empty()).ok_or("--why is needed: a few words on what moved you.")?;
    let (words, tone) = mood::check(&words, tone)?;
    if mood::changes_today(conn)? >= mood::CHANGES_A_DAY {
        return Err(format!("Your mood has already changed {} times today - let this one stay until tomorrow.", mood::CHANGES_A_DAY));
    }
    mood::insert(conn, &words, &tone, "companion", None, Some(why))?;
    Ok(format!("MOOD set: {} - {tone}", words.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_come_comma_separated_or_by_spaces() {
        assert_eq!(split_words("sweet, gentle ,  ojou-sama"), vec!["sweet", "gentle", "ojou-sama"]);
        assert_eq!(split_words("sweet gentle dreamy"), vec!["sweet", "gentle", "dreamy"]);
        assert!(split_words("  ,  ").is_empty());
    }
}
