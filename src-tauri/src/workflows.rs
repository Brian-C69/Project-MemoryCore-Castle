//! STUDIO's workflows: a picture recipe made of blocks, kept as a file. Castle's own come with the
//! app and cannot be changed (Duplicate makes the member's copy); the member's live in folders under
//! the settings folder, one `workflow.json` each, the way themes do. A workflow is never code: this
//! module reads, checks and runs the file. The schema is written out in `docs/workflow-schema.json`.

use crate::providers::{ImageAsk, ImageJob, Images};
use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

pub const FOLDER: &str = "workflows";
const FILE: &str = "workflow.json";
/// Beside a bought workflow: where it came from (`{"source":"bought","listing":"..."}`), kept out of
/// the workflow file itself so the file stays exactly what was sold.
const ORIGIN: &str = "origin.json";
pub const SCHEMA: u64 = 1;
const MAX_BLOCKS: usize = 20;
const MAX_REFERENCES: usize = 5;
const MAX_TEMPLATE: usize = 4000;
const MAX_FILE_BYTES: u64 = 256 * 1024;
/// What a member types into one block.
const MAX_VALUE: usize = 4000;
const MAX_PICTURE_BYTES: u64 = 25 * 1024 * 1024;
/// Under the memory folder, outside STUDIO's shelves: copies of the pictures a run follows (the image
/// worker reads references only from inside the memory).
pub const INPUTS_FOLDER: &str = "studio-inputs";
pub const SIZES: [&str; 3] = ["1024x1024", "1024x1536", "1536x1024"];
pub const QUALITIES: [&str; 3] = ["low", "medium", "high"];
const INPUTS: [&str; 4] = ["picture", "character", "text", "chips"];
const STEPS: [&str; 2] = ["describe", "create"];

/// Castle's own workflows, read-only.
pub const BUILT_INS: [(&str, &str); 4] = [
    ("from-a-picture", include_str!("../resources/workflows/from-a-picture.json")),
    ("two-characters", include_str!("../resources/workflows/two-characters.json")),
    ("character-sheet", include_str!("../resources/workflows/character-sheet.json")),
    ("poster", include_str!("../resources/workflows/poster.json")),
];

/// One card on the Workflows rail.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// "built-in", "yours" or "bought"
    pub source: String,
    /// the marketplace listing a bought workflow came from
    pub listing: Option<String>,
    pub workflow: Option<Value>,
    pub error: Option<String>,
}

/// A thing wrong with a workflow, tied to its block when it belongs to one (the builder shows it there).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub block: Option<String>,
    pub message: String,
}

fn problem(block: Option<&str>, message: impl Into<String>) -> Problem {
    Problem { block: block.map(str::to_string), message: message.into() }
}

/// A folder-safe id: lowercase letters, digits and single dashes.
pub fn is_plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 60
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

/// A block id inside a workflow: letters, digits, `_` and `-`.
fn is_block_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 30 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// "Rin's Two Cats" -> "rins-two-cats".
pub fn id_of(name: &str) -> String {
    let mut id = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if (c == ' ' || c == '-' || c == '_') && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id: String = id.trim_matches('-').chars().take(50).collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() { "my-workflow".into() } else { id }
}

fn is_built_in(id: &str) -> bool {
    BUILT_INS.iter().any(|(b, _)| *b == id)
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// The names a `create` template may use: `{{text}}`, `{{chips}}`, `{{character}}` and
/// `{{character.name}}`. Each `{{...}}` token, trimmed, in order.
pub fn template_names(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        names.push(after[..end].trim().to_string());
        rest = &after[end + 2..];
    }
    names
}

