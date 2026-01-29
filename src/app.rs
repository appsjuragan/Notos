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
    show_line_numbers: bool,
    dark_mode: bool,
    editor_font_size: f32,
    editor_font_family: String,
    custom_fonts: std::collections::HashMap<String, Vec<u8>>,
    close_confirmation: CloseConfirmation,
    // Settings, etc.
}

struct CloseConfirmation {
    open: bool,
    tab_id: Option<uuid::Uuid>, // If Some, we are trying to close a specific tab
    closing_app: bool,          // If true, we are trying to close the whole app
}

impl Default for CloseConfirmation {
    fn default() -> Self {
        Self {
            open: false,
            tab_id: None,
            closing_app: false,
        }
    }
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
            show_line_numbers: false,
            dark_mode: false,
            editor_font_size: 14.0,
            editor_font_family: "Monospace".to_string(),
            custom_fonts: std::collections::HashMap::new(),
            close_confirmation: CloseConfirmation::default(),
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
        if let Some(id) = self.active_tab_id {
            self.save_tab_as_by_id(id);
        }
    }

    fn save_tab_by_id(&mut self, id: uuid::Uuid) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if tab.path.is_some() {
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                }
            } else {
                self.save_tab_as_by_id(id);
            }
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
                tab.set_path(path);
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
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

    fn show_close_confirmation(&mut self, ctx: &egui::Context) {
        if !self.close_confirmation.open {
            return;
        }

        let mut should_close_dialog = false;
        let mut tab_to_ask = None;

        if let Some(id) = self.close_confirmation.tab_id {
            tab_to_ask = self.tabs.iter().find(|t| t.id == id);
        } else if self.close_confirmation.closing_app {
            tab_to_ask = self.tabs.iter().find(|t| t.is_dirty);
        }

        if let Some(tab) = tab_to_ask {
            let tab_id = tab.id;
            let tab_title = tab.title.clone();
            
            egui::Window::new("Save Changes?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label(format!("Do you want to save changes to \"{}\"?", tab_title));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.save_tab_by_id(tab_id);
                            // Check if it was actually saved (not dirty anymore)
                            if let Some(t) = self.tabs.iter().find(|t| t.id == tab_id) {
                                if !t.is_dirty {
                                    if !self.close_confirmation.closing_app {
                                        self.tabs.retain(|t| t.id != tab_id);
                                        if self.active_tab_id == Some(tab_id) {
                                            self.active_tab_id = self.tabs.last().map(|t| t.id);
                                        }
                                        should_close_dialog = true;
                                    }
                                }
                            }
                        }
                        if ui.button("No").clicked() {
                            if !self.close_confirmation.closing_app {
                                self.tabs.retain(|t| t.id != tab_id);
                                if self.active_tab_id == Some(tab_id) {
                                    self.active_tab_id = self.tabs.last().map(|t| t.id);
                                }
                                should_close_dialog = true;
                            } else {
                                // Mark as not dirty so we don't ask again
                                if let Some(t) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                                    t.is_dirty = false;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.close_confirmation.open = false;
                            self.close_confirmation.closing_app = false;
                        }
                    });
                });
        } else {
            // No more dirty tabs or tab already gone
            if self.close_confirmation.closing_app {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            should_close_dialog = true;
        }

        if should_close_dialog {
            self.close_confirmation.open = false;
            self.close_confirmation.tab_id = None;
        }
    }
}

