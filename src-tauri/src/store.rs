//! Castle's own reading of the memory once it lives in the database: the HOME cards and the
//! DIARY and PROJECTS rooms get the same shapes they got from the files, so the page is the same.
//!
//! A row opens in the reader under a made-up path - `db/diary/<id>.md`, `db/project/<id>.md` -
//! that `read` understands. A memory that has a database but has not moved in yet is still read
//! from its files.

use crate::memory::{DiaryEntry, Growth, HomeData, Page, Project, Reminder, Shelf};
use mysql::prelude::Queryable;
use mysql::Conn;
use std::path::Path;

/// The made-up folder a database page opens under.
pub const PAGE_PREFIX: &str = "db/";

/// True once anything of the memory is in the database.
pub fn moved_in(conn: &mut Conn) -> bool {
    conn.query_first::<u64, _>(
        "SELECT (SELECT COUNT(*) FROM imported_files) + (SELECT COUNT(*) FROM diary_entries) + (SELECT COUNT(*) FROM memory_files) \
         + (SELECT COUNT(*) FROM reminders) + (SELECT COUNT(*) FROM projects)",
    )
    .ok()
    .flatten()
    .is_some_and(|n| n > 0)
}

/// The database to read from, when this memory lives there and it answers.
pub fn source(folder: &Path) -> Option<Conn> {
    if !crate::setup::has_database(folder) {
        return None;
    }
    let mut conn = crate::memdb::open_for(folder).ok()?;
    moved_in(&mut conn).then_some(conn)
}

fn diary_page(id: u64) -> String {
    format!("{PAGE_PREFIX}diary/{id}.md")
}

fn project_page(id: u64) -> String {
    format!("{PAGE_PREFIX}project/{id}.md")
}

/// HOME's cards from the rows.
pub fn home(conn: &mut Conn, folder: &Path, today: &str) -> HomeData {
    let mut features = crate::memory::features(folder);
    // these three are rows now, whatever folders are left on disk
    features.diary = true;
    features.projects = true;
    features.reminders = true;

    let reminders: Vec<Reminder> = conn
        .query::<(String, Option<String>, String), _>("SELECT title, DATE_FORMAT(due_date, '%Y-%m-%d'), detail FROM reminders WHERE state = 'open' ORDER BY position, id")
        .unwrap_or_default()
        .into_iter()
        .map(|(title, due, detail)| Reminder { title, due, detail })
        .collect();
    let projects: Vec<Project> = conn
        .query::<(u64, String, Option<String>, Option<String>), _>("SELECT id, name, last_updated, status FROM projects WHERE shelf = 'active' ORDER BY updated_at DESC, id")
        .unwrap_or_default()
        .into_iter()
        .map(|(id, name, updated, status)| Project { name, file: Some(project_page(id)), updated, status })
        .collect();
    let diary = conn
        .query_first::<(u64, String, Option<String>), _>(
            "SELECT id, title, DATE_FORMAT(entry_date, '%Y-%m-%d') FROM diary_entries ORDER BY entry_date IS NULL, entry_date DESC, session_no DESC, id DESC LIMIT 1",
        )
        .ok()
        .flatten()
        .map(|(id, title, date)| DiaryEntry { title, date, file: diary_page(id) });

    let count = |conn: &mut Conn, table: &str| conn.query_first::<u64, _>(format!("SELECT COUNT(*) FROM {table}")).ok().flatten().unwrap_or(0) as usize;
    let diary_entries = count(conn, "diary_entries");
    let first: Option<String> = conn.query_first("SELECT DATE_FORMAT(MIN(entry_date), '%Y-%m-%d') FROM diary_entries").ok().flatten().flatten();
    let growth = Growth {
        memories: count(conn, "memory_files") + count(conn, "session_sections").min(1) + diary_entries + count(conn, "projects"),
        days: first.and_then(|f| crate::memory::days_between(&f, today)),
        skills: count(conn, "skills"),
        diary_entries,
    };
    HomeData { features, reminders, projects, diary, growth }
}