/// Every problem with a workflow, or none. Unknown keys are ignored, as in theme.json.
pub fn validate(w: &Value) -> Vec<Problem> {
    let mut out = Vec::new();
    if !w.is_object() {
        return vec![problem(None, "A workflow is a JSON object.")];
    }
    if w.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
        out.push(problem(None, format!("This workflow needs \"schema\": {SCHEMA}.")));
    }
    let name = str_of(w, "name").trim();
    if name.is_empty() {
        out.push(problem(None, "Give the workflow a name."));
    } else if name.chars().count() > 60 {
        out.push(problem(None, "Keep the name under 60 letters."));
    }
    if str_of(w, "description").chars().count() > 300 {
        out.push(problem(None, "Keep the description under 300 letters."));
    }
    match str_of(w, "power") {
        "image" => {}
        "video" => out.push(problem(None, "Video workflows come later; this Castle makes pictures.")),
        _ => out.push(problem(None, "\"power\" must be \"image\".")),
    }
    let Some(blocks) = w.get("blocks").and_then(Value::as_array) else {
        out.push(problem(None, "A workflow needs its blocks."));
        return out;
    };
    if blocks.is_empty() {
        out.push(problem(None, "A workflow needs its blocks."));
        return out;
    }
    if blocks.len() > MAX_BLOCKS {
        out.push(problem(None, format!("A workflow holds at most {MAX_BLOCKS} blocks.")));
    }

    // first pass: ids and types
    let mut seen = HashSet::new();
    let mut kinds: Vec<(String, String)> = Vec::new();
    for b in blocks {
        let id = str_of(b, "id");
        let kind = str_of(b, "type");
        if !is_block_id(id) {
            out.push(problem(None, format!("A block needs a short id of letters and digits (found \"{id}\").")));
            continue;
        }
        if !seen.insert(id.to_string()) {
            out.push(problem(Some(id), format!("Two blocks are called \"{id}\".")));
            continue;
        }
        if !INPUTS.contains(&kind) && !STEPS.contains(&kind) {
            out.push(problem(Some(id), format!("\"{kind}\" is not a block Castle knows.")));
            continue;
        }
        kinds.push((id.to_string(), kind.to_string()));
    }
    let kind_of = |id: &str| kinds.iter().find(|(i, _)| i == id).map(|(_, k)| k.as_str());

    // second pass: each block's own rules
    let mut creates = 0;
    for b in blocks {
        let id = str_of(b, "id");
        let Some(kind) = kind_of(id) else { continue };
        if str_of(b, "label").chars().count() > 60 {
            out.push(problem(Some(id), "Keep the label under 60 letters."));
        }
        match kind {
            "chips" => {
                let options = b.get("options").and_then(Value::as_array).cloned().unwrap_or_default();
                let words: Vec<&str> = options.iter().filter_map(Value::as_str).map(str::trim).filter(|o| !o.is_empty()).collect();
                if words.is_empty() || words.len() != options.len() {
                    out.push(problem(Some(id), "Choices need at least one option, each a few words."));
                } else if words.len() > 12 || words.iter().any(|o| o.chars().count() > 30) {
                    out.push(problem(Some(id), "Keep to 12 options of under 30 letters."));
                }
            }
            "describe" => {
                if kind_of(str_of(b, "from")) != Some("picture") {
                    out.push(problem(Some(id), "Write prompt from picture must read a picture block."));
                }
                if kind_of(str_of(b, "into")) != Some("text") {
                    out.push(problem(Some(id), "Write prompt from picture must write into a text block."));
                }
            }
            "create" => {
                creates += 1;
                let template = str_of(b, "prompt");
                if template.trim().is_empty() {
                    out.push(problem(Some(id), "The picture step needs its prompt."));
                } else if template.chars().count() > MAX_TEMPLATE {
                    out.push(problem(Some(id), format!("Keep the prompt under {MAX_TEMPLATE} letters.")));
                }
                for name in template_names(template) {
                    let (block, field) = name.split_once('.').map(|(b, f)| (b, Some(f))).unwrap_or((name.as_str(), None));
                    let ok = match (kind_of(block), field) {
                        (Some("text" | "chips"), None) => true,
                        (Some("character"), None | Some("name")) => true,
                        _ => false,
                    };
                    if !ok {
                        out.push(problem(Some(id), format!("The prompt uses {{{{{name}}}}}, which is not a text, choice or character block.")));
                    }
                }
                let references = b.get("references").and_then(Value::as_array).cloned().unwrap_or_default();
                if references.len() > MAX_REFERENCES {
                    out.push(problem(Some(id), format!("A picture can follow at most {MAX_REFERENCES} reference pictures.")));
                }
                for r in &references {
                    let r = r.as_str().unwrap_or("");
                    if !matches!(kind_of(r), Some("character" | "picture")) {
                        out.push(problem(Some(id), format!("\"{r}\" is not a picture or character block to follow.")));
                    }
                }
                let size = b.get("size").and_then(Value::as_str).unwrap_or("1024x1024");
                if !SIZES.contains(&size) {
                    out.push(problem(Some(id), format!("The size must be one of {}.", SIZES.join(", "))));
                }
                let quality = b.get("quality").and_then(Value::as_str).unwrap_or("medium");
                if !QUALITIES.contains(&quality) {
                    out.push(problem(Some(id), "The quality must be low, medium or high."));
                }
            }
            _ => {}
        }
    }
    if creates != 1 {
        out.push(problem(None, "A workflow makes one picture: it needs exactly one picture step."));
    }
    out
}