impl eframe::App for NotosApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Window Close
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.tabs.iter().any(|t| t.is_dirty) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.close_confirmation.open = true;
                self.close_confirmation.closing_app = true;
                self.close_confirmation.tab_id = None;
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
                self.close_tab(id);
            }
        }

        // Zoom Shortcuts
        if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals))) {
            self.editor_font_size = (self.editor_font_size + 1.0).clamp(6.0, 72.0);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Minus)) {
            self.editor_font_size = (self.editor_font_size - 1.0).clamp(6.0, 72.0);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) {
            self.editor_font_size = 14.0;
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
                    if ui.checkbox(&mut self.show_line_numbers, "Show Line Number").clicked() {
                        ui.close_menu();
                    }
                    if ui.checkbox(&mut self.dark_mode, "Dark Mode").clicked() {
                         setup_custom_style(ctx, self.dark_mode);
                         ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Zoom In").clicked() { 
                        self.editor_font_size = (self.editor_font_size + 1.0).min(72.0);
                    }
                    if ui.button("Zoom Out").clicked() { 
                         self.editor_font_size = (self.editor_font_size - 1.0).max(6.0);
                    }
                    if ui.button("Reset Zoom").clicked() { 
                        self.editor_font_size = 14.0;
                    }
                    ui.separator();
                    ui.menu_button("Change Font", |ui| {
                        if ui.selectable_label(self.editor_font_family == "Monospace", "Monospace").clicked() {
                            self.editor_font_family = "Monospace".to_string();
                            ui.close_menu();
                        }
                        if ui.selectable_label(self.editor_font_family == "Proportional", "Proportional").clicked() {
                            self.editor_font_family = "Proportional".to_string();
                            ui.close_menu();
                        }
                        
                        if !self.custom_fonts.is_empty() {
                            ui.separator();
                            for name in self.custom_fonts.keys() {
                                if ui.selectable_label(&self.editor_font_family == name, name).clicked() {
                                    self.editor_font_family = name.clone();
                                    ui.close_menu();
                                }
                            }
                        }

                        ui.separator();
                        if ui.button("Load Font File...").clicked() {
                            if let Some(path) = FileDialog::new()
                                .add_filter("Font", &["ttf", "otf"])
                                .pick_file() 
                            {
                                if let Ok(bytes) = std::fs::read(&path) {
                                    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                    self.custom_fonts.insert(name.clone(), bytes.clone());
                                    
                                    // Update egui fonts
                                    let mut fonts = egui::FontDefinitions::default();
                                    // Re-add all custom fonts
                                    for (n, b) in &self.custom_fonts {
                                        fonts.font_data.insert(n.clone(), egui::FontData::from_owned(b.clone()));
                                        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, n.clone());
                                        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, n.clone());
                                    }
                                    ctx.set_fonts(fonts);
                                    
                                    self.editor_font_family = name;
                                }
                            }
                            ui.close_menu();
                        }
                    });
                });
                
                // Plugin Menus
                self.plugin_manager.menu_ui(ui);
            });
            
            ui.add_space(4.0);
            if let Some(action) = ui::tab_bar(ui, &self.tabs, self.active_tab_id) {
                match action {
                    ui::TabAction::New => {
                        let tab = EditorTab::default();
                        self.active_tab_id = Some(tab.id);
                        self.tabs.push(tab);
                    }
                    ui::TabAction::Select(id) => {
                        self.active_tab_id = Some(id);
                    }
                    ui::TabAction::Close(id) => {
                        self.close_tab(id);
                    }
                    ui::TabAction::CloseOthers(id) => {
                        // For simplicity, we'll just close them one by one or handle it specifically
                        let ids_to_close: Vec<_> = self.tabs.iter()
                            .filter(|t| t.id != id)
                            .map(|t| t.id)
                            .collect();
                        for close_id in ids_to_close {
                            self.close_tab(close_id);
                        }
                    }
                }
            }
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
                    let mut close_tab_id = None;
                    ui.menu_button(format!("Tabs: {}", self.tabs.len()), |ui| {
                        ui.set_width(220.0); // Fixed width for a more "solid" look
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        
                        for t in &self.tabs {
                            ui.horizontal(|ui| {
                                let is_active = Some(t.id) == self.active_tab_id;
                                
                                // Allocate space for the label, leaving room for the close button
                                let label_width = ui.available_width() - 30.0;
                                ui.allocate_ui(egui::vec2(label_width, ui.available_height()), |ui| {
                                    if ui.selectable_label(is_active, &t.title).clicked() {
                                        switch_to_tab = Some(t.id);
                                        ui.close_menu();
                                    }
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("x").clicked() {
                                        close_tab_id = Some(t.id);
                                    }
                                });
                            });
                        }
                    });
                    if let Some(id) = switch_to_tab {
                        self.active_tab_id = Some(id);
                    }
                    if let Some(id) = close_tab_id {
                        self.close_tab(id);
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
                        ui.label(format!("{}%", (self.editor_font_size / 14.0 * 100.0) as i32));
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
            // Handle Ctrl + Scroll for Zoom
            if ctx.input(|i| i.modifiers.ctrl) {
                let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
                if scroll_delta != 0.0 {
                    let delta = if scroll_delta > 0.0 { 1.0 } else { -1.0 };
                    self.editor_font_size = (self.editor_font_size + delta).clamp(6.0, 72.0);
                }
            }

            let mut new_cursor_pos = None;
            let mut content_changed = false;
            let mut previous_content = String::new();
            let mut tab_changed_idx = None;

            if let Some((idx, tab)) = self.tabs.iter_mut().enumerate().find(|(_i, t)| Some(t.id) == self.active_tab_id) {
                previous_content = tab.content.clone();

                // Apply font size and family to the editor scope
                ui.scope(|ui| {
                    let font_id = if self.editor_font_family == "Proportional" {
                        egui::FontId::proportional(self.editor_font_size)
                    } else if self.editor_font_family == "Monospace" {
                        egui::FontId::monospace(self.editor_font_size)
                    } else {
                        egui::FontId::new(self.editor_font_size, egui::FontFamily::Name(self.editor_font_family.clone().into()))
                    };

                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Monospace,
                        font_id.clone(),
                    );
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Body,
                        font_id.clone(),
                    );

                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                        let available_height = ui.available_height();
                        
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            
                            let font_id = font_id.clone();
                            let margin = 2.0;

                            let line_number_width = if self.show_line_numbers {
                                let line_count = tab.content.lines().count().max(1);
                                let line_count = if tab.content.ends_with('\n') { line_count + 1 } else { line_count };
                                let num_digits = line_count.to_string().len().max(2);
                                (num_digits as f32 * self.editor_font_size * 0.6) + 12.0
                            } else {
                                0.0
                            };

                            if self.show_line_numbers {
                                // Reserve space for line numbers
                                ui.add_space(line_number_width + 8.0); 
                            }

                            let editor_bg = if ui.visuals().dark_mode {
                                egui::Color32::from_gray(75) // Even lighter grey for text area as requested
                            } else {
                                egui::Color32::WHITE
                            };

                            let mut response = None;
                            let mut galley_to_draw = None;

                            if self.show_line_numbers {
                                let galley = ui.fonts(|f| {
                                    let layout_job = egui::text::LayoutJob::simple(
                                        tab.content.clone(),
                                        font_id.clone(),
                                        ui.visuals().widgets.noninteractive.text_color(),
                                        if self.word_wrap { ui.available_width() - margin * 2.0 } else { f32::INFINITY },
                                    );
                                    f.layout_job(layout_job)
                                });
                                galley_to_draw = Some(galley);
                            }

                            egui::Frame::none()
                                .fill(editor_bg)
                                .show(ui, |ui| {
                                    let res = ui.add_sized(
                                        [ui.available_width(), available_height],
                                        egui::TextEdit::multiline(&mut tab.content)
                                            .id(egui::Id::new("editor"))
                                            .font(font_id.clone())
                                            .frame(false)
                                            .code_editor()
                                            .lock_focus(true)
                                            .margin(egui::Margin::same(margin))
                                            .desired_width(if self.word_wrap { ui.available_width() } else { f32::INFINITY })
                                    );
                                    
                                    if res.changed() {
                                        content_changed = true;
                                        tab.is_dirty = true;
                                        tab_changed_idx = Some(idx);
                                    }
                                    
                                    if let Some(state) = egui::TextEdit::load_state(ui.ctx(), res.id) {
                                        if let Some(range) = state.cursor.char_range() {
                                            let idx = range.primary.index;
                                            let text = &tab.content;
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
                                            new_cursor_pos = Some((line, col));
                                        }
                                    }
                                    response = Some(res);
                                });

                            if self.show_line_numbers {
                                if let (Some(res), Some(galley)) = (response, galley_to_draw) {
                                    let painter = ui.painter();
                                    let mut logical_line = 1;
                                    let mut is_start_of_logical_line = true;
                                    
                                    // Draw background for line numbers
                                    let line_num_rect = egui::Rect::from_min_max(
                                        egui::pos2(res.rect.min.x - line_number_width - 8.0, res.rect.min.y),
                                        egui::pos2(res.rect.min.x, res.rect.max.y)
                                    );
                                    painter.rect_filled(line_num_rect, 0.0, ui.visuals().widgets.noninteractive.bg_fill);
                                    
                                    // Draw separator
                                    painter.line_segment(
                                        [egui::pos2(res.rect.min.x - 2.0, res.rect.min.y), egui::pos2(res.rect.min.x - 2.0, res.rect.max.y)],
                                        ui.visuals().widgets.noninteractive.bg_stroke
                                    );

                                    for row in &galley.rows {
                                        if is_start_of_logical_line {
                                            let pos = egui::pos2(
                                                res.rect.min.x - 8.0, 
                                                res.rect.min.y + margin + row.rect.min.y
                                            );
                                            painter.text(
                                                pos,
                                                egui::Align2::RIGHT_TOP,
                                                logical_line.to_string(),
                                                font_id.clone(),
                                                ui.visuals().weak_text_color()
                                            );
                                            logical_line += 1;
                                        }
                                        is_start_of_logical_line = row.ends_with_newline;
                                    }
                                }
                            }
                        });
                        None::<(usize, usize)>
                    }).inner;
                });
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

        self.show_close_confirmation(ctx);
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
        
        let mut style = (*ctx.style()).clone();
        // Lighter grey background for dark mode as requested
        let dark_grey = egui::Color32::from_gray(40);
        style.visuals.widgets.noninteractive.bg_fill = dark_grey;
        style.visuals.window_fill = dark_grey;
        style.visuals.panel_fill = dark_grey;
        style.visuals.extreme_bg_color = egui::Color32::from_gray(55); // Even lighter for text area
        
        ctx.set_style(style);
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
