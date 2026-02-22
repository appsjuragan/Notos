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

                                // Tabs (Left aligned, scrollable, filling remaining space)
                                ui.add_space(4.0);
                                for tab in tabs {
                                    let is_active = Some(tab.id) == active_tab_id;

                                    let tab_action = ui
                                        .scope(|ui| {
                                            let mut inner_action = None;
                                            let mut close_btn_rect: Option<egui::Rect> = None;

                                            let text_color = if is_active {
                                                ui.visuals().widgets.active.text_color()
                                            } else {
                                                ui.visuals().widgets.inactive.text_color()
                                            };

                                            let bg_fill = if is_active {
                                                egui::Color32::from_rgb(144, 192, 240)
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            };

                                            let tab_res = egui::Frame::canvas(ui.style())
                                                .fill(bg_fill)
                                                .stroke(
                                                    ui.visuals().widgets.noninteractive.bg_stroke,
                                                )
                                                .rounding(4.0)
                                                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;

                                                        let title = if tab.is_dirty {
                                                            format!("* {}", tab.title)
                                                        } else {
                                                            tab.title.clone()
                                                        };

                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(title)
                                                                    .color(text_color)
                                                                    .strong(),
                                                            )
                                                            .selectable(false),
                                                        )
                                                        .on_hover_cursor(egui::CursorIcon::Default);

                                                        let is_hovering_close = if let Some(pos) =
                                                            ui.input(|i| i.pointer.interact_pos())
                                                        {
                                                            if let Some(rect) = close_btn_rect {
                                                                // Use previous frame rect or a heuristic if None
                                                                rect.contains(pos)
                                                            } else {
                                                                false
                                                            }
                                                        } else {
                                                            false
                                                        };

                                                        let close_btn_color = if is_hovering_close {
                                                            if ui.visuals().dark_mode {
                                                                egui::Color32::from_rgb(
                                                                    255, 100, 100,
                                                                ) // Hover: bright red
                                                            } else {
                                                                egui::Color32::from_rgb(200, 0, 0)
                                                                // Hover: bright red
                                                            }
                                                        } else {
                                                            if ui.visuals().dark_mode {
                                                                egui::Color32::from_rgb(180, 50, 50)
                                                            // Dark mode: light maroon/muted red
                                                            } else {
                                                                egui::Color32::from_rgb(128, 0, 0)
                                                                // Light mode: Maroon
                                                            }
                                                        };

                                                        let close_btn = egui::Button::new(
                                                            egui::RichText::new("×")
                                                                .color(close_btn_color)
                                                                .strong(),
                                                        )
                                                        .small()
                                                        .frame(false);

                                                        let close_btn_response = ui.add(close_btn);

                                                        if is_hovering_close {
                                                            ui.ctx().request_repaint();
                                                        }

                                                        close_btn_rect =
                                                            Some(close_btn_response.rect);
                                                        if close_btn_response.clicked() {
                                                            inner_action =
                                                                Some(TabAction::Close(tab.id));
                                                        }
                                                    });
                                                });

                                            let response = ui
                                                .interact(
                                                    tab_res.response.rect,
                                                    ui.id().with(tab.id),
                                                    egui::Sense::click(),
                                                )
                                                .on_hover_cursor(egui::CursorIcon::Default);

                                            // Only select tab if click is not on close button
                                            if response.clicked() && inner_action.is_none() {
                                                // Check if click was on close button area
                                                let click_pos =
                                                    ui.input(|i| i.pointer.interact_pos());
                                                let is_on_close_btn =
                                                    if let (Some(pos), Some(btn_rect)) =
                                                        (click_pos, close_btn_rect)
                                                    {
                                                        btn_rect.contains(pos)
                                                    } else {
                                                        false
                                                    };

                                                if is_on_close_btn {
                                                    inner_action = Some(TabAction::Close(tab.id));
                                                } else {
                                                    inner_action = Some(TabAction::Select(tab.id));
                                                }
                                            }
                                            if response.middle_clicked() {
                                                inner_action = Some(TabAction::Close(tab.id));
                                            }

                                            response.context_menu(|ui| {
                                                if ui.button("Close").clicked() {
                                                    inner_action = Some(TabAction::Close(tab.id));
                                                    ui.close_menu();
                                                }
                                                if ui.button("Close Others").clicked() {
                                                    inner_action =
                                                        Some(TabAction::CloseOthers(tab.id));
                                                    ui.close_menu();
                                                }
                                            });

                                            inner_action
                                        })
                                        .inner;

                                    if let Some(a) = tab_action {
                                        action = Some(a);
                                    }

                                    ui.add_space(4.0);
                                }
                                action
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
