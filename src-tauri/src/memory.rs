//! Reading the member's MemoryCore folder: which features are installed. MemoryCore features
//! create their own folders or skills when installed, so the files themselves are the answer
//! (the repo's Feature/ folders say nothing - installs often delete them).

use serde::Serialize;
use std::path::{Path, PathBuf};

/// The MemoryCore features Castle knows how to show, as found in one folder.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    /// daily-diary/ (MemoryCore's diary; always there in a fresh MemoryCore)
    pub diary: bool,
    /// projects/ (LRU Projects)
    pub projects: bool,
    /// library/ (Library)
    pub library: bool,
    /// a save-memory skill from the Skill Plugin System
    pub save_memory: bool,
    /// main/reminders.md (Reminders)
    pub reminders: bool,
    /// adventures/ (Interactive Story; Castle's STORY room creates it when missing)
    pub adventures: bool,
    /// music/ (Song Creation)
    pub music: bool,
    /// media-generation/ (Image / Video Generation)
    pub media: bool,
    /// appearance/ (Castle's APPEARANCE room)
    pub appearance: bool,
}

/// Whether any plugin in `plugins/` carries a skill folder named `skill` with a SKILL.md.
fn has_skill(folder: &Path, skill: &str) -> bool {
    let Ok(plugins) = std::fs::read_dir(folder.join("plugins")) else {
        return false;
    };
    plugins.flatten().any(|plugin| plugin.path().join("skills").join(skill).join("SKILL.md").is_file())
}

pub fn features(folder: &Path) -> Features {
    let dir = |name: &str| folder.join(name).is_dir();
    Features {
        diary: dir("daily-diary"),
        projects: dir("projects"),
        library: dir("library"),
        save_memory: has_skill(folder, "save-memory"),
        reminders: folder.join("main").join("reminders.md").is_file(),
        adventures: dir("adventures"),
        music: dir("music"),
        media: dir("media-generation"),
        appearance: dir("appearance"),
    }
}

// ---------- What the HOME widgets read ----------

/// One open reminder from main/reminders.md: "- **Title** (by 2026-11-10): what".
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub title: String,
    pub due: Option<String>,
    pub detail: String,
}

/// The open reminders, in the file's order (MemoryCore's Reminders feature).
pub fn reminders(folder: &Path) -> Vec<Reminder> {
    let Ok(text) = std::fs::read_to_string(folder.join("main").join("reminders.md")) else {
        return Vec::new();
    };
    let mut open = false;
    let mut found = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            open = heading.trim().eq_ignore_ascii_case("open");
            continue;
        }
        if !open {
            continue;
        }
        let Some(item) = trimmed.strip_prefix("- ") else { continue };
        let item = item.trim();
        let (head, detail) = item.split_once(':').map(|(h, d)| (h.trim(), d.trim())).unwrap_or((item, ""));
        let due = head.split_once("(by ").and_then(|(_, rest)| rest.split(')').next()).map(|d| d.trim().to_string());
        let title = head.split(" (by ").next().unwrap_or(head).trim().trim_matches('*').trim().to_string();
        if !title.is_empty() {
            found.push(Reminder { title, due, detail: detail.to_string() });
        }
    }
    found
}

/// One project row from projects/project-list.md (LRU Projects).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    /// the project file, relative to the memory folder, when the row links one
    pub file: Option<String>,
    pub updated: Option<String>,
    pub status: Option<String>,
}

/// The active projects, most recent first (the list's own order: | Pos | Project | Last Updated | Status |).
pub fn projects(folder: &Path) -> Vec<Project> {
    let Ok(text) = std::fs::read_to_string(folder.join("projects").join("project-list.md")) else {
        return Vec::new();
    };
    let mut in_active = true;
    let mut found = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            in_active = heading.to_lowercase().contains("active");
            continue;
        }
        if !in_active || !trimmed.starts_with('|') || trimmed.contains("---") {
            continue;
        }
        let cells: Vec<&str> = trimmed.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 || cells[0].eq_ignore_ascii_case("pos") || cells[1].eq_ignore_ascii_case("project") {
            continue;
        }
        let cell = cells[1];
        // "[Portfolio site](./active/portfolio-site.md)" or plain text
        let (name, file) = match (cell.find('['), cell.find("]("), cell.rfind(')')) {
            (Some(a), Some(b), Some(c)) if a < b && b < c => {
                let link = cell[b + 2..c].trim_start_matches("./");
                (cell[a + 1..b].to_string(), Some(format!("projects/{link}")))
            }
            _ => (cell.to_string(), None),
        };
        found.push(Project {
            name,
            file,
            updated: cells.get(2).filter(|c| !c.is_empty()).map(|c| c.to_string()),
            status: cells.get(3).filter(|c| !c.is_empty()).map(|c| c.to_string()),
        });
    }
    found
}

