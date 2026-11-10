//! Create theme: a working theme drafted at once from the member's picture (its colours become
//! the theme's colours, contrast checked), with the Constellation layout. The companion may
//! polish theme.json afterwards; Castle's loader checks every edit before using it.

use crate::settings::SettingsStore;
use crate::themes::FOLDER;
use image::{GenericImageView, Rgb, RgbImage};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::State;

/// The palette read from a picture.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Palette {
    pub background: [u8; 3],
    pub text: [u8; 3],
    pub accent: [u8; 3],
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn rgba(c: [u8; 3], alpha: f32) -> String {
    format!("rgba({}, {}, {}, {alpha})", c[0], c[1], c[2])
}

/// WCAG relative luminance.
pub fn luminance(c: [u8; 3]) -> f64 {
    let lin = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn saturation_value(c: [u8; 3]) -> (f64, f64) {
    let max = *c.iter().max().unwrap() as f64 / 255.0;
    let min = *c.iter().min().unwrap() as f64 / 255.0;
    let s = if max == 0.0 { 0.0 } else { (max - min) / max };
    (s, max)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round().clamp(0.0, 255.0) as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// Background from the darkest tenth of the picture (darkened further so glass panels read),
/// accent from its most vivid bright colours, text a near-white tinted toward the accent -
/// then the text is pushed to plain white until it reads at 7:1 on the background.
pub fn palette_of(pixels: &[[u8; 3]]) -> Palette {
    if pixels.is_empty() {
        return Palette { background: [5, 11, 31], text: [238, 243, 255], accent: [143, 184, 255] };
    }
    let mut by_light: Vec<[u8; 3]> = pixels.to_vec();
    by_light.sort_by(|a, b| luminance(*a).partial_cmp(&luminance(*b)).unwrap());
    let dark = &by_light[..(by_light.len() / 10).max(1)];
    let avg = |set: &[[u8; 3]]| {
        let n = set.len() as f64;
        let s = set.iter().fold([0f64; 3], |acc, c| [acc[0] + c[0] as f64, acc[1] + c[1] as f64, acc[2] + c[2] as f64]);
        [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8]
    };
    let mut background = mix(avg(dark), [0, 0, 0], 0.35);
    // Castle's glass panels are dark by design: a bright picture still gets a dark background
    while luminance(background) > 0.04 {
        background = mix(background, [0, 0, 0], 0.25);
    }
    let mut vivid: Vec<[u8; 3]> = pixels.iter().copied().filter(|c| saturation_value(*c).1 > 0.45).collect();
    vivid.sort_by(|a, b| {
        let score = |c: &[u8; 3]| {
            let (s, v) = saturation_value(*c);
            s * v
        };
        score(b).partial_cmp(&score(a)).unwrap()
    });
    let accent = if vivid.is_empty() { [143, 184, 255] } else { avg(&vivid[..(vivid.len() / 20).max(1)]) };
    // an accent too dark to read on the background is lifted toward white
    let mut accent = accent;
    let mut lift = 0.0;
    while contrast(accent, background) < 4.5 && lift < 1.0 {
        lift += 0.1;
        accent = mix(accent, [255, 255, 255], 0.1);
    }
    let mut text = mix(accent, [255, 255, 255], 0.88);
    while contrast(text, background) < 7.0 && text != [255, 255, 255] {
        text = mix(text, [255, 255, 255], 0.5);
    }
    Palette { background, text, accent }
}

/// The Constellation layout: the panel on the left, the cards down the right.
fn default_layout() -> (Value, Value) {
    (
        json!({ "left": "1.6%", "top": "3%", "bottom": "3%", "width": "31%" }),
        json!({
            "clock": { "right": "21%", "top": "4%", "size": "l" },
            "gauges": { "right": "2%", "top": "4%" },
            "today": { "right": "2%", "top": "22%", "width": "28%" },
            "projects": { "right": "2%", "top": "calc(22% + 190px)", "width": "28%" },
            "diary": { "right": "2%", "top": "calc(22% + 395px)", "width": "28%" },
            "growth": { "right": "2%", "bottom": "4%", "width": "28%" },
            "backup": { "right": "31%", "bottom": "4%", "width": "16%" },
            "database": { "right": "48%", "bottom": "4%", "width": "16%" }
        }),
    )
}

pub fn theme_json(id: &str, name: &str, mood: &str, wallpaper: &str, p: &Palette) -> Value {
    let (panel, widgets) = default_layout();
    let line = mix(p.accent, p.text, 0.3);
    json!({
        "id": id,
        "name": name,
        "description": mood,
        "wallpaper": wallpaper,
        "wallpaperPosition": "center",
        "widgetStyle": "glass",
        "portrait": "panel",
        "colors": {
            "--background": hex(p.background),
            "--bar-background": rgba(p.background, 0.72),
            "--panel-background": rgba(mix(p.background, p.accent, 0.08), 0.62),
            "--widget-background": rgba(mix(p.background, p.accent, 0.1), 0.55),
            "--line": rgba(line, 0.34),
            "--text": hex(p.text),
            "--text-dim": hex(mix(p.text, p.background, 0.32)),
            "--accent": hex(p.accent),
            "--hover": rgba(p.accent, 0.14),
            "--glass-blur": "12px"
        },
        "panel": panel,
        "widgets": widgets
    })
}

/// A folder-safe id from the theme's name: "Rin's Sakura" -> "rins-sakura".
pub fn theme_id(name: &str) -> String {
    let mut id = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if (c == ' ' || c == '-' || c == '_') && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_matches('-').to_string();
    if id.is_empty() { "my-theme".into() } else { id }
}

/// A soft vertical gradient wallpaper, for a theme made without a picture.
fn gradient(p: &Palette) -> RgbImage {
    let (w, h) = (1920u32, 1080u32);
    let top = mix(p.background, [0, 0, 0], 0.3);
    let bottom = mix(p.background, p.accent, 0.35);
    RgbImage::from_fn(w, h, |_, y| {
        let c = mix(top, bottom, (y as f64 / h as f64).powf(1.6));
        Rgb(c)
    })
}

/// Makes `<settings>/themes/<id>/` with the wallpaper and theme.json. Returns the new id.
pub fn create(settings_folder: &Path, name: &str, mood: &str, picture: Option<&Path>) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Give the theme a name.".into());
    }
    let root = settings_folder.join(FOLDER);
    let base = theme_id(name);
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() || id == "constellation" {
        id = format!("{base}-{n}");
        n += 1;
    }
    let folder = root.join(&id);
    std::fs::create_dir_all(&folder).map_err(|_| "The theme folder could not be created.".to_string())?;
    let (wallpaper, palette) = match picture {
        Some(path) => {
            let img = image::open(path).map_err(|_| "That picture could not be read (png, jpg or webp).".to_string())?;
            let small = img.thumbnail(64, 64);
            let pixels: Vec<[u8; 3]> = small.pixels().map(|(_, _, p)| [p[0], p[1], p[2]]).collect();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
            let ext = if ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) { ext } else { "png".into() };
            let file = format!("wallpaper.{ext}");
            std::fs::copy(path, folder.join(&file)).map_err(|_| "The picture could not be copied.".to_string())?;
            (file, palette_of(&pixels))
        }
        None => {
            let p = palette_of(&[]);
            gradient(&p).save(folder.join("wallpaper.png")).map_err(|_| "The wallpaper could not be drawn.".to_string())?;
            ("wallpaper.png".to_string(), p)
        }
    };
    let json = theme_json(&id, name.trim(), mood.trim(), &wallpaper, &palette);
    std::fs::write(folder.join("theme.json"), serde_json::to_string_pretty(&json).unwrap_or_default())
        .map_err(|_| "theme.json could not be written.".to_string())?;
    Ok(id)
}