fn first_problem(w: &Value) -> Option<String> {
    validate(w).into_iter().next().map(|p| match p.block {
        Some(b) => format!("{b}: {}", p.message),
        None => p.message,
    })
}

fn built_in_entries() -> Vec<Entry> {
    BUILT_INS
        .iter()
        .map(|(id, text)| {
            let workflow: Option<Value> = serde_json::from_str(text).ok();
            let error = workflow.as_ref().map_or(Some("Castle's own workflow could not be read.".into()), first_problem);
            Entry { id: id.to_string(), source: "built-in".into(), listing: None, workflow, error }
        })
        .collect()
}

fn read_folder(folder: &Path) -> Entry {
    let id = folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let origin: Option<Value> = fs::read_to_string(folder.join(ORIGIN)).ok().and_then(|t| serde_json::from_str(&t).ok());
    let bought = origin.as_ref().is_some_and(|o| str_of(o, "source") == "bought");
    let listing = origin.as_ref().map(|o| str_of(o, "listing").to_string()).filter(|l| bought && !l.is_empty());
    let source = if bought { "bought" } else { "yours" }.to_string();
    let broken = |error: &str| Entry { id: id.clone(), source: source.clone(), listing: listing.clone(), workflow: None, error: Some(error.into()) };
    let path = folder.join(FILE);
    match fs::metadata(&path) {
        Err(_) => return broken("workflow.json was not found."),
        Ok(m) if m.len() > MAX_FILE_BYTES => return broken("workflow.json is larger than 256 KB."),
        Ok(_) => {}
    }
    let Some(workflow) = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
        return broken("workflow.json could not be read.");
    };
    let error = first_problem(&workflow);
    Entry { id, source, listing, workflow: Some(workflow), error }
}

/// Castle's own first, then every folder under `<settings>/workflows` in name order. A broken one is
/// listed with its reason, never dropped.
pub fn list(settings_folder: &Path) -> Vec<Entry> {
    let mut all = built_in_entries();
    if let Ok(entries) = fs::read_dir(settings_folder.join(FOLDER)) {
        let mut folders: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        folders.sort();
        all.extend(folders.iter().filter(|f| f.file_name().and_then(|n| n.to_str()).is_some_and(is_plain_id)).map(|f| read_folder(f)));
    }
    all
}

/// One workflow by id (Castle's own or the member's), checked.
pub fn find(settings_folder: &Path, id: &str) -> Result<(Entry, Value), String> {
    let entry = if let Some((_, text)) = BUILT_INS.iter().find(|(b, _)| *b == id) {
        let workflow: Value = serde_json::from_str(text).map_err(|_| "Castle's own workflow could not be read.".to_string())?;
        Entry { id: id.into(), source: "built-in".into(), listing: None, workflow: Some(workflow), error: None }
    } else if is_plain_id(id) && settings_folder.join(FOLDER).join(id).is_dir() {
        read_folder(&settings_folder.join(FOLDER).join(id))
    } else {
        return Err("That workflow is not on your rail.".into());
    };
    if let Some(error) = &entry.error {
        return Err(format!("This workflow has a problem: {error}"));
    }
    let workflow = entry.workflow.clone().ok_or("That workflow could not be read.")?;
    Ok((entry, workflow))
}

