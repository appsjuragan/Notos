use crate::editor::EditorTab;
use eframe::egui;

#[derive(Default)]
pub struct FindDialog {
    pub open: bool,
    pub query: String,
    pub replace_with: String,
    pub match_case: bool,
    pub replace_mode: bool,
}

impl FindDialog {
    pub fn show(&mut self, ctx: &egui::Context, mut active_tab: Option<&mut EditorTab>) {
        let mut open = self.open;
        let mut find_next_clicked = false;

        if open {
            let title = if self.replace_mode { "Replace" } else { "Find" };
            egui::Window::new(title).open(&mut open).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let res = ui.text_edit_singleline(&mut self.query);
                    if self.open && !find_next_clicked {
                        res.request_focus();
                    }

                    if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        find_next_clicked = true;
                    }
                });

                if self.replace_mode {
                    ui.horizontal(|ui| {
                        ui.label("Replace with:");
                        ui.text_edit_singleline(&mut self.replace_with);
                    });
                }

                ui.checkbox(&mut self.match_case, "Match case");

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button_size = egui::vec2(80.0, 24.0);

                        if self.replace_mode {
                            if ui.add_sized(button_size, egui::Button::new("Replace All")).clicked() {
                                self.perform_replace_all(active_tab.as_deref_mut());
                            }
                            if ui.add_sized(button_size, egui::Button::new("Replace")).clicked() {
                                self.perform_replace(
                                    ctx,
                                    active_tab.as_deref_mut(),
                                    &mut find_next_clicked,
                                );
                            }
                        }

                        if ui.add_sized(button_size, egui::Button::new("Find Next")).clicked() {
                            find_next_clicked = true;
                        }
                    });
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
                let id = egui::Id::new("editor");
                // Check if current selection matches query
                if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(range) = state.cursor.char_range() {
                        let start = range.primary.index.min(range.secondary.index);
                        let end = range.primary.index.max(range.secondary.index);

                        if start < tab.content.len() && end <= tab.content.len() {
                            let selected_text = &tab.content[start..end];
                            if selected_text == query {
                                // Replace
                                tab.push_undo(tab.content.clone());
                                tab.content.replace_range(start..end, replace);
                                tab.is_dirty = true;

                                // Update cursor to end of replacement
                                let new_idx = start + replace.len();
                                state
                                    .cursor
                                    .set_char_range(Some(egui::text::CCursorRange::one(
                                        egui::text::CCursor::new(new_idx),
                                    )));
                                egui::TextEdit::store_state(ctx, id, state);
                                tab.scroll_to_cursor = true;
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
                let id = egui::Id::new("editor");
                let mut start_idx = 0;

                if let Some(state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(range) = state.cursor.char_range() {
                        // Start searching after the current selection/cursor
                        start_idx = range.primary.index.max(range.secondary.index);
                    }
                }

                let search_slice = if start_idx < text.len() {
                    &text[start_idx..]
                } else {
                    ""
                };

                let found_idx = search_slice.find(query).map(|i| start_idx + i).or_else(|| {
                    // Wrap around
                    text.find(query)
                });

                if let Some(idx) = found_idx {
                    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(idx),
                                egui::text::CCursor::new(idx + query.len()),
                            )));
                        egui::TextEdit::store_state(ctx, id, state);
                        tab.scroll_to_cursor = true;
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
                .open(&mut goto_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Line number:");
                        let res = ui.text_edit_singleline(&mut self.line_str);
                        if self.open && !goto_clicked {
                            res.request_focus();
                        }
                        if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            goto_clicked = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let button_size = egui::vec2(80.0, 24.0);
                            if ui.add_sized(button_size, egui::Button::new("Go To")).clicked() {
                                goto_clicked = true;
                            }
                        });
                    });
                });
        }
        self.open = goto_open;

        if goto_clicked {
            if let Ok(target_line) = self.line_str.parse::<usize>() {
                if let Some(tab) = active_tab {
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
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::new(char_idx),
                            )));
                        egui::TextEdit::store_state(ctx, id, state);

                        // Force scroll to cursor
                        tab.scroll_to_cursor = true;
                        ctx.request_repaint();
                        // We need to request a repaint to ensure the scroll happens
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
    pub tab_id: Option<uuid::Uuid>,
    pub closing_app: bool,
}

impl CloseConfirmationDialog {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        tabs: &mut Vec<EditorTab>,
        active_tab_id: &mut Option<uuid::Uuid>,
        save_tab_fn: impl Fn(&mut EditorTab) -> anyhow::Result<()>,
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
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label(format!("Do you want to save changes to \"{}\"?", tab_title));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let button_size = egui::vec2(80.0, 24.0);

                            if ui.add_sized(button_size, egui::Button::new("Cancel")).clicked() {
                                self.open = false;
                                self.closing_app = false;
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

                            if ui.add_sized(button_size, egui::Button::new("Yes")).clicked() {
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
                        });
                    });
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
