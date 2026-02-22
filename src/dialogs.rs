use crate::editor::EditorTab;
use eframe::egui;

#[derive(Default)]
pub struct FindDialog {
    pub open: bool,
    pub query: String,
    pub replace_with: String,
    pub match_case: bool,
    pub replace_mode: bool,
    pub just_opened: bool,
}

impl FindDialog {
    pub fn show(&mut self, ctx: &egui::Context, mut active_tab: Option<&mut EditorTab>) {
        let mut open = self.open;
        let mut find_next_clicked = false;

        if open {
            let title = if self.replace_mode { "Replace" } else { "Find" };
            egui::Window::new(title)
                .id(egui::Id::new("find_replace_dialog_v4"))
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .default_size([320.0, 100.0])
                .show(ctx, |ui| {
                    egui::Grid::new("find_replace_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Find:");
                            let mut res = None;
                            egui::Frame::none()
                                .fill(ui.visuals().widgets.inactive.bg_fill)
                                .stroke(ui.visuals().widgets.inactive.bg_stroke)
                                .rounding(ui.visuals().widgets.inactive.rounding)
                                .inner_margin(2.0)
                                .show(ui, |ui| {
                                    res = Some(
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.query)
                                                .frame(false)
                                                .desired_width(f32::INFINITY),
                                        ),
                                    );
                                });

                            if let Some(res) = res {
                                if self.just_opened {
                                    res.request_focus();
                                    self.just_opened = false;
                                }

                                if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    find_next_clicked = true;
                                }
                            }
                            ui.end_row();

                            if self.replace_mode {
                                ui.label("Replace:");
                                egui::Frame::none()
                                    .fill(ui.visuals().widgets.inactive.bg_fill)
                                    .stroke(ui.visuals().widgets.inactive.bg_stroke)
                                    .rounding(ui.visuals().widgets.inactive.rounding)
                                    .inner_margin(2.0)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.replace_with)
                                                .frame(false)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });
                                ui.end_row();
                            }
                        });

                    ui.checkbox(&mut self.match_case, "Match case");

                    ui.add_space(8.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button_size = egui::vec2(80.0, 24.0);

                        if self.replace_mode {
                            if ui
                                .add_sized(button_size, egui::Button::new("Replace All"))
                                .clicked()
                            {
                                self.perform_replace_all(active_tab.as_deref_mut());
                            }
                            if ui
                                .add_sized(button_size, egui::Button::new("Replace"))
                                .clicked()
                            {
                                self.perform_replace(
                                    ctx,
                                    active_tab.as_deref_mut(),
                                    &mut find_next_clicked,
                                );
                            }
                        }

                        if ui
                            .add_sized(button_size, egui::Button::new("Find Next"))
                            .clicked()
                        {
                            find_next_clicked = true;
                        }
                    });
                });
        }
        self.open = open;

        if find_next_clicked {
            self.perform_find_next(ctx, active_tab);
        }
    }

    fn perform_replace(
        &self,
        ctx: &egui::Context,
        active_tab: Option<&mut EditorTab>,
        find_next_clicked: &mut bool,
    ) {
        let query = &self.query;
        let replace = &self.replace_with;

        if !query.is_empty() {
            if let Some(tab) = active_tab {
                let id = egui::Id::new("editor").with(tab.id);
                // Check if current selection matches query
                if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(range) = state.cursor.char_range() {
                        // egui gives us char counts; convert to byte offsets for slice ops
                        let char_start = range.primary.index.min(range.secondary.index);
                        let char_end = range.primary.index.max(range.secondary.index);
                        let byte_start = tab
                            .content
                            .char_indices()
                            .nth(char_start)
                            .map(|(i, _)| i)
                            .unwrap_or(tab.content.len());
                        let byte_end = tab
                            .content
                            .char_indices()
                            .nth(char_end)
                            .map(|(i, _)| i)
                            .unwrap_or(tab.content.len());

                        if byte_start <= tab.content.len() && byte_end <= tab.content.len() {
                            let selected_text = &tab.content[byte_start..byte_end];
                            if selected_text == query {
                                // Replace
                                tab.push_undo(tab.content.clone());
                                tab.content.replace_range(byte_start..byte_end, replace);
                                tab.is_dirty = true;

                                // new cursor position: byte offset -> char count
                                let new_byte = byte_start + replace.len();
                                let new_char = tab.content[..new_byte].chars().count();
                                state
                                    .cursor
                                    .set_char_range(Some(egui::text::CCursorRange::one(
                                        egui::text::CCursor::new(new_char),
                                    )));
                                egui::TextEdit::store_state(ctx, id, state);
                                tab.cursor_range = Some((new_char, new_char));
                                tab.scroll_to_cursor = true;
                                tab.center_cursor = true;
                                ctx.request_repaint();
                            }
                        }
                    }
                }
                // Find next occurrence
                *find_next_clicked = true;
            }
        }
    }

    fn perform_replace_all(&self, active_tab: Option<&mut EditorTab>) {
        let query = &self.query;
        let replace = &self.replace_with;

        if !query.is_empty() {
            if let Some(tab) = active_tab {
                let new_content = tab.content.replace(query, replace);
                if new_content != tab.content {
                    tab.push_undo(tab.content.clone());
                    tab.content = new_content;
                    tab.is_dirty = true;
                }
            }
        }
    }

    fn perform_find_next(&self, ctx: &egui::Context, active_tab: Option<&mut EditorTab>) {
        let query = &self.query;

        if !query.is_empty() {
            if let Some(tab) = active_tab {
                let text = &tab.content;
                let id = egui::Id::new("editor").with(tab.id);

                // egui cursor gives char count; convert to byte offset for str::find
                let mut start_byte = 0usize;
                if let Some(state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(range) = state.cursor.char_range() {
                        let char_pos = range.primary.index.max(range.secondary.index);
                        start_byte = text
                            .char_indices()
                            .nth(char_pos)
                            .map(|(i, _)| i)
                            .unwrap_or(text.len());
                    }
                }

                // Find next match by byte offset
                let found_byte = text[start_byte..]
                    .find(query)
                    .map(|i| start_byte + i)
                    .or_else(|| text.find(query)); // wrap around

                if let Some(byte_idx) = found_byte {
                    let byte_end = byte_idx + query.len();
                    // Convert byte offsets to char counts for egui CCursor
                    let char_idx = text[..byte_idx].chars().count();
                    let char_end = text[..byte_end].chars().count();

                    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(char_idx),
                                egui::text::CCursor::new(char_end),
                            )));
                        egui::TextEdit::store_state(ctx, id, state);
                        // Store char counts so editor_panel uses consistent units
                        tab.cursor_range = Some((char_idx, char_end));
                        tab.scroll_to_cursor = true;
                        tab.center_cursor = true;
                        ctx.request_repaint();
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct GotoLineDialog {
    pub open: bool,
    pub line_str: String,
}

impl GotoLineDialog {
    pub fn show(&mut self, ctx: &egui::Context, active_tab: Option<&mut EditorTab>) {
        let mut goto_open = self.open;
        let mut goto_clicked = false;
        if goto_open {
            egui::Window::new("Go To Line")
                .id(egui::Id::new("gotoline_dialog_v4"))
                .open(&mut goto_open)
                .collapsible(false)
                .resizable(false)
                .default_size([220.0, 80.0])
                .show(ctx, |ui| {
                    egui::Grid::new("gotoline_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Line number:");
                            let mut res = None;
                            egui::Frame::none()
                                .fill(ui.visuals().widgets.inactive.bg_fill)
                                .stroke(ui.visuals().widgets.inactive.bg_stroke)
                                .rounding(ui.visuals().widgets.inactive.rounding)
                                .inner_margin(2.0)
                                .show(ui, |ui| {
                                    res = Some(
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.line_str)
                                                .frame(false)
                                                .desired_width(f32::INFINITY),
                                        ),
                                    );
                                });

                            if let Some(res) = res {
                                if self.open && !goto_clicked {
                                    res.request_focus();
                                }
                                if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    goto_clicked = true;
                                }
                            }
                            ui.end_row();
                        });

                    ui.add_space(4.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button_size = egui::vec2(80.0, 24.0);
                        if ui
                            .add_sized(button_size, egui::Button::new("Go To"))
                            .clicked()
                        {
                            goto_clicked = true;
                        }
                    });
                });
        }
        self.open = goto_open;

        if goto_clicked {
            if let Ok(target_line) = self.line_str.parse::<usize>() {
                if let Some(tab) = active_tab {
                    let text = &tab.content;

                    // Find the byte offset of the target line start
                    let mut current_line = 1;
                    let mut byte_idx = 0usize;
                    for (i, c) in text.char_indices() {
                        if current_line == target_line {
                            byte_idx = i;
                            break;
                        }
                        if c == '\n' {
                            current_line += 1;
                        }
                    }
                    if current_line < target_line {
                        byte_idx = text.len();
                    }

                    // Convert byte offset to char count for egui CCursor
                    let char_idx = text[..byte_idx].chars().count();

                    let id = egui::Id::new("editor").with(tab.id);
                    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::new(char_idx),
                            )));
                        egui::TextEdit::store_state(ctx, id, state);

                        // Store char count so editor_panel uses consistent units
                        tab.cursor_range = Some((char_idx, char_idx));

                        tab.scroll_to_cursor = true;
                        tab.center_cursor = true;
                        ctx.request_repaint();

                        self.open = false;
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct CloseConfirmationDialog {
    pub open: bool,
    pub tab_id: Option<crate::editor::TabId>,
    pub closing_app: bool,
}