/// The diary, newest first, by month; pages without a date under "Earlier".
pub fn diary_shelves(conn: &mut Conn) -> Vec<Shelf> {
    let rows: Vec<(u64, String, Option<String>)> = conn
        .query("SELECT id, title, DATE_FORMAT(entry_date, '%Y-%m-%d') FROM diary_entries ORDER BY entry_date IS NULL, entry_date DESC, session_no DESC, id DESC")
        .unwrap_or_default();
    let mut shelves: Vec<Shelf> = Vec::new();
    for (id, title, date) in rows {
        let month = date.as_ref().map(|d| d[..7].to_string()).unwrap_or_else(|| "Earlier".into());
        let page = Page { title, file: diary_page(id), date };
        match shelves.iter_mut().find(|s| s.name == month) {
            Some(shelf) => shelf.pages.push(page),
            None => shelves.push(Shelf { name: month, pages: vec![page] }),
        }
    }
    shelves
}

pub fn project_shelves(conn: &mut Conn) -> Vec<Shelf> {
    let rows: Vec<(u64, String, String)> = conn.query("SELECT id, name, shelf FROM projects ORDER BY updated_at DESC, id").unwrap_or_default();
    [("Active", false), ("Archived", true)]
        .iter()
        .map(|(name, archived)| Shelf {
            name: name.to_string(),
            pages: rows.iter().filter(|r| (r.2 == "archived") == *archived).map(|(id, title, _)| Page { title: title.clone(), file: project_page(*id), date: None }).collect(),
        })
        .filter(|s| !s.pages.is_empty())
        .collect()
}

/// (kind, id) of a database page path: "db/diary/12.md" -> ("diary", 12).
pub fn page_of(file: &str) -> Option<(&str, u64)> {
    let rest = file.strip_prefix(PAGE_PREFIX)?;
    let (kind, name) = rest.split_once('/')?;
    let id = name.strip_suffix(".md")?.parse().ok()?;
    matches!(kind, "diary" | "project").then_some((kind, id))
}