/// A free id for a new workflow: the name's, with `-2`, `-3` on a clash; never one of Castle's own.
fn free_id(settings_folder: &Path, name: &str) -> String {
    let root = settings_folder.join(FOLDER);
    let base = id_of(name);
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() || is_built_in(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

fn write(settings_folder: &Path, id: &str, workflow: &mut Value) -> Result<(), String> {
    workflow["id"] = Value::String(id.to_string());
    let folder = settings_folder.join(FOLDER).join(id);
    fs::create_dir_all(&folder).map_err(|_| "The workflow folder could not be created.".to_string())?;
    let text = serde_json::to_string_pretty(workflow).map_err(|_| "The workflow could not be written.".to_string())?;
    let temp = folder.join("workflow.json.tmp");
    fs::write(&temp, text).map_err(|_| "workflow.json could not be written.".to_string())?;
    fs::rename(&temp, folder.join(FILE)).map_err(|_| "workflow.json could not be written.".to_string())
}

/// Saves the member's workflow. `replace` names one of theirs to overwrite (Edit); without it a new
/// folder is made from the name. Castle's own and bought ones are never overwritten. Returns the id.
pub fn save(settings_folder: &Path, mut workflow: Value, replace: Option<&str>) -> Result<String, Vec<Problem>> {
    let problems = validate(&workflow);
    if !problems.is_empty() {
        return Err(problems);
    }
    let id = match replace {
        Some(id) => {
            if is_built_in(id) || !is_plain_id(id) {
                return Err(vec![problem(None, "Castle's own workflows cannot be changed; Duplicate makes your copy.")]);
            }
            let folder = settings_folder.join(FOLDER).join(id);
            if folder.join(ORIGIN).is_file() {
                return Err(vec![problem(None, "A bought workflow stays as it was sold; Duplicate makes your copy.")]);
            }
            if !folder.join(FILE).is_file() {
                return Err(vec![problem(None, "That workflow is not one of yours.")]);
            }
            id.to_string()
        }
        None => free_id(settings_folder, str_of(&workflow, "name")),
    };
    write(settings_folder, &id, &mut workflow).map_err(|e| vec![problem(None, e)])?;
    Ok(id)
}

/// The member's own copy of any workflow on the rail, named "<name> (copy)". Returns the new id.
pub fn duplicate(settings_folder: &Path, id: &str) -> Result<String, String> {
    let (_, mut workflow) = find(settings_folder, id)?;
    let name = format!("{} (copy)", str_of(&workflow, "name"));
    let name: String = name.chars().take(60).collect();
    workflow["name"] = Value::String(name.clone());
    let new_id = free_id(settings_folder, &name);
    write(settings_folder, &new_id, &mut workflow)?;
    Ok(new_id)
}

/// Deletes one of the member's workflows (theirs or bought); Castle's own stay.
pub fn delete(settings_folder: &Path, id: &str) -> Result<(), String> {
    if is_built_in(id) {
        return Err("Castle's own workflows stay; you can leave them unused.".into());
    }
    if !is_plain_id(id) {
        return Err("That is not one of your workflows.".into());
    }
    let folder = settings_folder.join(FOLDER).join(id);
    if !folder.is_dir() {
        return Err("That is not one of your workflows.".into());
    }
    fs::remove_dir_all(folder).map_err(|_| "The workflow could not be deleted.".to_string())
}

/// The text a workflow is exported as: the file alone, pretty. A bought one stays where it was bought.
pub fn export_text(settings_folder: &Path, id: &str) -> Result<(String, String), String> {
    let (entry, workflow) = find(settings_folder, id)?;
    if entry.source == "bought" {
        return Err("A bought workflow is for your own use; it cannot be exported.".into());
    }
    let text = serde_json::to_string_pretty(&workflow).map_err(|_| "The workflow could not be written.".to_string())?;
    Ok((format!("{}.json", entry.id), text))
}

/// Reads a workflow file from anywhere and keeps it as the member's own. Returns the new id.
pub fn import_file(settings_folder: &Path, path: &Path) -> Result<String, String> {
    let size = fs::metadata(path).map_err(|_| "That file could not be found.".to_string())?.len();
    if size > MAX_FILE_BYTES {
        return Err("That file is larger than a workflow can be (256 KB).".into());
    }
    let text = fs::read_to_string(path).map_err(|_| "That file could not be read.".to_string())?;
    let workflow: Value = serde_json::from_str(&text).map_err(|_| "That file is not a workflow (it is not JSON).".to_string())?;
    if let Some(error) = first_problem(&workflow) {
        return Err(format!("That workflow cannot be used: {error}"));
    }
    save(settings_folder, workflow, None).map_err(|p| p.first().map(|p| p.message.clone()).unwrap_or_default())
}

#[tauri::command]
pub async fn workflow_list(store: State<'_, SettingsStore>) -> Result<Vec<Entry>, String> {
    crate::rooms::need(&store, "studio")?;
    let folder = store.folder();
    tauri::async_runtime::spawn_blocking(move || list(&folder)).await.map_err(|_| "The workflows could not be read.".to_string())
}

/// Problems for the builder, before a save.
#[tauri::command]
pub fn workflow_check(workflow: Value) -> Vec<Problem> {
    validate(&workflow)
}

#[tauri::command]
pub fn workflow_save(workflow: Value, replace: Option<String>, store: State<SettingsStore>) -> Result<String, Vec<Problem>> {
    crate::rooms::need(&store, "studio").map_err(|e| vec![problem(None, e)])?;
    save(&store.folder(), workflow, replace.as_deref())
}

#[tauri::command]
pub fn workflow_duplicate(id: String, store: State<SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "studio")?;
    duplicate(&store.folder(), &id)
}

#[tauri::command]
pub fn workflow_delete(id: String, store: State<SettingsStore>) -> Result<(), String> {
    crate::rooms::need(&store, "studio")?;
    delete(&store.folder(), &id)
}

/// Where the member's workflows live, for the companion's brief.
#[tauri::command]
pub fn workflow_folder(store: State<SettingsStore>) -> String {
    store.folder().join(FOLDER).display().to_string()
}

/// Saves a workflow where the member chooses. None when the dialog is closed.
#[tauri::command]
pub async fn workflow_export(app: AppHandle, id: String, store: State<'_, SettingsStore>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    crate::rooms::need(&store, "studio")?;
    let (file_name, text) = export_text(&store.folder(), &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let chosen = app.dialog().file().set_title("Export the workflow").set_file_name(&file_name).add_filter("Workflow", &["json"]).blocking_save_file();
        let Some(path) = chosen.and_then(|f| f.into_path().ok()) else { return Ok(None) };
        fs::write(&path, text).map_err(|_| "The workflow could not be saved there.".to_string())?;
        Ok(Some(path.display().to_string()))
    })
    .await
    .map_err(|_| "The export stopped before it finished.".to_string())?
}

