use eframe::egui;
use crate::editor::EditorTab;
use crate::plugin::PluginManager;
use crate::ui;
use rfd::FileDialog;
// use std::path::PathBuf;

pub struct NotosApp {
    tabs: Vec<EditorTab>,
    active_tab_id: Option<uuid::Uuid>,
    plugin_manager: PluginManager,
    current_cursor_pos: (usize, usize), // Line, Col (1-based)
    find_state: FindDialogState,
    goto_line_state: GotoLineState,
    word_wrap: bool,
    dark_mode: bool,
    // Settings, etc.
}

struct GotoLineState {
    open: bool,
    line_str: String,
}

impl Default for GotoLineState {
    fn default() -> Self {
        Self {
            open: false,
            line_str: String::new(),
        }
    }
}

struct FindDialogState {
    open: bool,
    query: String,
    replace_with: String,
    match_case: bool,
    replace_mode: bool,
}

impl Default for FindDialogState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            replace_with: String::new(),
            match_case: false,
            replace_mode: false,
        }
    }
}

impl NotosApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize fonts/style here to match Notepad
        setup_custom_fonts(&cc.egui_ctx);
        setup_custom_fonts(&cc.egui_ctx);
        setup_custom_style(&cc.egui_ctx, false);

        let mut app = Self {
            tabs: vec![EditorTab::default()],
            active_tab_id: None, // Will be set in init
            plugin_manager: PluginManager::new(),
            current_cursor_pos: (1, 1),
            find_state: FindDialogState::default(),
            goto_line_state: GotoLineState::default(),
            word_wrap: true,
            dark_mode: false,
        };
        
        if let Some(first) = app.tabs.first() {
            app.active_tab_id = Some(first.id);
        }

        // Load plugins here
        app.plugin_manager.register(Box::new(crate::plugins::stats::StatsPlugin::default()));
        
        app.plugin_manager.on_load(&cc.egui_ctx);

        app
    }

    fn active_tab_mut(&mut self) -> Option<&mut EditorTab> {
        self.tabs.iter_mut().find(|t| Some(t.id) == self.active_tab_id)
    }

    fn open_file(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            match EditorTab::from_file(path) {
                Ok(tab) => {
                    self.active_tab_id = Some(tab.id);
                    self.tabs.push(tab);
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
        if let Some(tab) = self.active_tab_mut() {
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
                tab.set_path(path);
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                }
            }
        }
    }
}