/// The text of a database page, for the room's reader.
pub fn read(conn: &mut Conn, file: &str) -> Result<String, String> {
    let (kind, id) = page_of(file).ok_or("That page is not in the database.")?;
    let gone = || "That page is not in the database any more.".to_string();
    match kind {
        "diary" => {
            let row: Option<(u32, String, String, Option<String>, Option<String>)> = conn
                .exec_first("SELECT session_no, title, body, source_file, DATE_FORMAT(entry_date, '%Y-%m-%d') FROM diary_entries WHERE id = ?", (id,))
                .map_err(|_| "The page could not be read.".to_string())?;
            let (n, title, body, source, date) = row.ok_or_else(gone)?;
            // a page that was moved in is its own whole text; one written through the door gets a heading
            Ok(if source.is_some() { body } else { format!("# Daily Diary - {}\n\n## Session {n} - {title}\n{}\n", date.unwrap_or_default(), body.trim_end()) })
        }
        _ => {
            let row: Option<(String, Option<String>, Option<String>, Option<String>)> =
                conn.exec_first("SELECT name, status, summary, body FROM projects WHERE id = ?", (id,)).map_err(|_| "The page could not be read.".to_string())?;
            let (name, status, summary, body) = row.ok_or_else(gone)?;
            Ok(match body.filter(|b| !b.trim().is_empty()) {
                Some(body) => body,
                None => format!("# {name}\n\n**Status:** {}\n\n{}\n", status.unwrap_or_else(|| "-".into()), summary.unwrap_or_default()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_database_page_path_names_its_kind_and_row() {
        assert_eq!(page_of("db/diary/12.md"), Some(("diary", 12)));
        assert_eq!(page_of("db/project/3.md"), Some(("project", 3)));
        for bad in ["daily-diary/current/2026-11-08.md", "db/diary/x.md", "db/skill/1.md", "db/diary/12", "db/diary/../1.md"] {
            assert_eq!(page_of(bad), None, "{bad}");
        }
    }

    /// Against a real server, on demand (CASTLE_TEST_DB): the same memory read from its files
    /// and, after moving in, from the database gives the same cards and shelves.
    #[test]
    #[ignore]
    fn live_the_rooms_read_the_same_from_files_and_from_the_database() {
        let spec = std::env::var("CASTLE_TEST_DB").expect("set CASTLE_TEST_DB");
        let mut parts = spec.splitn(3, ':');
        let port: u16 = parts.next().unwrap().parse().unwrap();
        let (admin, admin_pw) = (parts.next().unwrap().to_string(), parts.next().unwrap_or("").to_string());
        let access = crate::memdb::provision(port, &admin, &admin_pw, &format!("Store Test {}", std::process::id())).unwrap();

        // a copy of the fixture memory, its second listed project given a page
        let root = std::env::temp_dir().join(format!("castle-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        fn copy_dir(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap().flatten() {
                let target = to.join(entry.file_name());
                if entry.path().is_dir() { copy_dir(&entry.path(), &target) } else { std::fs::copy(entry.path(), target).map(|_| ()).unwrap() }
            }
        }
        copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("memory-existing"), &root);
        std::fs::write(root.join("projects/active/study-notes.md"), "# Study notes\n\nChapter 4.\n").unwrap();
        std::fs::create_dir_all(root.join("daily-diary/current/2026-11-10")).unwrap();
        std::fs::write(root.join("daily-diary/current/2026-11-10/diary-1.md"), "# Diary\n\n## The board breathes\nRows.\n").unwrap();

        let today = "2026-11-12";
        assert!(source(&root).is_none());
        let from_files = crate::memory::home(&root, today);
        let file_projects = crate::memory::project_shelves(&root);

        crate::memdb::write_env(&root, &access).unwrap();
        // a database that holds nothing yet: Castle still reads the files
        assert!(source(&root).is_none());
        let mut conn = crate::memdb::open(&access).unwrap();
        crate::import::move_in(&mut conn, &root, || Ok(())).unwrap();
        assert!(!root.join("daily-diary/current/2026-11-08.md").exists());

        let mut conn = source(&root).expect("the database is the source now");
        let from_db = home(&mut conn, &root, today);
        assert_eq!(from_db.reminders, from_files.reminders);
        assert_eq!(from_db.projects.iter().map(|p| (&p.name, &p.updated, &p.status)).collect::<Vec<_>>(), from_files.projects.iter().map(|p| (&p.name, &p.updated, &p.status)).collect::<Vec<_>>());
        // the folder-per-day page is the newest now (the file reader never saw that shape)
        let newest = from_db.diary.clone().unwrap();
        assert_eq!((newest.title.as_str(), newest.date.as_deref()), ("The board breathes", Some("2026-11-10")));
        assert_eq!(from_db.growth.diary_entries, 2);
        // Nov 8 to Nov 12, both days counted - the same answer the files gave
        assert_eq!(from_db.growth.days, Some(5));
        assert_eq!(from_db.growth.days, from_files.growth.days);
        assert!(from_db.features.diary && from_db.features.projects && from_db.features.reminders);
        // through Castle's own entry points
        assert_eq!(crate::memory::home(&root, today), from_db);

        let diary = crate::memory::shelves_of(&root, "diary");
        assert_eq!(diary.len(), 1);
        assert_eq!(diary[0].name, "2026-11");
        assert_eq!(diary[0].pages.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), vec!["The board breathes", "The day we planned the garden"]);
        let projects = crate::memory::shelves_of(&root, "projects");
        assert_eq!(projects.iter().map(|s| (s.name.as_str(), s.pages.len())).collect::<Vec<_>>(), file_projects.iter().map(|s| (s.name.as_str(), s.pages.len())).collect::<Vec<_>>());

        // the reader opens a row as the page it was
        let page = crate::memory::read_page(&root, &diary[0].pages[1].file).unwrap();
        assert_eq!(page, std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/memory-existing/daily-diary/current/2026-11-08.md")).unwrap());
        assert_eq!(crate::memory::read_page(&root, &from_db.projects[0].file.clone().unwrap()).unwrap(), std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/memory-existing/projects/active/portfolio-site.md")).unwrap());
        assert!(crate::memory::read_page(&root, "db/diary/999999.md").is_err());
        // a new entry through the door shows on the card at once
        crate::door::run(&mut conn, &crate::door::parse(&["diary".into(), "add".into(), "--date".into(), "2026-11-12".into(), "--title".into(), "After the move".into(), "--body".into(), "It works.".into()]).unwrap(), &root).unwrap();
        assert_eq!(crate::memory::home(&root, today).diary.unwrap().title, "After the move");

        drop(conn);
        std::fs::remove_dir_all(&root).unwrap();
        let mut root_conn = mysql::Conn::new(mysql::Opts::from(
            mysql::OptsBuilder::new().ip_or_hostname(Some("127.0.0.1")).tcp_port(port).user(Some(admin)).pass(Some(admin_pw)).prefer_socket(false),
        ))
        .unwrap();
        root_conn.query_drop(format!("DROP DATABASE `{}`", access.name)).unwrap();
        for host in ["localhost", "127.0.0.1"] {
            root_conn.query_drop(format!("DROP USER '{}'@'{host}'", access.user)).unwrap();
        }
    }
}