/// Deletes one of the member's own themes (built-in themes live in the app and cannot be).
pub fn delete(settings_folder: &Path, id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains(['/', '\\', '.']) {
        return Err("That is not one of your themes.".into());
    }
    let folder = settings_folder.join(FOLDER).join(id);
    if !folder.join("theme.json").is_file() {
        return Err("That is not one of your themes.".into());
    }
    std::fs::remove_dir_all(folder).map_err(|_| "The theme could not be deleted.".to_string())
}

pub fn folder_of(settings_folder: &Path, id: &str) -> PathBuf {
    settings_folder.join(FOLDER).join(id)
}

#[tauri::command]
pub fn theme_create(name: String, mood: String, picture: Option<String>, store: State<SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "theme")?;
    create(&store.folder(), &name, &mood, picture.as_deref().map(Path::new))
}

#[tauri::command]
pub fn theme_delete(id: String, store: State<SettingsStore>) -> Result<(), String> {
    crate::rooms::need(&store, "theme")?;
    delete(&store.folder(), &id)
}

/// Where a theme of the member's lives, for the companion's polish brief.
#[tauri::command]
pub fn theme_folder(id: String, store: State<SettingsStore>) -> String {
    folder_of(&store.folder(), &id).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_gives_a_readable_palette() {
        // a dark navy night with a few bright pink flowers
        let mut pixels = vec![[8, 14, 40]; 900];
        pixels.extend(vec![[240, 120, 180]; 60]);
        pixels.extend(vec![[120, 130, 160]; 40]);
        let p = palette_of(&pixels);
        assert!(luminance(p.background) < 0.02, "the background stays dark");
        assert!(p.accent[0] > p.accent[2], "the pink flowers become the accent");
        assert!(contrast(p.text, p.background) >= 7.0);
        assert!(contrast(p.accent, p.background) >= 4.5);
    }

    #[test]
    fn a_bright_picture_still_reads() {
        let p = palette_of(&vec![[250, 245, 235]; 500]);
        assert!(luminance(p.background) <= 0.04);
        assert!(contrast(p.text, p.background) >= 7.0);
    }

    #[test]
    fn ids_are_folder_safe_and_built_ins_are_never_overwritten() {
        assert_eq!(theme_id("Rin's Sakura Night"), "rins-sakura-night");
        assert_eq!(theme_id("  !!  "), "my-theme");
        let root = std::env::temp_dir().join(format!("castle-theme-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let first = create(&root, "Constellation", "my own", None).unwrap();
        assert_eq!(first, "constellation-2");
        let second = create(&root, "Constellation", "again", None).unwrap();
        assert_eq!(second, "constellation-3");
        let json: Value = serde_json::from_str(&std::fs::read_to_string(root.join(FOLDER).join(&first).join("theme.json")).unwrap()).unwrap();
        assert_eq!(json["wallpaper"], "wallpaper.png");
        assert!(json["colors"]["--accent"].as_str().unwrap().starts_with('#'));
        assert!(root.join(FOLDER).join(&first).join("wallpaper.png").is_file());
        assert!(delete(&root, "../x").is_err());
        delete(&root, &first).unwrap();
        assert!(!root.join(FOLDER).join(&first).exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
