use eframe::egui;
use rfd::FileDialog;

use crate::editor::EditorTab;
use crate::ui::MenuAction;

use super::style::setup_custom_style;
use super::NotosApp;

impl NotosApp {
    pub(crate) fn handle_plugin_action(
        &mut self,
        action: notos_sdk::PluginAction,
        ctx: &egui::Context,
    ) {
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
            PluginAction::UnderlineRegion(_, _) => {}
        }
    }

    pub(crate) fn handle_menu_action(&mut self, action: MenuAction, ctx: &egui::Context) {
        match action {
            MenuAction::NewTab => {
                let tab = EditorTab::default();
                self.active_tab_id = Some(tab.id);
                self.tabs.push(tab);
            }
            MenuAction::Open => self.open_file(),
            MenuAction::OpenRecent(path) => {
                self.open_path(path);
            }
            MenuAction::ClearHistory => {
                self.recent_files.clear();
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

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
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
            self.handle_menu_action(MenuAction::TimeDate, ctx);
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
            self.handle_menu_action(MenuAction::GotoLine, ctx);
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.handle_menu_action(MenuAction::Find, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.handle_menu_action(MenuAction::Replace, ctx);
        }
    }
}