impl eframe::App for NotosApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Keyboard Shortcuts
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
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S)) {
            self.save_file_as();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::W)) {
            if let Some(id) = self.active_tab_id {
                if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
                    self.tabs.remove(index);
                    self.active_tab_id = self.tabs.last().map(|t| t.id);
                }
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
            if let Some(tab) = self.active_tab_mut() {
                let now = chrono::Local::now();
                let time_str = now.format("%I:%M %p %m/%d/%Y").to_string(); // Notepad format: 12:00 PM 1/1/2023
                
                // We need to insert at cursor. 
                // Since we don't track cursor index perfectly in `self.current_cursor_pos` (it's line/col),
                // and `TextEdit` doesn't easily expose "insert at cursor" programmatically without the state,
                // we have to rely on the `TextEdit` state again.
                
                let id = egui::Id::new("editor");
                if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(range) = state.cursor.char_range() {
                        let idx = range.primary.index;
                        tab.push_undo(tab.content.clone());
                        tab.content.insert_str(idx, &time_str);
                        tab.is_dirty = true;
                        
                        // Move cursor
                        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(idx + time_str.len()),
                        )));
                        egui::TextEdit::store_state(ctx, id, state);
                    }
                }
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
            self.goto_line_state.open = true;
            self.goto_line_state.line_str = self.current_cursor_pos.0.to_string();
        }

        // Go To Line Dialog
        let mut goto_open = self.goto_line_state.open;
        let mut goto_clicked = false;
        if goto_open {
             egui::Window::new("Go To Line")
                .open(&mut goto_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Line number:");
                        ui.text_edit_singleline(&mut self.goto_line_state.line_str);
                    });
                    
                    if ui.button("Go To").clicked() {
                        goto_clicked = true;
                    }
                });
        }
        self.goto_line_state.open = goto_open;

        if goto_clicked {
            if let Ok(target_line) = self.goto_line_state.line_str.parse::<usize>() {
                if let Some(tab) = self.active_tab_mut() {
                    // Find the byte index of the start of the line
                    let text = &tab.content;
                    let mut current_line = 1;
                    let mut char_idx = 0;
                    
                    for (i, c) in text.char_indices() {
                        if current_line == target_line {
                            char_idx = i;
                            break;
                        }
                        if c == '\n' {
                            current_line += 1;
                        }
                    }
                    
                    // If target line is beyond end, go to end
                    if current_line < target_line {
                        char_idx = text.len();
                    }

                    let id = egui::Id::new("editor");
                    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                         state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(char_idx),
                        )));
                        egui::TextEdit::store_state(ctx, id, state);
                        self.goto_line_state.open = false;
                    }
                }
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.find_state.open = true;
            self.find_state.replace_mode = false;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.find_state.open = true;
            self.find_state.replace_mode = true;
        }

        // Find Dialog
        let mut open = self.find_state.open;
        let mut find_next_clicked = false;
        if open {
            let title = if self.find_state.replace_mode { "Replace" } else { "Find" };
            egui::Window::new(title)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find what:");
                        ui.text_edit_singleline(&mut self.find_state.query);
                    });
                    
                    if self.find_state.replace_mode {
                        ui.horizontal(|ui| {
                            ui.label("Replace with:");
                            ui.text_edit_singleline(&mut self.find_state.replace_with);
                        });
                    }

                    ui.checkbox(&mut self.find_state.match_case, "Match case");
                    
                    ui.horizontal(|ui| {
                        if ui.button("Find Next").clicked() {
                            find_next_clicked = true;
                        }
                        
                        if self.find_state.replace_mode {
                            if ui.button("Replace").clicked() {
                                let query = self.find_state.query.clone();
                                let replace = self.find_state.replace_with.clone();
                                
                                if !query.is_empty() {
                                    if let Some(tab) = self.active_tab_mut() {
                                        let id = egui::Id::new("editor");
                                        // Check if current selection matches query
                                        if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                                            if let Some(range) = state.cursor.char_range() {
                                                let start = range.primary.index.min(range.secondary.index);
                                                let end = range.primary.index.max(range.secondary.index);
                                                
                                                // Ensure indices are valid char boundaries (TextEdit usually ensures this)
                                                if start < tab.content.len() && end <= tab.content.len() {
                                                    let selected_text = &tab.content[start..end];
                                                    if selected_text == query {
                                                        // Replace
                                                        tab.push_undo(tab.content.clone());
                                                        tab.content.replace_range(start..end, &replace);
                                                        tab.is_dirty = true;
                                                        
                                                        // Update cursor to end of replacement
                                                        let new_idx = start + replace.len();
                                                        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                                                            egui::text::CCursor::new(new_idx),
                                                        )));
                                                        egui::TextEdit::store_state(ctx, id, state);
                                                    }
                                                }
                                            }
                                        }
                                        // Find next occurrence
                                        find_next_clicked = true;
                                    }
                                }
                            }
                            if ui.button("Replace All").clicked() {
                                let query = self.find_state.query.clone();
                                let replace = self.find_state.replace_with.clone();
                                
                                if !query.is_empty() {
                                    if let Some(tab) = self.active_tab_mut() {
                                        let new_content = tab.content.replace(&query, &replace);
                                        if new_content != tab.content {
                                            tab.push_undo(tab.content.clone());
                                            tab.content = new_content;
                                            tab.is_dirty = true;
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
        }
        self.find_state.open = open;

        if find_next_clicked {
            let query = self.find_state.query.clone();
            // let match_case = self.find_state.match_case; // Unused for now
            
            if !query.is_empty() {
                if let Some(tab) = self.active_tab_mut() {
                    let text = &tab.content;
                    let id = egui::Id::new("editor");
                    let mut start_idx = 0;
                    
                    if let Some(state) = egui::TextEdit::load_state(ctx, id) {
                        if let Some(range) = state.cursor.char_range() {
                            // Start searching after the current selection/cursor
                            start_idx = range.primary.index.max(range.secondary.index);
                        }
                    }
                    
                    // Search forward from start_idx
                    // We need to handle potential char boundary issues if start_idx is somehow invalid, 
                    // but TextEdit should give valid boundaries.
                    // Also handle if start_idx is at end.
                    let search_slice = if start_idx < text.len() { &text[start_idx..] } else { "" };
                    
                    let found_idx = search_slice.find(&query).map(|i| start_idx + i)
                        .or_else(|| {
                            // Wrap around
                            text.find(&query)
                        });

                    if let Some(idx) = found_idx {
                        if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                             state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(idx),
                                egui::text::CCursor::new(idx + query.len()),
                            )));
                            egui::TextEdit::store_state(ctx, id, state);
                        }
                    }
                }
            }
        }

        // Top Panel: Menu and Tabs
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Tab").clicked() {
                        let tab = EditorTab::default();
                        self.active_tab_id = Some(tab.id);
                        self.tabs.push(tab);
                        ui.close_menu();
                    }
                    if ui.button("Open").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As").clicked() {
                        self.save_file_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        if let Some(tab) = self.active_tab_mut() {
                            tab.undo();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        if let Some(tab) = self.active_tab_mut() {
                            tab.redo();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Find").clicked() {
                        self.find_state.open = true;
                        self.find_state.replace_mode = false;
                        ui.close_menu();
                    }
                    if ui.button("Replace").clicked() {
                        self.find_state.open = true;
                        self.find_state.replace_mode = true;
                        ui.close_menu();
                    }
                    if ui.button("Go To...").clicked() {
                        self.goto_line_state.open = true;
                        self.goto_line_state.line_str = self.current_cursor_pos.0.to_string();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Time/Date  F5").clicked() {
                         // Simulate F5 press or call logic directly
                         // For simplicity, we just close menu and let user press F5 or duplicate logic.
                         // Duplicating logic is safer here.
                         if let Some(tab) = self.active_tab_mut() {
                            let now = chrono::Local::now();
                            let time_str = now.format("%I:%M %p %m/%d/%Y").to_string();
                            
                            let id = egui::Id::new("editor");
                            if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                                if let Some(range) = state.cursor.char_range() {
                                    let idx = range.primary.index;
                                    tab.push_undo(tab.content.clone());
                                    tab.content.insert_str(idx, &time_str);
                                    tab.is_dirty = true;
                                    
                                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                                        egui::text::CCursor::new(idx + time_str.len()),
                                    )));
                                    egui::TextEdit::store_state(ctx, id, state);
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.checkbox(&mut self.word_wrap, "Word Wrap").clicked() {
                        ui.close_menu();
                    }
                    if ui.checkbox(&mut self.dark_mode, "Dark Mode").clicked() {
                         setup_custom_style(ctx, self.dark_mode);
                         ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Zoom In").clicked() { 
                        let zoom = ctx.zoom_factor();
                        ctx.set_zoom_factor(zoom + 0.1);
                    }
                    if ui.button("Zoom Out").clicked() { 
                         let zoom = ctx.zoom_factor();
                        ctx.set_zoom_factor((zoom - 0.1).max(0.2));
                    }
                    if ui.button("Reset Zoom").clicked() { 
                        ctx.set_zoom_factor(1.0);
                    }
                });
                
                // Plugin Menus
                self.plugin_manager.menu_ui(ui);
            });
            
            ui.add_space(4.0);
            ui::tab_bar(ui, &mut self.tabs, &mut self.active_tab_id);
        });

        // Bottom Panel: Status Bar
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let active_tab_index = self.tabs.iter().position(|t| Some(t.id) == self.active_tab_id);
                
                if let Some(index) = active_tab_index {
                    let (chars, line, col) = {
                        let tab = &self.tabs[index];
                        (tab.content.chars().count(), self.current_cursor_pos.0, self.current_cursor_pos.1)
                    };

                    ui.label(format!("Ln {}, Col {}", line, col));
                    ui.label(format!("Length: {} chars", chars));
                    
                    let mut switch_to_tab = None;
                    ui.menu_button(format!("Tabs: {}", self.tabs.len()), |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        for t in &self.tabs {
                            if ui.button(&t.title).clicked() {
                                switch_to_tab = Some(t.id);
                                ui.close_menu();
                            }
                        }
                    });
                    if let Some(id) = switch_to_tab {
                        self.active_tab_id = Some(id);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(tab) = self.tabs.get_mut(index) {
                            ui.menu_button(tab.encoding.name(), |ui| {
                                if ui.button("UTF-8").clicked() {
                                    tab.encoding = encoding_rs::UTF_8;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                                if ui.button("Windows-1252 (ANSI)").clicked() {
                                    tab.encoding = encoding_rs::WINDOWS_1252;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                                if ui.button("UTF-16LE").clicked() {
                                    tab.encoding = encoding_rs::UTF_16LE;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                                if ui.button("UTF-16BE").clicked() {
                                    tab.encoding = encoding_rs::UTF_16BE;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                            });
                            
                            ui.menu_button(tab.line_ending.name(), |ui| {
                                if ui.button("Windows (CRLF)").clicked() {
                                    tab.line_ending = crate::editor::LineEnding::Crlf;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                                if ui.button("Unix (LF)").clicked() {
                                    tab.line_ending = crate::editor::LineEnding::Lf;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                                if ui.button("Mac (CR)").clicked() {
                                    tab.line_ending = crate::editor::LineEnding::Cr;
                                    tab.is_dirty = true;
                                    ui.close_menu();
                                }
                            });
                        }
                        ui.label("100%");
                    });
                } else {
                     ui.label("Ready");
                }
            });
        });

        // Plugin UI (e.g. side panels, windows)
        // We call this before CentralPanel so plugins can add SidePanels that shrink the central area.
        self.plugin_manager.ui(ctx);

        // Central Panel: Editor
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut new_cursor_pos = None;
            let mut content_changed = false;
            let mut previous_content = String::new();
            let mut tab_changed_idx = None;

            if let Some((idx, tab)) = self.tabs.iter_mut().enumerate().find(|(_i, t)| Some(t.id) == self.active_tab_id) {
                previous_content = tab.content.clone();

                let inner_cursor_pos = egui::ScrollArea::vertical().show(ui, |ui| {
                    let available_height = ui.available_height();
                    let available_width = ui.available_width();
                    let response = ui.add_sized(
                        [available_width, available_height],
                        egui::TextEdit::multiline(&mut tab.content)
                            .id(egui::Id::new("editor"))
                            .frame(false) // Notepad-like look
                            .code_editor()
                            .lock_focus(true)
                            .desired_width(if self.word_wrap { available_width } else { f32::INFINITY })
                    );
                    
                    if response.changed() {
                        content_changed = true;
                        tab.is_dirty = true;
                        tab_changed_idx = Some(idx);
                    }
                    
                    if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                        if let Some(range) = state.cursor.char_range() {
                            let idx = range.primary.index;
                            let text = &tab.content;
                            // Calculate line and col
                            let mut line = 1;
                            let mut col = 1;
                            for (i, c) in text.char_indices() {
                                if i >= idx { break; }
                                if c == '\n' {
                                    line += 1;
                                    col = 1;
                                } else {
                                    col += 1;
                                }
                            }
                            return Some((line, col));
                        }
                    }
                    None
                }).inner;
                new_cursor_pos = inner_cursor_pos;
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No open tabs. Press Ctrl+N to create a new one.");
                });
            }

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
        });
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
    fonts.families.entry(egui::FontFamily::Monospace).or_default()
        .insert(0, "Hack".to_owned()); // If Hack was loaded
    
    ctx.set_fonts(fonts);
}

fn setup_custom_style(ctx: &egui::Context, dark_mode: bool) {
    if dark_mode {
        ctx.set_visuals(egui::Visuals::dark());
    } else {
        ctx.set_visuals(egui::Visuals::light());
        
        // Get the fresh light style to modify
        let mut style = (*ctx.style()).clone();
        
        // Make it look clean and flat like Notepad
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::WHITE;
        style.visuals.window_fill = egui::Color32::WHITE;
        style.visuals.panel_fill = egui::Color32::WHITE;
        
        // Selection color
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        
        ctx.set_style(style);
    }
}
