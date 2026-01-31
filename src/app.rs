use crate::editor::EditorTab;
use crate::plugin::PluginManager;
use eframe::egui;
use rfd::FileDialog;
// use std::path::PathBuf;

use crate::dialogs::{CloseConfirmationDialog, FindDialog, GotoLineDialog};
use serde::{Deserialize, Serialize}; // Added
use std::fs; // Added

#[derive(Serialize, Deserialize)]
struct SessionState {
    tabs: Vec<EditorTab>,
    active_tab_id: Option<uuid::Uuid>,
    word_wrap: bool,
    show_line_numbers: bool,
    dark_mode: bool,
    editor_font_size: f32,
    editor_font_family: String,
    custom_fonts: std::collections::HashMap<String, Vec<u8>>,
    #[serde(default)]
    recent_files: Vec<std::path::PathBuf>,
}

pub struct NotosApp {
    tabs: Vec<EditorTab>,
    active_tab_id: Option<uuid::Uuid>,
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
}

impl NotosApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initial setup
        setup_custom_fonts(&cc.egui_ctx);

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
        };

        if let Some(session) = Self::load_session() {
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

            // Mark all tabs to restore cursor
            for tab in &mut app.tabs {
                tab.scroll_to_cursor = true;
            }
        } else {
            // Default look
            setup_custom_style(&cc.egui_ctx, app.dark_mode);

            if let Some(first) = app.tabs.first() {
                app.active_tab_id = Some(first.id);
            }
        }

        // Load plugins here
        app.plugin_manager.load_plugins();
        app.plugin_manager.on_load(&cc.egui_ctx);

        app
    }

    fn save_session(&self) -> anyhow::Result<()> {
        let state = SessionState {
            tabs: self.tabs.clone(),
            active_tab_id: self.active_tab_id,
            word_wrap: self.word_wrap,
            show_line_numbers: self.show_line_numbers,
            dark_mode: self.dark_mode,
            editor_font_size: self.editor_font_size,
            editor_font_family: self.editor_font_family.clone(),
            custom_fonts: self.custom_fonts.clone(),
            recent_files: self.recent_files.clone(),
        };

        let path = std::env::temp_dir().join("notos_session.json");
        let file = fs::File::create(path)?;
        serde_json::to_writer(file, &state)?;
        Ok(())
    }

    fn load_session() -> Option<SessionState> {
        let path = std::env::temp_dir().join("notos_session.json");
        if let Ok(file) = fs::File::open(path) {
            match serde_json::from_reader(file) {
                Ok(state) => return Some(state),
                Err(e) => log::error!("Failed to load session: {}", e),
            }
        }
        None
    }

    fn active_tab_mut(&mut self) -> Option<&mut EditorTab> {
        self.tabs
            .iter_mut()
            .find(|t| Some(t.id) == self.active_tab_id)
    }

    fn add_to_recent(&mut self, path: std::path::PathBuf) {
        // Remove if already exists to move to top
        if let Some(pos) = self.recent_files.iter().position(|p| p == &path) {
            self.recent_files.remove(pos);
        }
        self.recent_files.insert(0, path);
        // Limit to 8
        self.recent_files.truncate(8);
    }

    fn get_editor_context(&self) -> notos_sdk::EditorContext<'_> {
        get_ed_ctx(&self.tabs, self.active_tab_id)
    }

    fn handle_plugin_action(&mut self, action: notos_sdk::PluginAction, ctx: &egui::Context) {
        use notos_sdk::PluginAction;
        match action {
            PluginAction::None => {}
            PluginAction::ReplaceAll(new_text) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.push_undo(tab.content.clone());
                    tab.content = new_text;
                    tab.is_dirty = true;
                }
            }
            PluginAction::ReplaceSelection(new_text) => {
                if let Some(tab) = self.active_tab_mut() {
                    let id = egui::Id::new("editor").with(tab.id);
                    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                    let range = state.cursor.char_range().unwrap_or_else(|| {
                        let (p, s) = tab.cursor_range.unwrap_or((0, 0));
                        egui::text::CCursorRange::two(
                            egui::text::CCursor::new(p),
                            egui::text::CCursor::new(s),
                        )
                    });

                    let (start, end) = (
                        range.primary.index.min(range.secondary.index),
                        range.primary.index.max(range.secondary.index),
                    );

                    tab.push_undo(tab.content.clone());
                    if start != end {
                        tab.content.replace_range(start..end, &new_text);
                    } else {
                        tab.content.insert_str(start, &new_text);
                    }
                    tab.is_dirty = true;

                    // Update cursor to end of new text
                    let new_idx = start + new_text.len();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(new_idx),
                        )));
                    egui::TextEdit::store_state(ctx, id, state);
                    tab.cursor_range = Some((new_idx, new_idx));
                }
            }
        }
    }

    fn open_file(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            let path_clone = path.clone();
            match EditorTab::from_file(path) {
                Ok(tab) => {
                    self.active_tab_id = Some(tab.id);
                    self.tabs.push(tab);
                    self.add_to_recent(path_clone);
                }
                Err(e) => {
                    log::error!("Failed to open file: {}", e);
                }
            }
        }
    }

    fn save_file(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            if tab.path.is_some() {
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                }
            } else {
                self.save_file_as();
            }
        }
    }

    fn save_file_as(&mut self) {
        if let Some(id) = self.active_tab_id {
            self.save_tab_as_by_id(id);
        }
    }

    fn save_tab_as_by_id(&mut self, id: uuid::Uuid) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if let Some(path) = FileDialog::new()
                .add_filter("Text", &["txt", "md"])
                .add_filter("Rust", &["rs", "toml"])
                .add_filter("Python", &["py"])
                .add_filter("JavaScript", &["js", "ts"])
                .add_filter("HTML", &["html"])
                .add_filter("CSS", &["css"])
                .add_filter("All Files", &["*"])
                .set_file_name("untitled.txt")
                .save_file()
            {
                tab.set_path(path.clone());
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                } else {
                    self.add_to_recent(path);
                }
            }
        }
    }

    fn close_tab(&mut self, id: uuid::Uuid) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            if self.tabs[index].is_dirty {
                self.close_confirmation.open = true;
                self.close_confirmation.tab_id = Some(id);
                self.close_confirmation.closing_app = false;
            } else {
                self.tabs.remove(index);
                if self.active_tab_id == Some(id) {
                    self.active_tab_id = self.tabs.last().map(|t| t.id);
                }
            }
        }
    }

    fn handle_menu_action(&mut self, action: crate::ui::MenuAction, ctx: &egui::Context) {
        use crate::ui::MenuAction;
        match action {
            MenuAction::NewTab => {
                let tab = EditorTab::default();
                self.active_tab_id = Some(tab.id);
                self.tabs.push(tab);
            }
            MenuAction::Open => self.open_file(),
            MenuAction::OpenRecent(path) => {
                let path_clone = path.clone();
                match EditorTab::from_file(path) {
                    Ok(tab) => {
                        self.active_tab_id = Some(tab.id);
                        self.tabs.push(tab);
                        self.add_to_recent(path_clone);
                    }
                    Err(e) => {
                        log::error!("Failed to open recent file: {}", e);
                    }
                }
            }
            MenuAction::Save => self.save_file(),
            MenuAction::SaveAs => self.save_file_as(),
            MenuAction::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            MenuAction::Undo => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.undo();
                }
            }
            MenuAction::Redo => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.redo();
                }
            }
            MenuAction::Find => {
                self.find_dialog.open = true;
                self.find_dialog.replace_mode = false;
            }
            MenuAction::Replace => {
                self.find_dialog.open = true;
                self.find_dialog.replace_mode = true;
            }
            MenuAction::GotoLine => {
                self.goto_dialog.open = true;
                self.goto_dialog.line_str = self.current_cursor_pos.0.to_string();
            }
            MenuAction::TimeDate => {
                if let Some(tab) = self.active_tab_mut() {
                    let now = chrono::Local::now();
                    let time_str = now.format("%I:%M %p %m/%d/%Y").to_string();

                    let id = egui::Id::new("editor").with(tab.id);
                    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                        if let Some(range) = state.cursor.char_range() {
                            let idx = range.primary.index;
                            tab.push_undo(tab.content.clone());
                            tab.content.insert_str(idx, &time_str);
                            tab.is_dirty = true;

                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::one(
                                    egui::text::CCursor::new(idx + time_str.len()),
                                )));
                            egui::TextEdit::store_state(ctx, id, state);
                        }
                    }
                }
            }
            MenuAction::ToggleWordWrap => { /* handled by ref mut */ }
            MenuAction::ToggleLineNumbers => { /* handled by ref mut */ }
            MenuAction::ToggleDarkMode => {
                setup_custom_style(ctx, self.dark_mode);
            }
            MenuAction::ZoomIn => {
                self.editor_font_size = (self.editor_font_size + 1.0).min(72.0);
            }
            MenuAction::ZoomOut => {
                self.editor_font_size = (self.editor_font_size - 1.0).max(6.0);
            }
            MenuAction::ResetZoom => {
                self.editor_font_size = 14.0;
            }
            MenuAction::ChangeFont(name) => {
                self.editor_font_family = name;
            }
            MenuAction::LoadFont => {
                if let Some(path) = FileDialog::new()
                    .add_filter("Font", &["ttf", "otf"])
                    .pick_file()
                {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        self.custom_fonts.insert(name.clone(), bytes.clone());

                        // Update egui fonts
                        let mut fonts = egui::FontDefinitions::default();
                        // Re-add all custom fonts
                        for (n, b) in &self.custom_fonts {
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
                        ctx.set_fonts(fonts);

                        self.editor_font_family = name;
                    }
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::N)) {
            let tab = EditorTab::default();
            self.active_tab_id = Some(tab.id);
            self.tabs.push(tab);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.open_file();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
            self.save_file();
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S)
        }) {
            self.save_file_as();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::W)) {
            if let Some(id) = self.active_tab_id {
                self.close_tab(id);
            }
        }

        // Zoom Shortcuts
        if ctx.input(|i| {
            i.modifiers.ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals))
        }) {
            self.editor_font_size = (self.editor_font_size + 1.0).clamp(6.0, 72.0);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Minus)) {
            self.editor_font_size = (self.editor_font_size - 1.0).clamp(6.0, 72.0);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) {
            self.editor_font_size = 14.0;
        }

        // Mouse Wheel Zoom
        if ctx.input(|i| i.modifiers.ctrl) {
            let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                if scroll_delta > 0.0 {
                    self.editor_font_size = (self.editor_font_size + 1.0).min(72.0);
                } else {
                    self.editor_font_size = (self.editor_font_size - 1.0).max(6.0);
                }
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
            self.handle_menu_action(crate::ui::MenuAction::TimeDate, ctx);
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
            self.handle_menu_action(crate::ui::MenuAction::GotoLine, ctx);
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.handle_menu_action(crate::ui::MenuAction::Find, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.handle_menu_action(crate::ui::MenuAction::Replace, ctx);
        }
    }
}
impl eframe::App for NotosApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ALWAYS ensure visuals are synced with our state
        setup_custom_style(ctx, self.dark_mode);

        // Handle Window Close
        if ctx.input(|i| i.viewport().close_requested()) {
            match self.save_session() {
                Ok(_) => {
                    // Session saved, allow close without confirmation
                    // Do not cancel close
                }
                Err(e) => {
                    log::error!("Failed to save session: {}", e);
                    // Fallback to confirmation if save failed and dirty
                    if self.tabs.iter().any(|t| t.is_dirty) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.close_confirmation.open = true;
                        self.close_confirmation.closing_app = true;
                        self.close_confirmation.tab_id = None;
                    }
                }
            }
        }

        // Handle Drag and Drop
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped_files {
            if let Some(path) = file.path {
                match EditorTab::from_file(path) {
                    Ok(tab) => {
                        self.active_tab_id = Some(tab.id);
                        self.tabs.push(tab);
                    }
                    Err(e) => {
                        log::error!("Failed to open dropped file: {}", e);
                    }
                }
            }
        }

        self.handle_shortcuts(ctx);

        // Dialogs
        let active_tab = self
            .tabs
            .iter_mut()
            .find(|t| Some(t.id) == self.active_tab_id);
        self.find_dialog.show(ctx, active_tab);

        let active_tab = self
            .tabs
            .iter_mut()
            .find(|t| Some(t.id) == self.active_tab_id);
        self.goto_dialog.show(ctx, active_tab);

        // Close Confirmation
        let save_fn = |tab: &mut EditorTab| -> anyhow::Result<()> {
            if tab.path.is_some() {
                tab.save()
            } else if let Some(path) = FileDialog::new()
                .add_filter("Text", &["txt", "md"])
                .add_filter("Rust", &["rs", "toml"])
                .add_filter("Python", &["py"])
                .add_filter("JavaScript", &["js", "ts"])
                .add_filter("HTML", &["html"])
                .add_filter("CSS", &["css"])
                .add_filter("All Files", &["*"])
                .set_file_name("untitled.txt")
                .save_file()
            {
                tab.set_path(path);
                tab.save()
            } else {
                Err(anyhow::anyhow!("Cancelled"))
            }
        };

        self.close_confirmation
            .show(ctx, &mut self.tabs, &mut self.active_tab_id, save_fn);

        // Determine background colors
        let panel_bg = if self.dark_mode {
            egui::Color32::from_gray(38)
        } else {
            egui::Color32::WHITE
        };

        // Create EditorContext for plugins
        let ed_ctx = get_ed_ctx(&self.tabs, self.active_tab_id);

        let mut menu_action_to_run = None;
        let mut plugin_action_to_run_top = notos_sdk::PluginAction::None;
        let mut tab_action_to_run = None;

        let plugin_manager = &mut self.plugin_manager;
        let word_wrap = &mut self.word_wrap;
        let show_line_numbers = &mut self.show_line_numbers;
        let dark_mode = &mut self.dark_mode;
        let tabs = &self.tabs;
        let active_tab_id = self.active_tab_id;

        // Top Panel: Menu and Tabs
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::default().fill(panel_bg).inner_margin(8.0))
            .show(ctx, |ui| {
                let (m, p) = crate::ui::menu_bar(
                    ui,
                    plugin_manager,
                    word_wrap,
                    show_line_numbers,
                    dark_mode,
                    &self.editor_font_family,
                    &self.custom_fonts,
                    &self.recent_files,
                    &ed_ctx,
                );
                menu_action_to_run = m;
                plugin_action_to_run_top = p;

                ui.add_space(4.0);
                tab_action_to_run = crate::ui::tab_bar(ui, tabs, active_tab_id);
            });

        if let Some(action) = menu_action_to_run {
            self.handle_menu_action(action, ctx);
        }
        self.handle_plugin_action(plugin_action_to_run_top, ctx);

        if let Some(action) = tab_action_to_run {
            match action {
                crate::ui::TabAction::New => {
                    let tab = EditorTab::default();
                    self.active_tab_id = Some(tab.id);
                    self.tabs.push(tab);
                }
                crate::ui::TabAction::Select(id) => {
                    self.active_tab_id = Some(id);
                }
                crate::ui::TabAction::Close(id) => {
                    self.close_tab(id);
                }
                crate::ui::TabAction::CloseOthers(id) => {
                    let ids_to_close: Vec<_> = self
                        .tabs
                        .iter()
                        .filter(|t| t.id != id)
                        .map(|t| t.id)
                        .collect();
                    for close_id in ids_to_close {
                        self.close_tab(close_id);
                    }
                }
            }
        }

        // Bottom Panel: Status Bar
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(
                egui::Frame::default()
                    .fill(panel_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                if let Some(action) = crate::ui::status_bar(
                    ui,
                    &self.tabs,
                    self.active_tab_id,
                    self.current_cursor_pos,
                    self.editor_font_size,
                ) {
                    match action {
                        crate::ui::StatusBarAction::SwitchTab(id) => self.active_tab_id = Some(id),
                        crate::ui::StatusBarAction::CloseTab(id) => self.close_tab(id),
                        crate::ui::StatusBarAction::SetEncoding(id, enc) => {
                            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                                tab.encoding = enc;
                                tab.is_dirty = true;
                            }
                        }
                        crate::ui::StatusBarAction::SetLineEnding(id, le) => {
                            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                                tab.line_ending = le;
                                tab.is_dirty = true;
                            }
                        }
                    }
                }
            });

        // Central Panel: Editor
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(if self.dark_mode {
                egui::Color32::from_gray(28)
            } else {
                egui::Color32::WHITE
            }))
            .show(ctx, |ui| {
                let plugin_manager = &mut self.plugin_manager;
                let tabs = &mut self.tabs;
                let active_tab_id = self.active_tab_id;

                let idx = tabs
                    .iter()
                    .position(|t| Some(t.id) == active_tab_id)
                    .unwrap_or(0);

                if let Some(tab) = tabs.get_mut(idx) {
                    let mut content_changed = false;
                    let previous_content = tab.content.clone();

                    let mut new_cursor_pos = None;
                    let mut tab_changed_idx = None;
                    let mut plugin_action_to_run_context = notos_sdk::PluginAction::None;

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let margin = 4.0;
                        let family = if self.editor_font_family == "Monospace" {
                            egui::FontFamily::Monospace
                        } else if self.editor_font_family == "Proportional" {
                            egui::FontFamily::Proportional
                        } else {
                            egui::FontFamily::Name(self.editor_font_family.clone().into())
                        };
                        let font_id = egui::FontId::new(self.editor_font_size, family);

                        let line_number_width = if self.show_line_numbers {
                            let line_count = tab.content.lines().count().max(1);
                            let line_count = if tab.content.ends_with('\n') {
                                line_count + 1
                            } else {
                                line_count
                            };
                            let num_digits = line_count.to_string().len().max(2);
                            (num_digits as f32 * self.editor_font_size * 0.6) + 12.0
                        } else {
                            0.0
                        };

                        let available_height = ui.available_height();

                        let editor_bg = if self.dark_mode {
                            egui::Color32::from_gray(28)
                        } else {
                            egui::Color32::WHITE
                        };

                        let mut response = None;
                        let mut galley_to_draw = None;

                        ui.horizontal(|ui| {
                            if self.show_line_numbers {
                                ui.add_space(line_number_width + 8.0);
                            }

                            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                                let layout_job = egui::text::LayoutJob::simple(
                                    string.to_string(),
                                    font_id.clone(),
                                    ui.visuals().widgets.noninteractive.text_color(),
                                    if self.word_wrap {
                                        wrap_width
                                    } else {
                                        f32::INFINITY
                                    },
                                );
                                let galley = ui.fonts(|f| f.layout_job(layout_job));
                                galley_to_draw = Some(galley.clone());
                                galley
                            };

                            egui::Frame::none().fill(editor_bg).show(ui, |ui| {
                                let res = ui.add_sized(
                                    [ui.available_width(), available_height],
                                    egui::TextEdit::multiline(&mut tab.content)
                                        .id(egui::Id::new("editor").with(tab.id))
                                        .font(font_id.clone())
                                        .frame(false)
                                        .code_editor()
                                        .lock_focus(true)
                                        .margin(egui::Margin::same(margin))
                                        .layouter(&mut layouter)
                                        .desired_width(if self.word_wrap {
                                            ui.available_width()
                                        } else {
                                            f32::INFINITY
                                        }),
                                );

                                // Add context menu
                                let ed_ctx = notos_sdk::EditorContext {
                                    content: &tab.content,
                                    selection: tab.cursor_range,
                                };
                                res.context_menu(|ui| {
                                    plugin_action_to_run_context =
                                        plugin_manager.context_menu_ui(ui, &ed_ctx);
                                });

                                if tab.scroll_to_cursor {
                                    res.request_focus();
                                    // Restore cursor position
                                    let id = egui::Id::new("editor").with(tab.id);
                                    let mut state = egui::TextEdit::load_state(ui.ctx(), id)
                                        .unwrap_or_default();
                                    if let Some((p, s)) = tab.cursor_range {
                                        state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(p),
                                                egui::text::CCursor::new(s),
                                            ),
                                        ));
                                    }
                                    egui::TextEdit::store_state(ui.ctx(), id, state);

                                    tab.scroll_to_cursor = false;
                                }

                                if res.changed() {
                                    content_changed = true;
                                    tab.is_dirty = true;
                                    tab_changed_idx = Some(idx);
                                }

                                if let Some(mut state) =
                                    egui::TextEdit::load_state(ui.ctx(), res.id)
                                {
                                    if let Some(range) = state.cursor.char_range() {
                                        // Update persisted range only when NOT right-clicking,
                                        // to "lock" the selection when the context menu is about to open.
                                        if !ui.input(|i| {
                                            i.pointer.secondary_down()
                                                || i.pointer.secondary_clicked()
                                        }) {
                                            if res.has_focus() {
                                                tab.cursor_range = Some((
                                                    range.primary.index,
                                                    range.secondary.index,
                                                ));
                                            }
                                        } else {
                                            // If right-clicking, and we have a saved selection, force it back
                                            // into the TextEdit state to prevent it from disappearing.
                                            if let Some((p, s)) = tab.cursor_range {
                                                if p != s {
                                                    state.cursor.set_char_range(Some(
                                                        egui::text::CCursorRange::two(
                                                            egui::text::CCursor::new(p),
                                                            egui::text::CCursor::new(s),
                                                        ),
                                                    ));
                                                    egui::TextEdit::store_state(
                                                        ui.ctx(),
                                                        res.id,
                                                        state.clone(),
                                                    );
                                                }
                                            }
                                        }

                                        let idx = range.primary.index;
                                        let text = &tab.content;
                                        let mut line = 1;
                                        let mut col = 1;
                                        for (i, c) in text.char_indices() {
                                            if i >= idx {
                                                break;
                                            }
                                            if c == '\n' {
                                                line += 1;
                                                col = 1;
                                            } else {
                                                col += 1;
                                            }
                                        }
                                        new_cursor_pos = Some((line, col));
                                    }
                                }

                                // Context Menu
                                res.context_menu(|ui| {
                                    // Ensure the TextEdit keeps its selection state while the menu is open
                                    let id = egui::Id::new("editor").with(tab.id);
                                    if let (Some(mut state), Some((p, s))) =
                                        (egui::TextEdit::load_state(ui.ctx(), id), tab.cursor_range)
                                    {
                                        if p != s {
                                            state.cursor.set_char_range(Some(
                                                egui::text::CCursorRange::two(
                                                    egui::text::CCursor::new(p),
                                                    egui::text::CCursor::new(s),
                                                ),
                                            ));
                                            egui::TextEdit::store_state(ui.ctx(), id, state);
                                        }
                                    }

                                    if ui.button("Undo").clicked() {
                                        tab.undo();
                                        ui.close_menu();
                                    }
                                    if ui.button("Redo").clicked() {
                                        tab.redo();
                                        ui.close_menu();
                                    }
                                    ui.separator();

                                    if ui.button("Cut").clicked() {
                                        let id = egui::Id::new("editor").with(tab.id);
                                        if let Some(state) =
                                            egui::TextEdit::load_state(ui.ctx(), id)
                                        {
                                            // Use either the current state range or the persisted tab range
                                            let range = state
                                                .cursor
                                                .char_range()
                                                .filter(|r| r.primary != r.secondary)
                                                .unwrap_or_else(|| {
                                                    let (p, s) = tab.cursor_range.unwrap_or((0, 0));
                                                    egui::text::CCursorRange::two(
                                                        egui::text::CCursor::new(p),
                                                        egui::text::CCursor::new(s),
                                                    )
                                                });

                                            if range.primary != range.secondary {
                                                let (start, end) = (
                                                    range.primary.index.min(range.secondary.index),
                                                    range.primary.index.max(range.secondary.index),
                                                );
                                                if end <= tab.content.len() {
                                                    let selected_text = &tab.content[start..end];
                                                    if let Ok(mut clipboard) =
                                                        arboard::Clipboard::new()
                                                    {
                                                        let _ = clipboard
                                                            .set_text(selected_text.to_string());
                                                    }
                                                    tab.push_undo(tab.content.clone());
                                                    tab.content.replace_range(start..end, "");
                                                    tab.is_dirty = true;

                                                    let mut new_state = state.clone();
                                                    new_state.cursor.set_char_range(Some(
                                                        egui::text::CCursorRange::one(
                                                            egui::text::CCursor::new(start),
                                                        ),
                                                    ));
                                                    egui::TextEdit::store_state(
                                                        ui.ctx(),
                                                        id,
                                                        new_state,
                                                    );
                                                    tab.cursor_range = Some((start, start));
                                                }
                                            }
                                        }
                                        ui.close_menu();
                                    }

                                    if ui.button("Copy").clicked() {
                                        let id = egui::Id::new("editor").with(tab.id);
                                        if let Some(state) =
                                            egui::TextEdit::load_state(ui.ctx(), id)
                                        {
                                            let range = state
                                                .cursor
                                                .char_range()
                                                .filter(|r| r.primary != r.secondary)
                                                .unwrap_or_else(|| {
                                                    let (p, s) = tab.cursor_range.unwrap_or((0, 0));
                                                    egui::text::CCursorRange::two(
                                                        egui::text::CCursor::new(p),
                                                        egui::text::CCursor::new(s),
                                                    )
                                                });

                                            if range.primary != range.secondary {
                                                let (start, end) = (
                                                    range.primary.index.min(range.secondary.index),
                                                    range.primary.index.max(range.secondary.index),
                                                );
                                                if end <= tab.content.len() {
                                                    let selected_text = &tab.content[start..end];
                                                    if let Ok(mut clipboard) =
                                                        arboard::Clipboard::new()
                                                    {
                                                        let _ = clipboard
                                                            .set_text(selected_text.to_string());
                                                    }
                                                }
                                            }
                                        }
                                        ui.close_menu();
                                    }

                                    if ui.button("Paste").clicked() {
                                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                            if let Ok(text) = clipboard.get_text() {
                                                let id = egui::Id::new("editor").with(tab.id);
                                                let mut state =
                                                    egui::TextEdit::load_state(ui.ctx(), id)
                                                        .unwrap_or_default();

                                                let range = state
                                                    .cursor
                                                    .char_range()
                                                    .unwrap_or_else(|| {
                                                        let (p, s) = tab.cursor_range.unwrap_or((
                                                            tab.content.len(),
                                                            tab.content.len(),
                                                        ));
                                                        egui::text::CCursorRange::two(
                                                            egui::text::CCursor::new(p),
                                                            egui::text::CCursor::new(s),
                                                        )
                                                    });

                                                let (start, end) = (
                                                    range.primary.index.min(range.secondary.index),
                                                    range.primary.index.max(range.secondary.index),
                                                );

                                                tab.push_undo(tab.content.clone());
                                                if start != end {
                                                    if end <= tab.content.len() {
                                                        tab.content
                                                            .replace_range(start..end, &text);
                                                    }
                                                } else if start <= tab.content.len() {
                                                    tab.content.insert_str(start, &text);
                                                }
                                                tab.is_dirty = true;

                                                let new_cursor_idx = start + text.len();
                                                state.cursor.set_char_range(Some(
                                                    egui::text::CCursorRange::one(
                                                        egui::text::CCursor::new(new_cursor_idx),
                                                    ),
                                                ));
                                                egui::TextEdit::store_state(ui.ctx(), id, state);
                                                tab.cursor_range =
                                                    Some((new_cursor_idx, new_cursor_idx));
                                            }
                                        }
                                        ui.close_menu();
                                    }

                                    ui.separator();

                                    if ui.button("Select All").clicked() {
                                        let id = egui::Id::new("editor").with(tab.id);
                                        let mut state = egui::TextEdit::load_state(ui.ctx(), id)
                                            .unwrap_or_default();
                                        let len = tab.content.len();
                                        state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(0),
                                                egui::text::CCursor::new(len),
                                            ),
                                        ));
                                        egui::TextEdit::store_state(ui.ctx(), id, state);
                                        tab.cursor_range = Some((0, len));
                                        ui.close_menu();
                                    }
                                });

                                response = Some(res);
                            });

                            // Line numbers rendering
                            if self.show_line_numbers {
                                if let (Some(res), Some(galley)) = (response, galley_to_draw) {
                                    let painter = ui.painter();
                                    let mut logical_line = 1;
                                    let mut is_start_of_logical_line = true;

                                    let line_num_rect = egui::Rect::from_min_max(
                                        egui::pos2(
                                            res.rect.min.x - line_number_width - 8.0,
                                            res.rect.min.y,
                                        ),
                                        egui::pos2(res.rect.min.x, res.rect.max.y),
                                    );
                                    painter.rect_filled(
                                        line_num_rect,
                                        0.0,
                                        ui.visuals().widgets.noninteractive.bg_fill,
                                    );

                                    painter.line_segment(
                                        [
                                            egui::pos2(res.rect.min.x - 2.0, res.rect.min.y),
                                            egui::pos2(res.rect.min.x - 2.0, res.rect.max.y),
                                        ],
                                        ui.visuals().widgets.noninteractive.bg_stroke,
                                    );

                                    for row in &galley.rows {
                                        if is_start_of_logical_line {
                                            let pos = egui::pos2(
                                                res.rect.min.x - 8.0,
                                                res.rect.min.y + margin + row.rect.min.y,
                                            );
                                            painter.text(
                                                pos,
                                                egui::Align2::RIGHT_TOP,
                                                logical_line.to_string(),
                                                font_id.clone(),
                                                ui.visuals().weak_text_color(),
                                            );
                                            logical_line += 1;
                                        }
                                        is_start_of_logical_line = row.ends_with_newline;
                                    }
                                }
                            }
                        });
                    });

                    self.handle_plugin_action(plugin_action_to_run_context, ctx);

                    if let Some(pos) = new_cursor_pos {
                        self.current_cursor_pos = pos;
                    }

                    if content_changed {
                        if let Some(idx) = tab_changed_idx {
                            if let Some(tab) = self.tabs.get_mut(idx) {
                                tab.push_undo(previous_content);
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No open tabs. Press Ctrl+N to create a new one.");
                    });
                }
            });

        let ed_ctx = get_ed_ctx(&self.tabs, self.active_tab_id);
        let plugin_action = self.plugin_manager.ui(ctx, &ed_ctx);
        self.handle_plugin_action(plugin_action, ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.plugin_manager.on_unload();
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // We could load "Consolas" or "Cascadia Code" here if we bundled them,
    // but for now we rely on default monospace.
    // In a real app, we'd load system fonts.

    // Example of configuring font families
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "Hack".to_owned()); // If Hack was loaded

    ctx.set_fonts(fonts);
}