impl CloseConfirmationDialog {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        tabs: &mut Vec<EditorTab>,
        active_tab_id: &mut Option<crate::editor::TabId>,
        save_tab_fn: impl Fn(&mut EditorTab) -> std::result::Result<(), Box<dyn std::error::Error>>,
    ) {
        if !self.open {
            return;
        }

        let mut should_close_dialog = false;
        let mut tab_to_ask_idx = None;

        if let Some(id) = self.tab_id {
            tab_to_ask_idx = tabs.iter().position(|t| t.id == id);
        } else if self.closing_app {
            tab_to_ask_idx = tabs.iter().position(|t| t.is_dirty);
        }

        if let Some(idx) = tab_to_ask_idx {
            // Get title and id without keeping mutable borrow
            let (tab_title, tab_id) = {
                let tab = &tabs[idx];
                (tab.title.clone(), tab.id)
            };

            egui::Window::new("Save Changes?")
                .id(egui::Id::new("close_confirmation_dialog_v4"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .default_size([320.0, 120.0])
                .show(ctx, |ui| {
                    ui.label(format!("Do you want to save changes to \"{}\"?", tab_title));
                    ui.add_space(12.0);
                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Min)
                            .with_main_align(egui::Align::Center),
                        |ui| {
                            let button_size = egui::vec2(80.0, 24.0);

                            if ui
                                .add_sized(button_size, egui::Button::new("Yes"))
                                .clicked()
                            {
                                let saved = {
                                    let tab = &mut tabs[idx];
                                    save_tab_fn(tab).is_ok() && !tab.is_dirty
                                };

                                if saved && !self.closing_app {
                                    tabs.remove(idx);
                                    if *active_tab_id == Some(tab_id) {
                                        *active_tab_id = tabs.last().map(|t| t.id);
                                    }
                                    should_close_dialog = true;
                                }
                            }

                            if ui.add_sized(button_size, egui::Button::new("No")).clicked() {
                                if !self.closing_app {
                                    tabs.remove(idx);
                                    if *active_tab_id == Some(tab_id) {
                                        *active_tab_id = tabs.last().map(|t| t.id);
                                    }
                                    should_close_dialog = true;
                                } else {
                                    // Mark as not dirty so we don't ask again
                                    if let Some(tab) = tabs.get_mut(idx) {
                                        tab.is_dirty = false;
                                    }
                                }
                            }

                            if ui
                                .add_sized(button_size, egui::Button::new("Cancel"))
                                .clicked()
                            {
                                self.open = false;
                                self.closing_app = false;
                            }
                        },
                    );
                });
        } else {
            // No more dirty tabs or tab already gone
            if self.closing_app {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            should_close_dialog = true;
        }

        if should_close_dialog {
            self.open = false;
            self.tab_id = None;
        }
    }
}
