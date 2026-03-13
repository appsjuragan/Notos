use eframe::egui;
use rfd::FileDialog;

use crate::editor::EditorTab;

use super::get_ed_ctx;

use super::style::setup_custom_style;
use super::NotosApp;

impl eframe::App for NotosApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle IPC messages (files opened from other instances)
        while let Ok(path_str) = self.ipc_receiver.try_recv() {
            let path = std::path::PathBuf::from(path_str);
            if path.exists() && path.is_file() {
                self.open_path(path);
                ctx.request_repaint();
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        // Handle background file loads
        while let Ok((path, result)) = self.file_load_receiver.try_recv() {
            self.loading_paths.remove(&path);
            match result {
                Ok(mut tab) => {
                    self.add_to_recent(path);
                    tab.scroll_to_cursor = true;
                    self.active_tab_id = Some(tab.id);
                    self.tabs.push(tab);
                    ctx.request_repaint();
                }
                Err(e) => {
                    log::error!("Background load failed for {:?}: {}", path, e);
                }
            }
        }

        // Periodic session save (every 30 seconds)
        if self.last_session_save.elapsed() >= std::time::Duration::from_secs(30) {
            if let Err(e) = self.save_session() {
                log::error!("Failed to save periodic session: {}", e);
            }
            self.last_session_save = std::time::Instant::now();
        }

        // Only rebuild visuals when dark_mode actually changes OR when eframe overrides our style
        let current_panel_fill = ctx.style().visuals.panel_fill;
        let expected_panel_fill = if self.dark_mode {
            egui::Color32::from_gray(38)
        } else {
            egui::Color32::WHITE
        };
        
        if self.prev_dark_mode != self.dark_mode || current_panel_fill != expected_panel_fill {
            setup_custom_style(ctx, self.dark_mode);
            self.prev_dark_mode = self.dark_mode;
        }

        // Handle Window Close
        if ctx.input(|i| i.viewport().close_requested()) {
            match self.save_session() {
                Ok(_) => {
                    // Session saved, allow close without confirmation
                }
                Err(e) => {
                    log::error!("Failed to save session: {}", e);
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
                match EditorTab::from_file(path.clone()) {
                    Ok(tab) => {
                        self.active_tab_id = Some(tab.id);
                        self.tabs.push(tab);
                        self.add_to_recent(path);
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
        self.find_dialog.show(ctx, active_tab, &mut self.undo_manager);

        let active_tab = self
            .tabs
            .iter_mut()
            .find(|t| Some(t.id) == self.active_tab_id);
        self.goto_dialog.show(ctx, active_tab);

        // Close Confirmation
        let save_fn = |tab: &mut EditorTab| -> std::result::Result<(), Box<dyn std::error::Error>> {
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
                Err("Cancelled".into())
            }
        };

        self.close_confirmation
            .show(ctx, &mut self.tabs, &mut self.active_tab_id, &mut self.undo_manager, save_fn);

        // Determine background colors
        let panel_bg = if self.dark_mode {
            egui::Color32::from_gray(38)
        } else {
            egui::Color32::WHITE
        };

        // Create EditorContext for plugins
        let mut menu_action_to_run = None;
        let mut plugin_action_to_run_top = notos_sdk::PluginAction::None;
        let mut tab_action_to_run = None;

        let plugin_manager = &mut self.plugin_manager;
        let word_wrap = &mut self.word_wrap;
        let show_line_numbers = &mut self.show_line_numbers;
        let dark_mode = &mut self.dark_mode;
        let editor_font_family = &self.editor_font_family;
        let custom_fonts = &self.custom_fonts;
        let recent_files = &self.recent_files;
        let tabs = &self.tabs;
        let active_tab_id = self.active_tab_id;

        // Top Panel: Menu and Tabs
        egui::TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::default()
                    .fill(panel_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 2.0)),
            )
            .show(ctx, |ui| {
                let ed_ctx = get_ed_ctx(tabs, active_tab_id, self.hovered_char_idx);
                let (m, p) = crate::ui::menu_bar(
                    ui,
                    plugin_manager,
                    word_wrap,
                    show_line_numbers,
                    dark_mode,
                    editor_font_family,
                    custom_fonts,
                    recent_files,
                    &ed_ctx,
                );
                menu_action_to_run = m;
                plugin_action_to_run_top = p;

                ui.add_space(4.0);
                tab_action_to_run = crate::ui::tab_bar(ui, tabs, active_tab_id, &self.loading_paths);
            });

        if let Some(action) = menu_action_to_run {
            self.handle_menu_action(action, ctx);
        }
        self.handle_plugin_action(plugin_action_to_run_top, ctx);

        if let Some(action) = tab_action_to_run {
            match action {
                crate::ui::TabAction::New => {
                    let mut tab = EditorTab::default();
                    tab.scroll_to_cursor = true;
                    self.active_tab_id = Some(tab.id);
                    self.tabs.push(tab);
                }
                crate::ui::TabAction::Select(id) => {
                    self.active_tab_id = Some(id);
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                        tab.scroll_to_cursor = true;
                    }
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
                        crate::ui::StatusBarAction::SetLineEnding(id, le) => {
                            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                                tab.line_ending = le;
                                tab.is_dirty = true;
                            }
                        }
                        crate::ui::StatusBarAction::SetEncoding(id, enc) => {
                            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                                tab.encoding = enc;
                                tab.is_dirty = true;
                            }
                        }
                    }
                }
            });

        // Central Panel: Editor
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(if self.dark_mode {
                        egui::Color32::from_gray(28)
                    } else {
                        egui::Color32::WHITE
                    })
                    .inner_margin(egui::Margin {
                        left: 4.0,
                        right: 0.0,
                        top: 0.0,
                        bottom: 0.0,
                    }),
            )
            .show(ctx, |ui| {
                self.show_editor_panel(ctx, ui);
            });

        let ed_ctx = get_ed_ctx(&self.tabs, self.active_tab_id, self.hovered_char_idx);
        let plugin_action = self.plugin_manager.ui(ctx, &ed_ctx);
        if let notos_sdk::PluginAction::UnderlineRegion(start, end) = plugin_action {
            self.next_underline = Some((start, end));
            ctx.request_repaint();
        } else {
            self.next_underline = None;
            self.handle_plugin_action(plugin_action, ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.plugin_manager.on_unload();
    }
}