/// Picks a workflow file and keeps it as the member's own. None when the dialog is closed.
#[tauri::command]
pub async fn workflow_import(app: AppHandle, store: State<'_, SettingsStore>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    crate::rooms::need(&store, "studio")?;
    let folder = store.folder();
    tauri::async_runtime::spawn_blocking(move || {
        let chosen = app.dialog().file().set_title("Import a workflow").add_filter("Workflow", &["json"]).blocking_pick_file();
        let Some(path) = chosen.and_then(|f| f.into_path().ok()) else { return Ok(None) };
        import_file(&folder, &path).map(Some)
    })
    .await
    .map_err(|_| "The import stopped before it finished.".to_string())?
}

/// What the member filled in for one block: a text or a choice, a character's name, a picture's path.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunValue {
    pub text: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// A run ready for the image queue.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub prompt: String,
    /// the pictures to follow, in the template's order, as the member picked them
    pub references: Vec<String>,
    pub size: String,
    pub quality: String,
    pub label: String,
}

fn block_by_id<'a>(blocks: &'a [Value], id: &str) -> Option<&'a Value> {
    blocks.iter().find(|b| str_of(b, "id") == id)
}

fn label_of(b: &Value) -> String {
    let label = str_of(b, "label").trim();
    if label.is_empty() { str_of(b, "id").to_string() } else { label.to_string() }
}

/// The text one `{{name}}` stands for. A member's words go in as they are: nothing they type is ever
/// read as another `{{...}}`.
fn value_for(blocks: &[Value], values: &HashMap<String, RunValue>, name: &str) -> Result<String, String> {
    let (id, field) = name.split_once('.').map(|(b, f)| (b, Some(f))).unwrap_or((name, None));
    let block = block_by_id(blocks, id).ok_or_else(|| format!("The prompt names {id}, which is not a block."))?;
    let value = values.get(id).cloned().unwrap_or_default();
    let text = match (str_of(block, "type"), field) {
        ("text", None) => {
            let text = value.text.unwrap_or_default().trim().to_string();
            if text.is_empty() {
                return Err(format!("Fill in {}.", label_of(block)));
            }
            text
        }
        ("chips", None) => {
            let options: Vec<&str> = block.get("options").and_then(Value::as_array).map(|o| o.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            match value.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
                None => options.first().map(|o| o.to_string()).unwrap_or_default(),
                Some(chosen) if options.contains(&chosen) => chosen.to_string(),
                Some(_) => return Err(format!("Choose one of the choices in {}.", label_of(block))),
            }
        }
        ("character", None | Some("name")) => value.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| label_of(block)),
        _ => return Err(format!("The prompt cannot use {{{{{name}}}}}.")),
    };
    if text.chars().count() > MAX_VALUE {
        return Err(format!("Keep {} under {MAX_VALUE} letters.", label_of(block)));
    }
    Ok(text)
}