/// The newest diary entry, in either diary format.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiaryEntry {
    pub title: String,
    /// "2026-11-08" for the current format; None for the older numbered files
    pub date: Option<String>,
    /// the file, relative to the memory folder
    pub file: String,
}

/// Diary files: daily-diary/current/YYYY-MM-DD.md (Save Diary) and, for memories that never
/// installed it, the older daily-diary/Daily-Diary-NNN.md. Sorted oldest first.
pub fn diary_files(folder: &Path) -> Vec<PathBuf> {
    let diary = folder.join("daily-diary");
    let mut dated: Vec<PathBuf> = std::fs::read_dir(diary.join("current"))
        .map(|e| e.flatten().map(|e| e.path()).filter(|p| is_dated_md(p)).collect())
        .unwrap_or_default();
    dated.sort();
    let mut numbered: Vec<PathBuf> = std::fs::read_dir(&diary)
        .map(|e| {
            e.flatten()
                .map(|e| e.path())
                .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("Daily-Diary-") && n.ends_with(".md")))
                .collect()
        })
        .unwrap_or_default();
    numbered.sort();
    numbered.extend(dated);
    numbered
}

fn is_dated_md(path: &Path) -> bool {
    let Some(stem) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".md")) else {
        return false;
    };
    stem.len() == 10 && stem.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
}

/// The entry's title: its last "## " heading (the newest session), else its "# " heading.
pub fn diary_title(text: &str) -> Option<String> {
    let last_session = text.lines().filter_map(|l| l.trim().strip_prefix("## ")).last();
    let title = last_session.or_else(|| text.lines().find_map(|l| l.trim().strip_prefix("# ")))?.trim();
    // "Session 1 - The day we planned the garden" -> "The day we planned the garden"
    let title = title
        .split_once(" - ")
        .filter(|(head, _)| head.to_lowercase().starts_with("session"))
        .map(|(_, rest)| rest)
        .unwrap_or(title);
    (!title.is_empty()).then(|| title.to_string())
}

pub fn newest_diary(folder: &Path) -> Option<DiaryEntry> {
    let path = diary_files(folder).pop()?;
    let text = std::fs::read_to_string(&path).ok()?;
    let name = path.file_name()?.to_str()?.to_string();
    let date = is_dated_md(&path).then(|| name.trim_end_matches(".md").to_string());
    let file = path.strip_prefix(folder).ok()?.display().to_string().replace('\\', "/");
    Some(DiaryEntry { title: diary_title(&text).unwrap_or_else(|| name.clone()), date, file })
}

/// How the memory has grown: memory files, days since the first diary, skills.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Growth {
    pub memories: usize,
    pub days: Option<u64>,
    pub skills: usize,
    pub diary_entries: usize,
}

/// Markdown files that are the companion's memory - not MemoryCore's feature installers, not the
/// git folder, not node_modules, not the plugins.
fn count_memory_files(folder: &Path, depth: usize) -> usize {
    let Ok(entries) = std::fs::read_dir(folder) else { return 0 };
    entries
        .flatten()
        .map(|e| e.path())
        .map(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            if p.is_dir() {
                if depth >= 6 || name.starts_with('.') || ["Feature", "node_modules", "plugins"].contains(&name.as_str()) {
                    0
                } else {
                    count_memory_files(&p, depth + 1)
                }
            } else if name.ends_with(".md") && !["README.md", "LICENSE.md", "SECURITY.md", "CLAUDE.md", "AGENTS.md"].contains(&name.as_str()) {
                1
            } else {
                0
            }
        })
        .sum()
}

fn skills(folder: &Path) -> usize {
    std::fs::read_dir(folder.join("plugins"))
        .map(|plugins| {
            plugins
                .flatten()
                .flat_map(|p| std::fs::read_dir(p.path().join("skills")).into_iter().flatten().flatten())
                .filter(|s| s.path().join("SKILL.md").is_file())
                .count()
        })
        .unwrap_or(0)
}

/// Days from `first` (YYYY-MM-DD) to `today` (YYYY-MM-DD), counting the first day as day 1.
pub fn days_between(first: &str, today: &str) -> Option<u64> {
    fn day_number(date: &str) -> Option<i64> {
        let mut parts = date.split('-').map(|p| p.parse::<i64>().ok());
        let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
        // days since 1970-01-01 (civil calendar)
        let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * m + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(era * 146_097 + doe - 719_468)
    }
    let span = day_number(today)? - day_number(first)?;
    (span >= 0).then(|| span as u64 + 1)
}

