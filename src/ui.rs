use egui::Ui;
use crate::editor::EditorTab;

pub fn tab_bar(ui: &mut Ui, tabs: &mut Vec<EditorTab>, active_tab_id: &mut Option<uuid::Uuid>) {
    #[derive(Clone, Copy)]
    enum TabAction {
        Close(usize),
        CloseOthers(usize),
        New,
    }

    let action = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let mut action = None;

        // New Tab Button (Right aligned)
        if ui.add(egui::Button::new("+").min_size(egui::vec2(32.0, 32.0))).clicked() {
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

                            let response = ui.scope(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    
                                    let title = if tab.is_dirty {
                                        format!("* {}", tab.title)
                                    } else {
                                        tab.title.clone()
                                    };

                                    // Use selectable_label for the title - it handles hover/active colors perfectly
                                    let label_res = ui.selectable_label(is_active, title);
                                    if label_res.clicked() {
                                        *active_tab_id = Some(tab.id);
                                    }
                                    
                                    if label_res.middle_clicked() {
                                        inner_action = Some(TabAction::Close(index));
                                    }

                                    label_res.context_menu(|ui| {
                                        if ui.button("Close").clicked() {
                                            inner_action = Some(TabAction::Close(index));
                                            ui.close_menu();
                                        }
                                        if ui.button("Close Others").clicked() {
                                            inner_action = Some(TabAction::CloseOthers(index));
                                            ui.close_menu();
                                        }
                                    });

                                    if ui.small_button("x").clicked() {
                                        inner_action = Some(TabAction::Close(index));
                                    }
                                });
                            }).response;

                            // Draw a subtle border around the tab
                            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
                            ui.painter().rect_stroke(response.rect.expand(4.0), 4.0, stroke);

                            if let Some(a) = inner_action {
                                action = Some(a);
                            }
                            
                            ui.add_space(12.0);
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
