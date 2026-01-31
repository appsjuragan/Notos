use crate::editor::EditorTab;
use crate::plugin::PluginManager;
use egui::Ui;
use notos_sdk::{EditorContext, PluginAction};

#[derive(Clone, Copy)]
pub enum TabAction {
    Select(uuid::Uuid),
    Close(uuid::Uuid),
    CloseOthers(uuid::Uuid),
    New,
}

pub enum MenuAction {
    NewTab,
    Open,
    Save,
    SaveAs,
    Exit,
    Undo,
    Redo,
    Find,
    Replace,
    GotoLine,
    TimeDate,
    ToggleWordWrap,
    ToggleLineNumbers,
    ToggleDarkMode,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ChangeFont(String),
    LoadFont,
}

pub fn menu_bar(
    ui: &mut Ui,
    plugin_manager: &mut PluginManager,
    word_wrap: &mut bool,
    show_line_numbers: &mut bool,
    dark_mode: &mut bool,
    editor_font_family: &str,
    custom_fonts: &std::collections::HashMap<String, Vec<u8>>,
    ed_ctx: &EditorContext,
) -> (Option<MenuAction>, PluginAction) {
    let mut action = None;
    let mut plugin_action = PluginAction::None;

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New Tab").clicked() {
                action = Some(MenuAction::NewTab);
                ui.close_menu();
            }
            if ui.button("Open").clicked() {
                action = Some(MenuAction::Open);
                ui.close_menu();
            }
            if ui.button("Save").clicked() {
                action = Some(MenuAction::Save);
                ui.close_menu();
            }
            if ui.button("Save As").clicked() {
                action = Some(MenuAction::SaveAs);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                action = Some(MenuAction::Exit);
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui.button("Undo").clicked() {
                action = Some(MenuAction::Undo);
                ui.close_menu();
            }
            if ui.button("Redo").clicked() {
                action = Some(MenuAction::Redo);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Find").clicked() {
                action = Some(MenuAction::Find);
                ui.close_menu();
            }
            if ui.button("Replace").clicked() {
                action = Some(MenuAction::Replace);
                ui.close_menu();
            }
            if ui.button("Go To...").clicked() {
                action = Some(MenuAction::GotoLine);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Time/Date  F5").clicked() {
                action = Some(MenuAction::TimeDate);
                ui.close_menu();
            }
        });

        ui.menu_button("View", |ui| {
            if ui.checkbox(word_wrap, "Word Wrap").clicked() {
                action = Some(MenuAction::ToggleWordWrap);
                ui.close_menu();
            }
            if ui.checkbox(show_line_numbers, "Show Line Number").clicked() {
                action = Some(MenuAction::ToggleLineNumbers);
                ui.close_menu();
            }
            if ui.checkbox(dark_mode, "Dark Mode").clicked() {
                action = Some(MenuAction::ToggleDarkMode);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Zoom In").clicked() {
                action = Some(MenuAction::ZoomIn);
            }
            if ui.button("Zoom Out").clicked() {
                action = Some(MenuAction::ZoomOut);
            }
            if ui.button("Reset Zoom").clicked() {
                action = Some(MenuAction::ResetZoom);
            }
            ui.separator();
            ui.menu_button("Change Font", |ui| {
                if ui
                    .selectable_label(editor_font_family == "Monospace", "Monospace")
                    .clicked()
                {
                    action = Some(MenuAction::ChangeFont("Monospace".to_string()));
                    ui.close_menu();
                }
                if ui
                    .selectable_label(editor_font_family == "Proportional", "Proportional")
                    .clicked()
                {
                    action = Some(MenuAction::ChangeFont("Proportional".to_string()));
                    ui.close_menu();
                }

                if !custom_fonts.is_empty() {
                    ui.separator();
                    for name in custom_fonts.keys() {
                        if ui
                            .selectable_label(editor_font_family == name, name)
                            .clicked()
                        {
                            action = Some(MenuAction::ChangeFont(name.clone()));
                            ui.close_menu();
                        }
                    }
                }

                ui.separator();
                if ui.button("Load Font File...").clicked() {
                    action = Some(MenuAction::LoadFont);
                    ui.close_menu();
                }
            });
        });

        // Plugin Menus
        plugin_action = plugin_manager.menu_ui(ui, ed_ctx);
    });
    (action, plugin_action)
}

pub fn tab_bar(
    ui: &mut Ui,
    tabs: &[EditorTab],
    active_tab_id: Option<uuid::Uuid>,
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
                                                ui.spacing_mut().item_spacing.x = 4.0;

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

                                                if ui.small_button("x").clicked() {
                                                    inner_action = Some(TabAction::Close(tab.id));
                                                }
                                            });
                                        })
                                        .response;

                                    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
                                    ui.painter().rect_stroke(
                                        response.rect.expand(4.0),
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

pub enum StatusBarAction {
    SwitchTab(uuid::Uuid),
    CloseTab(uuid::Uuid),
    SetEncoding(uuid::Uuid, &'static encoding_rs::Encoding),
    SetLineEnding(uuid::Uuid, crate::editor::LineEnding),
}

pub fn status_bar(
    ui: &mut Ui,
    tabs: &[EditorTab],
    active_tab_id: Option<uuid::Uuid>,
    cursor_pos: (usize, usize),
    zoom_level: f32,
) -> Option<StatusBarAction> {
    let mut action = None;
    ui.horizontal(|ui| {
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
                            action = Some(StatusBarAction::SetEncoding(tab.id, encoding_rs::UTF_8));
                            ui.close_menu();
                        }
                        if ui.button("Windows-1252 (ANSI)").clicked() {
                            action = Some(StatusBarAction::SetEncoding(
                                tab.id,
                                encoding_rs::WINDOWS_1252,
                            ));
                            ui.close_menu();
                        }
                        if ui.button("UTF-16LE").clicked() {
                            action =
                                Some(StatusBarAction::SetEncoding(tab.id, encoding_rs::UTF_16LE));
                            ui.close_menu();
                        }
                        if ui.button("UTF-16BE").clicked() {
                            action =
                                Some(StatusBarAction::SetEncoding(tab.id, encoding_rs::UTF_16BE));
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