pub fn growth(folder: &Path, today: &str) -> Growth {
    let diary = diary_files(folder);
    let first_dated = diary.iter().find(|p| is_dated_md(p)).and_then(|p| p.file_stem()?.to_str().map(str::to_string));
    Growth {
        memories: count_memory_files(folder, 0),
        days: first_dated.and_then(|first| days_between(&first, today)),
        skills: skills(folder),
        diary_entries: diary.len(),
    }
}

/// Everything HOME shows, read in one go.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeData {
    pub features: Features,
    pub reminders: Vec<Reminder>,
    pub projects: Vec<Project>,
    pub diary: Option<DiaryEntry>,
    pub growth: Growth,
}

pub fn home(folder: &Path, today: &str) -> HomeData {
    HomeData {
        features: features(folder),
        reminders: reminders(folder),
        projects: projects(folder),
        diary: newest_diary(folder),
        growth: growth(folder, today),
    }
}

/// HOME's widgets. `today` is the page's local date (YYYY-MM-DD), so days count in the member's time.
#[tauri::command]
pub fn memory_home(today: String, store: tauri::State<crate::settings::SettingsStore>) -> Option<HomeData> {
    store.get().memory_folder.map(|folder| home(Path::new(&folder), &today))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(name)
    }

    #[test]
    fn the_existing_memory_fills_every_widget() {
        let home = home(&fixture("memory-existing"), "2026-11-09");
        assert_eq!(home.reminders.len(), 2, "completed reminders are left out");
        assert_eq!(home.reminders[0].title, "Finish portfolio homepage");
        assert_eq!(home.reminders[0].due.as_deref(), Some("2026-11-10"));
        assert_eq!(home.reminders[1].due, None);
        assert_eq!(home.projects.len(), 2);
        assert_eq!(home.projects[0].name, "Portfolio site");
        assert_eq!(home.projects[0].file.as_deref(), Some("projects/active/portfolio-site.md"));
        let diary = home.diary.unwrap();
        assert_eq!(diary.title, "The day we planned the garden");
        assert_eq!(diary.date.as_deref(), Some("2026-11-08"));
        assert_eq!(home.growth.days, Some(2));
        assert!(home.growth.memories >= 6);
        assert!(home.features.projects && home.features.reminders);
    }

    #[test]
    fn the_fresh_memory_shows_empty_widgets_without_errors() {
        let home = home(&fixture("memory-fresh"), "2026-11-09");
        assert!(home.reminders.is_empty() && home.projects.is_empty() && home.diary.is_none());
        assert_eq!(home.growth.days, None);
    }

    #[test]
    fn the_older_numbered_diary_format_is_read_too() {
        let root = std::env::temp_dir().join(format!("castle-old-diary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("daily-diary")).unwrap();
        std::fs::write(root.join("daily-diary").join("Daily-Diary-001.md"), "# Daily Diary - First Day\nhello\n").unwrap();
        let entry = newest_diary(&root).unwrap();
        assert_eq!(entry.title, "Daily Diary - First Day");
        assert_eq!(entry.date, None);
        assert_eq!(entry.file, "daily-diary/Daily-Diary-001.md");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn days_are_counted_across_months_and_years() {
        assert_eq!(days_between("2026-11-08", "2026-11-09"), Some(2));
        assert_eq!(days_between("2025-12-31", "2026-01-01"), Some(2));
        assert_eq!(days_between("2024-02-28", "2024-03-01"), Some(3), "a leap day counts");
        assert_eq!(days_between("2026-11-10", "2026-11-09"), None);
    }

    #[test]
    fn features_are_found_by_what_they_create() {
        let root = std::env::temp_dir().join(format!("castle-features-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("daily-diary")).unwrap();
        assert_eq!(features(&root), Features { diary: true, ..Features::default() });

        std::fs::create_dir_all(root.join("projects").join("active")).unwrap();
        std::fs::create_dir_all(root.join("main")).unwrap();
        std::fs::write(root.join("main").join("reminders.md"), "## Open\n").unwrap();
        let skill = root.join("plugins").join("rin-skills").join("skills").join("save-memory");
        std::fs::create_dir_all(&skill).unwrap();
        // a skill folder without its SKILL.md is not installed
        assert!(!features(&root).save_memory);
        std::fs::write(skill.join("SKILL.md"), "---\nname: save-memory\n---\n").unwrap();
        let found = features(&root);
        assert!(found.projects && found.reminders && found.save_memory);
        assert!(!found.library && !found.music);
        let _ = std::fs::remove_dir_all(root);
    }
}
