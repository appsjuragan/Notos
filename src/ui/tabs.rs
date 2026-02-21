use crate::editor::EditorTab;
use egui::Ui;

#[derive(Clone, Copy)]
pub enum TabAction {
    Select(crate::editor::TabId),
    Close(crate::editor::TabId),
    CloseOthers(crate::editor::TabId),
    New,
}

pub fn tab_bar(
    ui: &mut Ui,
    tabs: &[EditorTab],
    active_tab_id: Option<crate::editor::TabId>,
) -> Option<TabAction> {
    let action = ui
        .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut action = None;

            // New Tab Button (Right aligned)
            if ui
                .add(egui::Button::new("+").min_size(egui::vec2(32.0, 32.0)))
                .clicked()
            {
                action = Some(TabAction::New);
            }

            // Tabs (Left aligned, scrollable, filling remaining space)
            let available_size = ui.available_size();
            if available_size.x > 0.0 {
                ui.allocate_ui_with_layout(
                    available_size,
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let scroll_action = egui::ScrollArea::horizontal()
                            .id_salt("tabs_scroll")
                            .show(ui, |ui| {
                                ui.set_min_height(32.0);

                                // Add some padding at the start so the first tab isn't flush with the edge
                                ui.add_space(4.0);

                                let mut inner_action = None;
                                for tab in tabs {
                                    let is_active = Some(tab.id) == active_tab_id;

                                    let response = ui
                                        .scope(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 2.0;

                                                let title = if tab.is_dirty {
                                                    format!("* {}", tab.title)
                                                } else {
                                                    tab.title.clone()
                                                };

                                                let label_res =
                                                    ui.selectable_label(is_active, title);
                                                if label_res.clicked() {
                                                    inner_action = Some(TabAction::Select(tab.id));
                                                }

                                                if label_res.middle_clicked() {
                                                    inner_action = Some(TabAction::Close(tab.id));
                                                }

                                                label_res.context_menu(|ui| {
                                                    if ui.button("Close").clicked() {
                                                        inner_action =
                                                            Some(TabAction::Close(tab.id));
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Close Others").clicked() {
                                                        inner_action =
                                                            Some(TabAction::CloseOthers(tab.id));
                                                        ui.close_menu();
                                                    }
                                                });

                                                if ui
                                                    .add(
                                                        egui::Button::new("×").small().frame(false),
                                                    )
                                                    .clicked()
                                                {
                                                    inner_action = Some(TabAction::Close(tab.id));
                                                }
                                            });
                                        })
                                        .response;

                                    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
                                    ui.painter().rect_stroke(
                                        response.rect.expand(2.0),
                                        4.0,
                                        stroke,
                                    );

                                    if let Some(a) = inner_action {
                                        action = Some(a);
                                    }

                                    ui.add_space(12.0);
                                }
                                inner_action
                            })
                            .inner;

                        if action.is_none() {
                            action = scroll_action;
                        }
                    },
                );
            }
            action
        })
        .inner;

    action
}
