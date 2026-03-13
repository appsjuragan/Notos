use crate::editor::{EditorTab, TabId};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Default)]
pub struct SessionState {
    #[serde(default)]
    pub tabs: Vec<EditorTab>,
    #[serde(default)]
    pub active_tab_id: Option<TabId>,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default)]
    pub show_line_numbers: bool,
    #[serde(default)]
    pub dark_mode: bool,
    #[serde(default = "default_font_size")]
    pub editor_font_size: f32,
    #[serde(default = "default_font_family")]
    pub editor_font_family: String,
    #[serde(default)]
    pub custom_fonts: std::collections::HashMap<String, Vec<u8>>,
    #[serde(default)]
    pub recent_files: Vec<std::path::PathBuf>,
    #[serde(default)]
    pub undo_state: crate::undo_manager::PersistentUndoState,
}

fn default_true() -> bool {
    true
}
fn default_font_size() -> f32 {
    14.0
}
fn default_font_family() -> String {
    "Monospace".to_string()
}

impl SessionState {
    fn session_path() -> std::path::PathBuf {
        std::env::temp_dir().join("notos_session.json.gz.b64")
    }

    fn legacy_path() -> std::path::PathBuf {
        std::env::temp_dir().join("notos_session.json")
    }

    pub fn save(
        tabs: &[EditorTab],
        active_tab_id: Option<TabId>,
        word_wrap: bool,
        show_line_numbers: bool,
        dark_mode: bool,
        editor_font_size: f32,
        editor_font_family: &str,
        custom_fonts: &std::collections::HashMap<String, Vec<u8>>,
        recent_files: &[std::path::PathBuf],
        undo_state: crate::undo_manager::PersistentUndoState,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        const MAX_TAB_SIZE: usize = 128 * 1024 * 1024; // 128 MB

        let filtered_tabs: Vec<EditorTab> = tabs
            .iter()
            .filter(|t| {
                (t.path.is_some() || !t.content.is_empty())
                    && t.content.len() <= MAX_TAB_SIZE
            })
            .cloned()
            .collect();

        let mut active_id = active_tab_id;
        if let Some(id) = active_id {
            if !filtered_tabs.iter().any(|t| t.id == id) {
                active_id = filtered_tabs.last().map(|t| t.id);
            }
        }

        let state = SessionState {
            tabs: filtered_tabs,
            active_tab_id: active_id,
            word_wrap,
            show_line_numbers,
            dark_mode,
            editor_font_size,
            editor_font_family: editor_font_family.to_string(),
            custom_fonts: custom_fonts.clone(),
            recent_files: recent_files.to_vec(),
            undo_state,
        };

        // Serialize to JSON
        let json_bytes = serde_json::to_vec(&state)?;

        // Compress with gzip
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&json_bytes)?;
        let gz_bytes = encoder.finish()?;

        // Encode to base64
        let b64_string = BASE64.encode(&gz_bytes);

        // Write to file
        let path = Self::session_path();
        std::fs::write(&path, b64_string.as_bytes())?;

        // Clean up legacy plain JSON file if it exists
        let legacy = Self::legacy_path();
        if legacy.exists() {
            let _ = std::fs::remove_file(legacy);
        }

        Ok(())
    }

    pub fn load() -> Option<Self> {
        // Try new format first (gz + base64)
        let path = Self::session_path();
        if path.exists() {
            if let Some(state) = Self::load_from_gz_b64(&path) {
                return Some(state);
            }
        }

        // Fall back to legacy plain JSON
        let legacy = Self::legacy_path();
        if legacy.exists() {
            if let Some(state) = Self::load_from_json(&legacy) {
                return Some(state);
            }
        }

        None
    }

    fn load_from_gz_b64(path: &std::path::Path) -> Option<Self> {
        let b64_data = std::fs::read_to_string(path).ok()?;
        let gz_bytes = BASE64.decode(b64_data.trim()).ok()?;
        let mut decoder = GzDecoder::new(&gz_bytes[..]);
        let mut json_bytes = Vec::new();
        decoder.read_to_end(&mut json_bytes).ok()?;

        match serde_json::from_slice::<SessionState>(&json_bytes) {
            Ok(state) => Some(state),
            Err(e) => {
                log::warn!(
                    "Failed to deserialize session (gz): {}. Attempting partial recovery...",
                    e
                );
                Self::partial_recovery_from_bytes(&json_bytes)
            }
        }
    }

    fn load_from_json(path: &std::path::Path) -> Option<Self> {
        let file = std::fs::File::open(path).ok()?;
        match serde_json::from_reader(file) {
            Ok(state) => Some(state),
            Err(e) => {
                log::warn!(
                    "Failed to deserialize session: {}. Attempting partial recovery...",
                    e
                );
                let file = std::fs::File::open(path).ok()?;
                if let Ok(json) = serde_json::from_reader::<_, serde_json::Value>(file) {
                    return Self::partial_recovery_from_value(&json);
                }
                None
            }
        }
    }

    fn partial_recovery_from_bytes(json_bytes: &[u8]) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_slice(json_bytes).ok()?;
        Self::partial_recovery_from_value(&json)
    }

    fn partial_recovery_from_value(json: &serde_json::Value) -> Option<Self> {
        let mut state = SessionState::default();
        if let Some(recent) = json.get("recent_files").and_then(|v| v.as_array()) {
            state.recent_files = recent
                .iter()
                .filter_map(|v| v.as_str())
                .map(std::path::PathBuf::from)
                .collect();
        }
        if let Some(d) = json.get("dark_mode").and_then(|v| v.as_bool()) {
            state.dark_mode = d;
        }
        if let Some(w) = json.get("word_wrap").and_then(|v| v.as_bool()) {
            state.word_wrap = w;
        }
        Some(state)
    }
}
