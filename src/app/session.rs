use crate::editor::{EditorTab, TabId};
use serde::{Deserialize, Serialize};
use std::fs;

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
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let filtered_tabs: Vec<EditorTab> = tabs
            .iter()
            .filter(|t| t.path.is_some() || !t.content.is_empty())
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
        };

        let path = std::env::temp_dir().join("notos_session.json");
        let file = fs::File::create(path)?;
        serde_json::to_writer(file, &state)?;
        Ok(())
    }

    pub fn load() -> Option<Self> {
        let path = std::env::temp_dir().join("notos_session.json");
        if path.exists() {
            if let Ok(file) = fs::File::open(&path) {
                // Try standard deserialization first
                match serde_json::from_reader(file) {
                    Ok(state) => return Some(state),
                    Err(e) => {
                        log::warn!(
                            "Failed to deserialize session: {}. Attempting partial recovery...",
                            e
                        );
                        // Try to at least recover recent files if possible by loading as generic JSON
                        if let Ok(file) = fs::File::open(&path) {
                            if let Ok(json) = serde_json::from_reader::<_, serde_json::Value>(file)
                            {
                                let mut state = SessionState::default();
                                if let Some(recent) =
                                    json.get("recent_files").and_then(|v| v.as_array())
                                {
                                    state.recent_files = recent
                                        .iter()
                                        .filter_map(|v| v.as_str())
                                        .map(std::path::PathBuf::from)
                                        .collect();
                                }
                                // Restore basic view settings
                                if let Some(d) = json.get("dark_mode").and_then(|v| v.as_bool()) {
                                    state.dark_mode = d;
                                }
                                if let Some(w) = json.get("word_wrap").and_then(|v| v.as_bool()) {
                                    state.word_wrap = w;
                                }
                                return Some(state);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
