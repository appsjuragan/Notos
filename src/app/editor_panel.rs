use eframe::egui;

use super::NotosApp;

/// The `DeferredAction` enum for context menu actions in the editor panel.
#[derive(PartialEq)]
pub(crate) enum DeferredAction {
    None,
    Plugin(notos_sdk::PluginAction),
    Undo,
    Redo,
    SelectAll,
    Cut,
    Copy,
    Paste,
}

impl NotosApp {
    /// Renders the central editor panel.
    pub(crate) fn show_editor_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let plugin_manager = &mut self.plugin_manager;
        let tabs = &mut self.tabs;
        let active_tab_id = self.active_tab_id;

        let idx = tabs
            .iter()
            .position(|t| Some(t.id) == active_tab_id)
            .unwrap_or(0);

        let mut hovered_idx_out = None;

        if let Some(tab) = tabs.get_mut(idx) {
            let mut content_changed = false;
            let previous_content = tab.content.clone();

            let mut new_cursor_pos = None;
            let mut tab_changed_idx = None;

            let mut deferred_action = DeferredAction::None;

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

                let _available_height = ui.available_height();

                let editor_bg = if self.dark_mode {
                    egui::Color32::from_gray(28)
                } else {
                    egui::Color32::WHITE
                };

                let mut text_edit_res = None;
                let mut galley_to_draw: Option<std::sync::Arc<egui::Galley>> = None;

                ui.horizontal(|ui| {
                    if self.show_line_numbers {
                        ui.add_space(line_number_width + 8.0);
                    }

                    let next_underline_ref = self.next_underline;
                    let word_wrap = self.word_wrap;

                    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                        let mut layout_job = egui::text::LayoutJob::default();
                        let base_format = egui::TextFormat {
                            font_id: font_id.clone(),
                            color: ui.visuals().widgets.noninteractive.text_color(),
                            ..Default::default()
                        };

                        if let Some((start, end)) = next_underline_ref {
                            if start < string.len() && end <= string.len() && start < end {
                                layout_job.append(&string[..start], 0.0, base_format.clone());
                                let mut ul_format = base_format.clone();
                                ul_format.underline = egui::Stroke::new(
                                    1.0,
                                    ui.visuals().widgets.noninteractive.text_color(),
                                );
                                layout_job.append(&string[start..end], 0.0, ul_format);
                                layout_job.append(&string[end..], 0.0, base_format.clone());
                            } else {
                                layout_job.append(string, 0.0, base_format.clone());
                            }
                        } else {
                            layout_job.append(string, 0.0, base_format.clone());
                        }

                        layout_job.wrap.max_width =
                            if word_wrap { wrap_width } else { f32::INFINITY };

                        ui.fonts(|f| f.layout_job(layout_job))
                    };

                    egui::Frame::none().fill(editor_bg).show(ui, |ui| {
                        let mut force_scroll = false;
                        if tab.scroll_to_cursor {
                            force_scroll = true;
                            let id = egui::Id::new("editor").with(tab.id);
                            ui.memory_mut(|mem| mem.request_focus(id));
                            let mut state =
                                egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                            if let Some((p, s)) = tab.cursor_range {
                                state
                                    .cursor
                                    .set_char_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(p),
                                        egui::text::CCursor::new(s),
                                    )));
                            }
                            egui::TextEdit::store_state(ui.ctx(), id, state);
                            tab.scroll_to_cursor = false;
                        }

                        let text_edit = egui::TextEdit::multiline(&mut tab.content)
                            .id(egui::Id::new("editor").with(tab.id))
                            .font(font_id.clone())
                            .frame(false)
                            .code_editor()
                            .lock_focus(true)
                            .margin(egui::Margin::same(margin))
                            .layouter(&mut layouter)
                            .desired_width(if word_wrap {
                                ui.available_width()
                            } else {
                                f32::INFINITY
                            });

                        let output = text_edit.show(ui);
                        let res = output.response;

                        galley_to_draw = Some(output.galley.clone());

                        text_edit_res = Some(res.clone());

                        // Render Find Highlight (Undermost Layer) if Dialog Active
                        if self.find_dialog.open && !self.find_dialog.query.is_empty() {
                            let text = &tab.content;
                            let query = &self.find_dialog.query;
                            let match_case = self.find_dialog.match_case;
                            let active_range = tab.cursor_range;
                            let mut match_ranges = Vec::new();

                            let mut last_idx = 0;
                            if match_case {
                                while let Some(idx) = text[last_idx..].find(query) {
                                    let start = last_idx + idx;
                                    let end = start + query.len();
                                    if text.is_char_boundary(start) && text.is_char_boundary(end) {
                                        match_ranges.push((start, end));
                                    }
                                    last_idx = start + 1;
                                }
                            } else {
                                let query_lower = query.to_lowercase();
                                let string_lower = text.to_lowercase();
                                while let Some(idx) = string_lower[last_idx..].find(&query_lower) {
                                    let start = last_idx + idx;
                                    let end = start + query_lower.len();
                                    if text.is_char_boundary(start)
                                        && text.is_char_boundary(end)
                                        && text[start..end].to_lowercase() == query_lower
                                    {
                                        match_ranges.push((start, end));
                                    }
                                    last_idx = start + 1;
                                }
                            }

                            // Use galley_pos: the exact Pos2 origin egui uses to paint the galley
                            let galley_origin = output.galley_pos;
                            let painter = ui.painter();

                            for (byte_start, byte_end) in match_ranges {
                                let galley = &output.galley;

                                // Convert byte offsets -> char counts for egui's galley
                                // (egui normalizes \r\n to \n, so raw byte offsets drift)
                                let char_start = text[..byte_start].chars().count();
                                let char_end = text[..byte_end].chars().count();

                                // Lookup start geometry (galley-local)
                                let pcursor_start = galley
                                    .from_ccursor(egui::text::CCursor::new(char_start))
                                    .pcursor;
                                let local_start = galley.pos_from_pcursor(pcursor_start);

                                // Lookup end geometry (galley-local)
                                let pcursor_end = galley
                                    .from_ccursor(egui::text::CCursor::new(char_end))
                                    .pcursor;
                                let local_end = galley.pos_from_pcursor(pcursor_end);

                                // Translate to screen space
                                let bg_rect = egui::Rect::from_min_max(
                                    galley_origin + local_start.min.to_vec2(),
                                    galley_origin + egui::vec2(local_end.min.x, local_start.max.y),
                                );

                                // Active match: compare by char counts (tab.cursor_range stores char counts)
                                let is_active = active_range
                                    .map(|(p, s)| p == char_start && s == char_end)
                                    .unwrap_or(false);

                                let (fill, stroke_color) = if is_active {
                                    (
                                        egui::Color32::from_rgba_unmultiplied(255, 171, 64, 210),
                                        egui::Color32::from_rgba_unmultiplied(230, 120, 0, 240),
                                    )
                                } else {
                                    (
                                        egui::Color32::from_rgba_unmultiplied(255, 241, 118, 130),
                                        egui::Color32::from_rgba_unmultiplied(200, 180, 0, 160),
                                    )
                                };

                                painter.rect_filled(bg_rect, 3.0, fill);
                                painter.rect_stroke(
                                    bg_rect,
                                    3.0,
                                    egui::Stroke::new(1.0, stroke_color),
                                );
                            }
                        }

                        if force_scroll {
                            if let Some(r) = output.cursor_range {
                                let p = output.galley.pos_from_pcursor(r.primary.pcursor);
                                let rect = egui::Rect::from_min_max(
                                    res.rect.min + egui::vec2(margin, margin) + p.min.to_vec2(),
                                    res.rect.min + egui::vec2(margin, margin) + p.max.to_vec2(),
                                );
                                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                            }
                        }

                        if res.changed() {
                            content_changed = true;
                            tab.is_dirty = true;
                            tab_changed_idx = Some(idx);
                        }

                        // Update hover index based on mouse position
                        let mut hovered_idx = None;
                        if let Some(hover_pos) = res.hover_pos() {
                            if let Some(galley) = galley_to_draw.as_ref() {
                                let text_pos = res.rect.min + egui::vec2(margin, margin);
                                let relative_pos = hover_pos - text_pos;
                                let cursor = galley.cursor_from_pos(relative_pos);
                                let paragraph = cursor.pcursor.paragraph;
                                let offset = cursor.pcursor.offset;
                                let content = &tab.content;
                                let mut abs_idx = 0usize;
                                for (i, line) in content.split('\n').enumerate() {
                                    if i == paragraph {
                                        abs_idx += offset.min(line.len());
                                        break;
                                    }
                                    abs_idx += line.len() + 1;
                                }
                                hovered_idx = Some(abs_idx);
                            }
                        }

                        hovered_idx_out = hovered_idx;

                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), res.id) {
                            if let Some(range) = state.cursor.char_range() {
                                if !ui.input(|i| {
                                    i.pointer.secondary_down() || i.pointer.secondary_clicked()
                                }) {
                                    if res.has_focus() {
                                        tab.cursor_range =
                                            Some((range.primary.index, range.secondary.index));
                                    }
                                } else if let Some((p, s)) = tab.cursor_range {
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
                    });

                    // Line numbers rendering
                    if self.show_line_numbers {
                        if let (Some(res), Some(galley)) = (text_edit_res.as_ref(), galley_to_draw)
                        {
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
                                    let row_y = res.rect.min.y + margin + row.rect.min.y;
                                    let pos = egui::pos2(res.rect.min.x - 8.0, row_y);
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

                // Handle Context Menu (Outside horizontal layout to avoid distortion)
                if let Some(res) = text_edit_res.as_ref() {
                    let ed_ctx = notos_sdk::EditorContext {
                        content: &tab.content,
                        selection: tab.cursor_range,
                        hovered_char_idx: hovered_idx_out,
                    };
                    let can_undo = tab.can_undo();
                    let can_redo = tab.can_redo();

                    res.context_menu(|ui| {
                        ui.set_min_width(180.0);

                        // Plugin actions
                        let p_action = plugin_manager.context_menu_ui(ui, &ed_ctx);
                        if p_action != notos_sdk::PluginAction::None {
                            deferred_action = DeferredAction::Plugin(p_action);
                            ui.separator();
                        }

                        // Standard Edit actions
                        ui.add_enabled_ui(can_undo, |ui| {
                            if ui.button("↩ Undo").clicked() {
                                deferred_action = DeferredAction::Undo;
                                ui.close_menu();
                            }
                        });
                        ui.add_enabled_ui(can_redo, |ui| {
                            if ui.button("↪ Redo").clicked() {
                                deferred_action = DeferredAction::Redo;
                                ui.close_menu();
                            }
                        });

                        ui.separator();

                        if ui.button("✂ Cut").clicked() {
                            deferred_action = DeferredAction::Cut;
                            ui.close_menu();
                        }
                        if ui.button("📄 Copy").clicked() {
                            deferred_action = DeferredAction::Copy;
                            ui.close_menu();
                        }
                        if ui.button("📋 Paste").clicked() {
                            deferred_action = DeferredAction::Paste;
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("✅ Select All").clicked() {
                            deferred_action = DeferredAction::SelectAll;
                            ui.close_menu();
                        }
                    });
                }
            });

            // Execute deferred action
            match deferred_action {
                DeferredAction::None => {}
                DeferredAction::Plugin(p) => self.handle_plugin_action(p, ctx),
                DeferredAction::Undo => {
                    if let Some(tab) = self.active_tab_mut() {
                        tab.undo();
                    }
                }
                DeferredAction::Redo => {
                    if let Some(tab) = self.active_tab_mut() {
                        tab.redo();
                    }
                }
                DeferredAction::SelectAll => {
                    if let Some(tab) = self.active_tab_mut() {
                        let len = tab.content.len();
                        let id = egui::Id::new("editor").with(tab.id);
                        if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(len),
                                )));
                            egui::TextEdit::store_state(ctx, id, state);
                        }
                        tab.cursor_range = Some((0, len));
                    }
                }
                DeferredAction::Cut => {
                    if let Some(tab) = self.active_tab_mut() {
                        if let Some((s, e)) = tab.cursor_range {
                            let range = s.min(e)..s.max(e);
                            if let Some(text) = tab.content.get(range) {
                                ctx.output_mut(|o| o.copied_text = text.to_string());
                                self.handle_plugin_action(
                                    notos_sdk::PluginAction::ReplaceSelection("".to_string()),
                                    ctx,
                                );
                            }
                        }
                    }
                }
                DeferredAction::Copy => {
                    if let Some(tab) = self.active_tab_mut() {
                        if let Some((s, e)) = tab.cursor_range {
                            let range = s.min(e)..s.max(e);
                            if let Some(text) = tab.content.get(range) {
                                ctx.output_mut(|o| o.copied_text = text.to_string());
                            }
                        }
                    }
                }
                DeferredAction::Paste => {
                    // Still can't easily paste from context menu in egui without host/async
                }
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
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No open tabs. Press Ctrl+N to create a new one.");
            });
        }
        self.hovered_char_idx = hovered_idx_out;
    }
}
