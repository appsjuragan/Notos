use crate::plugin::PluginManager;
use crate::undo_manager::UndoManager;
use eframe::egui;
use std::collections::HashSet;

use crate::dialogs::{CloseConfirmationDialog, FindDialog, GotoLineDialog};
use crate::editor::{EditorTab, TabId};

mod actions;
mod editor_panel;
mod file_ops;
mod session;
mod style;
mod update;

use session::SessionState;
use style::{setup_custom_fonts, setup_custom_style};

pub struct NotosApp {
    tabs: Vec<EditorTab>,
    active_tab_id: Option<TabId>,
    plugin_manager: PluginManager,
    current_cursor_pos: (usize, usize), // Line, Col (1-based)
    find_dialog: FindDialog,
    goto_dialog: GotoLineDialog,
    close_confirmation: CloseConfirmationDialog,
    word_wrap: bool,
    show_line_numbers: bool,
    dark_mode: bool,
    editor_font_size: f32,
    editor_font_family: String,
    custom_fonts: std::collections::HashMap<String, Vec<u8>>,
    recent_files: Vec<std::path::PathBuf>,
    ipc_receiver: std::sync::mpsc::Receiver<String>,
    next_underline: Option<(usize, usize)>,
    hovered_char_idx: Option<usize>,
    last_session_save: std::time::Instant,
    file_load_receiver: std::sync::mpsc::Receiver<(std::path::PathBuf, std::result::Result<EditorTab, String>)>,
    file_load_sender: std::sync::mpsc::Sender<(std::path::PathBuf, std::result::Result<EditorTab, String>)>,
    loading_paths: HashSet<std::path::PathBuf>,
    prev_dark_mode: bool,
    pub(crate) undo_manager: UndoManager,
}

impl NotosApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        args: Vec<String>,
        rx: std::sync::mpsc::Receiver<String>,
    ) -> Self {
        // Initial setup
        setup_custom_fonts(&cc.egui_ctx);

        let (tx_load, rx_load) =
            std::sync::mpsc::channel::<(std::path::PathBuf, std::result::Result<EditorTab, String>)>();

        let mut app = Self {
            tabs: vec![EditorTab::default()],
            active_tab_id: None, // Will be set in init
            plugin_manager: PluginManager::new(),
            current_cursor_pos: (1, 1),
            find_dialog: FindDialog::default(),
            goto_dialog: GotoLineDialog::default(),
            word_wrap: true,
            show_line_numbers: false,
            dark_mode: false,
            editor_font_size: 14.0,
            editor_font_family: "Monospace".to_string(),
            custom_fonts: std::collections::HashMap::new(),
            close_confirmation: CloseConfirmationDialog::default(),
            recent_files: Vec::new(),
            ipc_receiver: rx,
            next_underline: None,
            hovered_char_idx: None,
            last_session_save: std::time::Instant::now(),
            file_load_receiver: rx_load,
            file_load_sender: tx_load,
            loading_paths: HashSet::new(),
            prev_dark_mode: false,
            undo_manager: UndoManager::new(None),
        };

        if let Some(mut session) = SessionState::load() {
            app.undo_manager = UndoManager::new(Some(std::mem::take(&mut session.undo_state)));
            app.tabs = session.tabs;
            app.active_tab_id = session.active_tab_id;
            app.word_wrap = session.word_wrap;
            app.show_line_numbers = session.show_line_numbers;
            app.dark_mode = session.dark_mode;
            app.editor_font_size = session.editor_font_size;
            app.editor_font_family = session.editor_font_family;
            app.custom_fonts = session.custom_fonts;
            app.recent_files = session.recent_files;

            // Restore fonts in egui
            let mut fonts = egui::FontDefinitions::default();
            for (n, b) in &app.custom_fonts {
                fonts
                    .font_data
                    .insert(n.clone(), egui::FontData::from_owned(b.clone()));
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, n.clone());
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, n.clone());
            }
            if !app.custom_fonts.is_empty() {
                cc.egui_ctx.set_fonts(fonts);
            }

            setup_custom_style(&cc.egui_ctx, app.dark_mode);

            // Ensure at least one tab or fix active tab
            if app.tabs.is_empty() {
                let tab = EditorTab::default();
                app.active_tab_id = Some(tab.id);
                app.tabs.push(tab);
            }

            // Mark all tabs to restore cursor, sync ID counter, and refresh skipped fields
            for tab in &mut app.tabs {
                crate::editor::ensure_tab_id_at_least(tab.id.0);
                tab.scroll_to_cursor = true;
                tab.refresh_metadata();
                if !tab.large_file {
                    tab.undo_snapshot = tab.content.clone();
                }
            }

            app.prev_dark_mode = app.dark_mode;
        } else {
            // Default look
            setup_custom_style(&cc.egui_ctx, app.dark_mode);

            if let Some(first) = app.tabs.first() {
                app.active_tab_id = Some(first.id);
            }

            app.prev_dark_mode = app.dark_mode;
        }

        // Load plugins here
        app.plugin_manager.load_plugins();
        app.plugin_manager.on_load(&cc.egui_ctx);

        // Handle command line arguments
        let mut opened_any = false;
        for arg in args {
            let path = std::path::PathBuf::from(arg);
            if path.exists() && path.is_file() && app.open_path(path) {
                opened_any = true;
            }
        }

        // If we opened files from args and we have the default empty untitled tab, remove it
        if opened_any && app.tabs.len() > 1 {
            if let Some(pos) = app
                .tabs
                .iter()
                .position(|t| t.path.is_none() && t.content.is_empty())
            {
                let id = app.tabs[pos].id;
                app.tabs.remove(pos);
                // If we removed the active tab, set it to the last opened one
                if app.active_tab_id == Some(id) {
                    app.active_tab_id = app.tabs.last().map(|t| t.id);
                }
            }
        }

        app
    }

    fn active_tab_mut(&mut self) -> Option<&mut EditorTab> {
        self.tabs
            .iter_mut()
            .find(|t| Some(t.id) == self.active_tab_id)
    }
    pub fn save_session(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let undo_state = self.undo_manager.export_persistent_state();
        SessionState::save(
            &self.tabs,
            self.active_tab_id,
            self.word_wrap,
            self.show_line_numbers,
            self.dark_mode,
            self.editor_font_size,
            &self.editor_font_family,
            &self.custom_fonts,
            &self.recent_files,
            undo_state,
        )
    }
}

fn get_ed_ctx(
    tabs: &[EditorTab],
    active_tab_id: Option<TabId>,
    hovered_char_idx: Option<usize>,
) -> notos_sdk::EditorContext<'_> {
    if let Some(tab) = tabs.iter().find(|t| Some(t.id) == active_tab_id) {
        notos_sdk::EditorContext {
            content: &tab.content,
            selection: tab.cursor_range,
            hovered_char_idx,
            file_path: tab.path.as_deref(),
        }
    } else {
        notos_sdk::EditorContext {
            content: "",
            selection: None,
            hovered_char_idx,
            file_path: None,
        }
    }
}
