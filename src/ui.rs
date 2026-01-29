use egui::{Ui, RichText};
use crate::editor::EditorTab;

pub fn tab_bar(ui: &mut Ui, tabs: &mut Vec<EditorTab>, active_tab_id: &mut Option<uuid::Uuid>) {
    enum TabAction {
        Close(usize),
        CloseOthers(usize),
        New,
    }

    let action = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let mut action = None;

        // New Tab Button (Right aligned)
        if ui.button("+").clicked() {
            action = Some(TabAction::New);
        }

        // Tabs (Left aligned, scrollable)
        let available_width = ui.available_width();
        if available_width > 0.0 {
            let scroll_action = egui::ScrollArea::horizontal()
                .id_salt("tabs_scroll")
                .max_width(available_width)
                .show(ui, |ui| {
                    ui.set_min_height(32.0);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        let mut inner_action = None;
                        for (index, tab) in tabs.iter().enumerate() {
                            let is_active = Some(tab.id) == get_active_tab_id(active_tab_id);
                            let is_dark = ui.visuals().dark_mode;

                            // Define colors based on mode
                            let (active_bg, active_text) = if is_dark {
                                (egui::Color32::from_rgb(45, 45, 45), egui::Color32::WHITE)
                            } else {
                                (egui::Color32::WHITE, egui::Color32::BLACK)
                            };

                            let (hover_bg, hover_text) = if is_dark {
                                (egui::Color32::from_rgb(58, 58, 58), egui::Color32::WHITE)
                            } else {
                                (egui::Color32::from_rgb(229, 229, 229), egui::Color32::BLACK)
                            };

                            let (inactive_bg, inactive_text) = if is_dark {
                                (egui::Color32::TRANSPARENT, egui::Color32::from_gray(170))
                            } else {
                                (egui::Color32::TRANSPARENT, egui::Color32::from_gray(80))
                            };

                            let inner_response = egui::Frame::default()
                                .stroke(egui::Stroke::NONE)
                                .fill(egui::Color32::TRANSPARENT)
                                .rounding(4.0)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        
                                        let title = if tab.is_dirty {
                                            format!("* {}", tab.title)
                                        } else {
                                            tab.title.clone()
                                        };

                                        // We don't know if it's hovered yet for the text color, 
                                        // so we use a heuristic or just default to inactive/active.
                                        // To fix text color on hover, we might need to repaint or use a widget that handles it.
                                        // For now, let's use the active/inactive color for text, 
                                        // as hover usually doesn't change text color drastically in tabs.
                                        let text_color = if is_active { active_text } else { inactive_text };

                                        let text = RichText::new(title).color(text_color);
                                        let text = if is_active { text.strong() } else { text };

                                        // Label handles click for switching tabs
                                        let label_response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                                        
                                        if label_response.clicked() {
                                            *active_tab_id = Some(tab.id);
                                        }
                                        
                                        if label_response.middle_clicked() {
                                            inner_action = Some(TabAction::Close(index));
                                        }

                                        label_response.context_menu(|ui| {
                                            if ui.button("Close").clicked() {
                                                inner_action = Some(TabAction::Close(index));
                                                ui.close_menu();
                                            }
                                            if ui.button("Close Others").clicked() {
                                                inner_action = Some(TabAction::CloseOthers(index));
                                                ui.close_menu();
                                            }
                                        });

                                        if ui.add(egui::Button::new("x").frame(false).min_size(egui::vec2(0.0, 0.0))).clicked() {
                                            inner_action = Some(TabAction::Close(index));
                                        }
                                    })
                                });

                            let rect = inner_response.response.rect;
                            let hovered = ui.rect_contains_pointer(rect);
                            
                            // Handle click on the frame padding
                            let interact = ui.interact(rect, inner_response.response.id, egui::Sense::click());
                            if interact.clicked() {
                                 if !is_active {
                                     *active_tab_id = Some(tab.id);
                                 }
                            }

                            // Paint background and border
                            let bg_color = if is_active {
                                active_bg
                            } else if hovered {
                                hover_bg
                            } else {
                                inactive_bg
                            };
                            
                            let stroke_color = if is_dark {
                                egui::Color32::from_gray(80)
                            } else {
                                egui::Color32::from_gray(200)
                            };
                            let stroke = egui::Stroke::new(1.0, stroke_color);

                            ui.ctx().layer_painter(egui::LayerId::background()).rect(rect, 4.0, bg_color, stroke);
                            
                            ui.add_space(4.0);
                        }    ui.add_space(4.0);
                        }
                        inner_action
                    }).inner
                }).inner;
            
            if action.is_none() {
                action = scroll_action;
            }
        }
        action
    }).inner;

    match action {
        Some(TabAction::New) => {
            let new_tab = EditorTab::default();
            *active_tab_id = Some(new_tab.id);
            tabs.push(new_tab);
        }
        Some(TabAction::Close(index)) => {
            if index < tabs.len() {
                let removed = tabs.remove(index);
                if Some(removed.id) == *active_tab_id {
                    *active_tab_id = tabs.last().map(|t| t.id);
                }
            }
        }
        Some(TabAction::CloseOthers(index)) => {
            if index < tabs.len() {
                let keep_id = tabs[index].id;
                tabs.retain(|t| t.id == keep_id);
                *active_tab_id = Some(keep_id);
            }
        }
        None => {}
    }
}

fn get_active_tab_id(id: &Option<uuid::Uuid>) -> Option<uuid::Uuid> {
    *id
}