/// Fills the template once, left to right.
fn fill(template: &str, blocks: &[Value], values: &HashMap<String, RunValue>) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        out.push_str(&rest[..start]);
        out.push_str(&value_for(blocks, values, after[..end].trim())?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn is_picture_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str())
}

/// The workflow and what the member filled in, as one picture to ask for. Pure apart from looking at
/// the picked pictures' files.
pub fn prepare(workflow: &Value, values: &HashMap<String, RunValue>) -> Result<Prepared, String> {
    if let Some(error) = first_problem(workflow) {
        return Err(format!("This workflow has a problem: {error}"));
    }
    let blocks = workflow.get("blocks").and_then(Value::as_array).cloned().unwrap_or_default();
    let create = blocks.iter().find(|b| str_of(b, "type") == "create").ok_or("This workflow has no picture step.")?;
    let mut prompt = fill(str_of(create, "prompt"), &blocks, values)?;

    let mut references = Vec::new();
    let mut map = Vec::new();
    for id in create.get("references").and_then(Value::as_array).cloned().unwrap_or_default() {
        let id = id.as_str().unwrap_or("");
        let block = block_by_id(&blocks, id).ok_or("A reference names a block that is not there.")?;
        let value = values.get(id).cloned().unwrap_or_default();
        let picture = value.picture.map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).ok_or_else(|| format!("Add a picture to {}.", label_of(block)))?;
        let path = Path::new(&picture);
        if !is_picture_file(path) {
            return Err(format!("The picture in {} must be a PNG, JPEG or WebP.", label_of(block)));
        }
        let size = fs::metadata(path).map_err(|_| format!("The picture in {} was not found.", label_of(block)))?.len();
        if size > MAX_PICTURE_BYTES {
            return Err(format!("The picture in {} is larger than 25 MB.", label_of(block)));
        }
        references.push(picture);
        let who = if str_of(block, "type") == "character" { value_for(&blocks, values, id)? } else { label_of(block) };
        map.push(format!("{} is {who}", references.len()));
    }
    if !map.is_empty() {
        prompt.push_str(&format!("\n\nReference pictures, in order: {}. Keep each one's face, hair and clothes as in its picture.", map.join("; ")));
    }
    let size = create.get("size").and_then(Value::as_str).unwrap_or("1024x1024").to_string();
    let quality = create.get("quality").and_then(Value::as_str).unwrap_or("medium").to_string();
    prompt.push_str(&format!("\n\nSize {size}, {quality} quality."));
    let label: String = format!("Workflow: {}", str_of(workflow, "name")).chars().take(120).collect();
    Ok(Prepared { prompt, references, size, quality, label })
}

/// Copies the pictures a run follows into `<memory>/studio-inputs/<run>/`, so the image worker (which
/// reads only from inside the memory) can send them; returns the copies' paths, in order.
pub fn keep_inputs(memory: &Path, run: &str, pictures: &[String]) -> Result<Vec<String>, String> {
    if pictures.is_empty() {
        return Ok(Vec::new());
    }
    let folder = memory.join(INPUTS_FOLDER).join(run);
    fs::create_dir_all(&folder).map_err(|_| "The pictures could not be made ready.".to_string())?;
    pictures
        .iter()
        .enumerate()
        .map(|(n, picture)| {
            let source = Path::new(picture);
            let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
            let copy = folder.join(format!("{}.{ext}", n + 1));
            fs::copy(source, &copy).map_err(|_| "A picture could not be made ready.".to_string())?;
            Ok(copy.display().to_string())
        })
        .collect()
}

/// Runs a workflow: fills it from what the member gave and puts its picture on the image queue (the
/// same jobs, progress, Stop and Try again as every STUDIO picture).
#[tauri::command]
pub fn workflow_run(id: String, values: HashMap<String, RunValue>, store: State<SettingsStore>, images: State<Images>) -> Result<ImageJob, String> {
    crate::rooms::need(&store, "studio")?;
    let (_, workflow) = find(&store.folder(), &id)?;
    let prepared = prepare(&workflow, &values)?;
    let memory = store.get().memory_folder.ok_or("Choose a MemoryCore folder first.")?;
    let run = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos().to_string();
    let references = keep_inputs(Path::new(&memory), &run, &prepared.references)?;
    crate::providers::queue_image(
        &store,
        &images,
        ImageAsk {
            brief: prepared.prompt,
            output: None,
            references,
            room: "studio".into(),
            label: Some(prepared.label),
            look: None,
            size: Some(prepared.size),
            quality: Some(prepared.quality),
        },
    )
}
