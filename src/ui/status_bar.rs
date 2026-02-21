use crate::editor::EditorTab;
use egui::Ui;

pub enum StatusBarAction {
    SwitchTab(crate::editor::TabId),
    CloseTab(crate::editor::TabId),
    SetLineEnding(crate::editor::TabId, crate::editor::LineEnding),
    SetEncoding(crate::editor::TabId, crate::editor::Encoding),
}

pub fn status_bar(
    ui: &mut Ui,
    tabs: &[EditorTab],
    active_tab_id: Option<crate::editor::TabId>,
    cursor_pos: (usize, usize),
    zoom_level: f32,
) -> Option<StatusBarAction> {
    let mut action = None;
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        let active_tab_index = tabs.iter().position(|t| Some(t.id) == active_tab_id);

        if let Some(index) = active_tab_index {
            let (chars, line, col) = {
                let tab = &tabs[index];
                (tab.content.chars().count(), cursor_pos.0, cursor_pos.1)
            };

            // Left side items
            ui.label(format!("Ln {}, Col {}", line, col));
            ui.separator();
            ui.label(format!("{} characters", chars));
            ui.separator();

            ui.menu_button(format!("Tabs: {}", tabs.len()), |ui| {
                ui.set_width(220.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

                for t in tabs {
                    ui.horizontal(|ui| {
                        let is_active = Some(t.id) == active_tab_id;

                        let label_width = ui.available_width() - 30.0;
                        ui.allocate_ui(egui::vec2(label_width, ui.available_height()), |ui| {
                            if ui.selectable_label(is_active, &t.title).clicked() {
                                action = Some(StatusBarAction::SwitchTab(t.id));
                                ui.close_menu();
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("x").clicked() {
                                action = Some(StatusBarAction::CloseTab(t.id));
                            }
                        });
                    });
                }
            });

            // Right side items
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(tab) = tabs.get(index) {
                    ui.menu_button(tab.encoding.name(), |ui| {
                        if ui.button("UTF-8").clicked() {
                            action = Some(StatusBarAction::SetEncoding(
                                tab.id,
                                crate::editor::Encoding::Utf8,
                            ));
                            ui.close_menu();
                        }
                        if ui.button("Windows-1252").clicked() {
                            action = Some(StatusBarAction::SetEncoding(
                                tab.id,
                                crate::editor::Encoding::Windows1252,
                            ));
                            ui.close_menu();
                        }
                        if ui.button("UTF-16LE").clicked() {
                            action = Some(StatusBarAction::SetEncoding(
                                tab.id,
                                crate::editor::Encoding::Utf16Le,
                            ));
                            ui.close_menu();
                        }
                        if ui.button("UTF-16BE").clicked() {
                            action = Some(StatusBarAction::SetEncoding(
                                tab.id,
                                crate::editor::Encoding::Utf16Be,
                            ));
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    ui.menu_button(tab.line_ending.name(), |ui| {
                        if ui.button("Windows (CRLF)").clicked() {
                            action = Some(StatusBarAction::SetLineEnding(
                                tab.id,
                                crate::editor::LineEnding::Crlf,
                            ));
                            ui.close_menu();
                        }
                        if ui.button("Unix (LF)").clicked() {
                            action = Some(StatusBarAction::SetLineEnding(
                                tab.id,
                                crate::editor::LineEnding::Lf,
                            ));
                            ui.close_menu();
                        }
                        if ui.button("Mac (CR)").clicked() {
                            action = Some(StatusBarAction::SetLineEnding(
                                tab.id,
                                crate::editor::LineEnding::Cr,
                            ));
                            ui.close_menu();
                        }
                    });

                    ui.separator();
                    ui.label(format!("{:.0}%", (zoom_level / 14.0) * 100.0));
                }
            });
        }
    });
    action
}
