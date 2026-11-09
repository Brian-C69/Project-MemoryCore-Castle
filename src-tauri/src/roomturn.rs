//! The member's companion in the room: woken by new lines (anyone but itself), it answers from a
//! room-only run with no tools. Filled in by Phase 17; until then the room only reads.

use serde_json::Value;
use tauri::AppHandle;

/// New lines the reader just heard.
pub fn heard(_app: &AppHandle, _fresh: &[Value]) {}