fn setup_custom_style(ctx: &egui::Context, dark_mode: bool) {
    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    if dark_mode {
        // Deep dark background for the editor
        let editor_bg = egui::Color32::from_gray(28);
        // Slightly lighter for the panels
        let panel_bg = egui::Color32::from_gray(38);

        visuals.panel_fill = panel_bg;
        visuals.window_fill = panel_bg;
        visuals.extreme_bg_color = editor_bg;

        // Ensure buttons and non-interactive areas have correct background
        visuals.widgets.noninteractive.bg_fill = panel_bg;
        visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(45);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(55);
        visuals.widgets.active.bg_fill = egui::Color32::from_gray(65);

        // Contrasty text colors
        visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_gray(220);
        visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_gray(230);
        visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
        visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;

        visuals.window_shadow.color = egui::Color32::from_black_alpha(100);
    } else {
        visuals.widgets.noninteractive.bg_fill = egui::Color32::WHITE;
        visuals.window_fill = egui::Color32::WHITE;
        visuals.panel_fill = egui::Color32::WHITE;
        visuals.extreme_bg_color = egui::Color32::WHITE;

        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    }

    ctx.set_visuals(visuals.clone());

    // Update the whole style to ensure everything propagates
    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    // Add some padding to widgets for a more modern look
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    ctx.set_style(style);
}
fn get_ed_ctx(
    tabs: &[EditorTab],
    active_tab_id: Option<uuid::Uuid>,
) -> notos_sdk::EditorContext<'_> {
    if let Some(tab) = tabs.iter().find(|t| Some(t.id) == active_tab_id) {
        notos_sdk::EditorContext {
            content: &tab.content,
            selection: tab.cursor_range,
        }
    } else {
        notos_sdk::EditorContext {
            content: "",
            selection: None,
        }
    }
}
