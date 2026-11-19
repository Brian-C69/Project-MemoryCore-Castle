//! STUDIO's "Write prompt from picture": one picture in, one image prompt out, the same way a picture
//! would be drawn (`env::picture_way`) - the member's OpenAI key, or Codex signed in with ChatGPT.
//! Nothing is drawn and nothing is kept; the words go back to the page to be changed.

use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The model that reads the picture on the key way (OpenAI's vision guide, Responses API, Nov 2026).
pub const VISION_MODEL: &str = "gpt-6.2-astra";
const OPENAI: &str = "https://api.openai.com";
const MAX_PICTURE_BYTES: u64 = 25 * 1024 * 1024;
const TIME_LIMIT: Duration = Duration::from_secs(120);
/// What the AI is asked; the same words on both ways.
pub const INSTRUCTION: &str = "Write one image-generation prompt that would recreate this picture: the subject, the art style, the composition and framing, the light, and the colours. Describe people by how they look, never by a real person's name. Answer with the prompt alone, in one paragraph, no title and no quotes.";
pub const NO_WAY: &str = "Add the OpenAI key in Settings > Keys, or sign in to Codex with ChatGPT, to read a picture.";

/// A picture the reader may send: PNG, JPEG or WebP, at most 25 MB, as a data URL.
pub fn data_url(path: &Path) -> Result<String, String> {
    let lower = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let kind = match lower.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return Err("Choose a PNG, JPEG or WebP picture.".into()),
    };
    let size = std::fs::metadata(path).map_err(|_| "That picture was not found.".to_string())?.len();
    if size > MAX_PICTURE_BYTES {
        return Err("That picture is larger than 25 MB.".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "That picture could not be read.".to_string())?;
    Ok(format!("data:{kind};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// The Responses API request for one picture.
pub fn request_body(data_url: &str) -> Value {
    json!({
        "model": VISION_MODEL,
        "input": [{
            "role": "user",
            "content": [
                { "type": "input_text", "text": INSTRUCTION },
                { "type": "input_image", "image_url": data_url, "detail": "auto" }
            ]
        }]
    })
}

/// The answer's words: every `output_text` part of the message items, joined.
pub fn text_of(answer: &Value) -> Option<String> {
    let parts: Vec<String> = answer
        .get("output")?
        .as_array()?
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .flat_map(|item| item.get("content").and_then(Value::as_array).cloned().unwrap_or_default())
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("output_text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str).map(str::to_string))
        .collect();
    let text = parts.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// The key way: one Responses API call. `base` is OpenAI's address (a test passes its own).
pub fn ask_openai(base: &str, key: &str, data_url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIME_LIMIT)
        .user_agent(concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| "The picture could not be read just now.".to_string())?;
    let answer = client
        .post(format!("{base}/v1/responses"))
        .bearer_auth(key)
        .json(&request_body(data_url))
        .send()
        .map_err(|_| "OpenAI could not be reached.".to_string())?;
    let status = answer.status().as_u16();
    let body: Value = answer.json().map_err(|_| "OpenAI sent something unreadable.".to_string())?;
    if status == 401 {
        return Err("The OpenAI key was not accepted. Check it in Settings > Keys.".into());
    }
    if !(200..300).contains(&status) {
        // the provider's own words (a refusal, a spent balance), never the key
        let said = body.pointer("/error/message").and_then(Value::as_str).unwrap_or("the request was refused");
        return Err(format!("OpenAI said: {}", said.chars().take(200).collect::<String>()));
    }
    text_of(&body).ok_or_else(|| "OpenAI read the picture but wrote nothing back.".to_string())
}

/// The Codex way's arguments: read-only, the picture attached, the last message written to `out`,
/// the instruction on stdin.
pub fn codex_args(picture: &Path, out: &Path) -> Vec<String> {
    vec![
        "exec".into(),
        "--skip-git-repo-check".into(),
        "--sandbox".into(),
        "read-only".into(),
        "-c".into(),
        "forced_login_method=chatgpt".into(),
        "-c".into(),
        "model_provider=openai".into(),
        "--image".into(),
        picture.display().to_string(),
        "--output-last-message".into(),
        out.display().to_string(),
        "-".into(),
    ]
}

/// The Codex way: one `codex exec` in a folder of its own, like a room turn; the chat is untouched.
fn ask_codex(program: &Path, picture: &Path, work: &Path) -> Result<String, String> {
    use std::io::Write;
    std::fs::create_dir_all(work).map_err(|_| "The picture could not be read just now.".to_string())?;
    let out = work.join("prompt.txt");
    let _ = std::fs::remove_file(&out);
    let through_cmd = program.extension().is_some_and(|x| x.eq_ignore_ascii_case("cmd"));
    // platform: windows-only - a .cmd / .bat launcher exists only on Windows
    let mut command = if through_cmd {
        let mut c = std::process::Command::new("cmd.exe");
        c.arg("/c").arg(program);
        c
    } else {
        std::process::Command::new(program)
    };
    command
        .args(codex_args(picture, &out))
        .current_dir(work)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for name in crate::env::NEVER_INHERITED {
        command.env_remove(name);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|_| "Codex could not start.".to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(INSTRUCTION.as_bytes());
    }
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| "Codex stopped unexpectedly.".to_string())? {
            break status;
        }
        if started.elapsed() > TIME_LIMIT {
            let _ = child.kill();
            return Err("Codex took too long to read the picture.".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    let text = std::fs::read_to_string(&out).unwrap_or_default().trim().to_string();
    if !status.success() || text.is_empty() {
        return Err("Codex could not read the picture. Try again, or add an OpenAI key.".into());
    }
    Ok(text)
}

/// Reads one picture the way open now. `way` is `env::picture_way`'s answer.
pub fn describe(way: &str, picture: &Path, key: Option<String>, work: PathBuf) -> Result<String, String> {
    let url = data_url(picture)?;
    match way {
        "key" => ask_openai(OPENAI, key.as_deref().ok_or(NO_WAY)?, &url),
        "codex" => {
            let program = crate::env::find_on_path("codex").ok_or(NO_WAY)?;
            ask_codex(&program, picture, &work)
        }
        _ => Err(NO_WAY.into()),
    }
}

/// "Write prompt from picture": the picture's prompt, or a plain line saying why not.
#[tauri::command]
pub async fn picture_describe(path: String, store: tauri::State<'_, crate::settings::SettingsStore>) -> Result<String, String> {
    crate::rooms::need(&store, "studio")?;
    let work = store.folder().join("describe");
    tauri::async_runtime::spawn_blocking(move || {
        let key = crate::key::load("openai");
        let way = crate::env::picture_way(crate::providers::codex_chatgpt_cached, key.is_some());
        describe(way, Path::new(&path), key, work)
    })
    .await
    .map_err(|_| "The picture could not be read just now.".to_string())?
}
