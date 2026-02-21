use crate::plugin::PluginManager;
use egui::Ui;
use notos_sdk::{EditorContext, PluginAction};

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
    OpenRecent(std::path::PathBuf),
    ClearHistory,
}

pub fn menu_bar(
    ui: &mut Ui,
    plugin_manager: &mut PluginManager,
    word_wrap: &mut bool,
    show_line_numbers: &mut bool,
    dark_mode: &mut bool,
    editor_font_family: &str,
    custom_fonts: &std::collections::HashMap<String, Vec<u8>>,
    recent_files: &[std::path::PathBuf],
    ed_ctx: &EditorContext,
) -> (Option<MenuAction>, PluginAction) {
    let mut action = None;
    let mut plugin_action = PluginAction::None;

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("📄 New Tab").clicked() {
                action = Some(MenuAction::NewTab);
                ui.close_menu();
            }
            if ui.button("📂 Open").clicked() {
                action = Some(MenuAction::Open);
                ui.close_menu();
            }
            if ui.button("💾 Save").clicked() {
                action = Some(MenuAction::Save);
                ui.close_menu();
            }
            if ui.button("💾 Save As").clicked() {
                action = Some(MenuAction::SaveAs);
                ui.close_menu();
            }

            ui.separator();
            ui.menu_button("🕒 Recent Files", |ui| {
                if recent_files.is_empty() {
                    ui.label("No recent files");
                } else {
                    for path in recent_files {
                        let label = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_string_lossy().to_string());
                        if ui
                            .button(format!("📄 {}", label))
                            .on_hover_text(path.to_string_lossy())
                            .clicked()
                        {
                            action = Some(MenuAction::OpenRecent(path.clone()));
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("🗑 Clear History").clicked() {
                        action = Some(MenuAction::ClearHistory);
                        ui.close_menu();
                    }
                }
            });

            ui.separator();
            if ui.button("🚪 Exit").clicked() {
                action = Some(MenuAction::Exit);
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui.button("↩ Undo").clicked() {
                action = Some(MenuAction::Undo);
                ui.close_menu();
            }
            if ui.button("↪ Redo").clicked() {
                action = Some(MenuAction::Redo);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("🔍 Find").clicked() {
                action = Some(MenuAction::Find);
                ui.close_menu();
            }
            if ui.button("🔄 Replace").clicked() {
                action = Some(MenuAction::Replace);
                ui.close_menu();
            }
            if ui.button("🎯 Go To...").clicked() {
                action = Some(MenuAction::GotoLine);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("📅 Time/Date  F5").clicked() {
                action = Some(MenuAction::TimeDate);
                ui.close_menu();
            }
        });

        ui.menu_button("View", |ui| {
            if ui.checkbox(word_wrap, "Wrap Word").clicked() {
                action = Some(MenuAction::ToggleWordWrap);
                ui.close_menu();
            }
            if ui
                .checkbox(show_line_numbers, "🔢 Show Line Number")
                .clicked()
            {
                action = Some(MenuAction::ToggleLineNumbers);
                ui.close_menu();
            }
            if ui.checkbox(dark_mode, "🌙 Dark Mode").clicked() {
                action = Some(MenuAction::ToggleDarkMode);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("➕ Zoom In").clicked() {
                action = Some(MenuAction::ZoomIn);
            }
            if ui.button("➖ Zoom Out").clicked() {
                action = Some(MenuAction::ZoomOut);
            }
            if ui.button("🔄 Reset Zoom").clicked() {
                action = Some(MenuAction::ResetZoom);
            }
            ui.separator();
            ui.menu_button("🔠 Change Font", |ui| {
                if ui
                    .selectable_label(editor_font_family == "Monospace", "⌨ Monospace")
                    .clicked()
                {
                    action = Some(MenuAction::ChangeFont("Monospace".to_string()));
                    ui.close_menu();
                }
                if ui
                    .selectable_label(editor_font_family == "Proportional", "🎨 Proportional")
                    .clicked()
                {
                    action = Some(MenuAction::ChangeFont("Proportional".to_string()));
                    ui.close_menu();
                }

                if !custom_fonts.is_empty() {
                    ui.separator();
                    for name in custom_fonts.keys() {
                        if ui
                            .selectable_label(editor_font_family == name, format!("🔠 {}", name))
                            .clicked()
                        {
                            action = Some(MenuAction::ChangeFont(name.clone()));
                            ui.close_menu();
                        }
                    }
                }

                ui.separator();
                if ui.button("📥 Load Font File...").clicked() {
                    action = Some(MenuAction::LoadFont);
                    ui.close_menu();
                }
            });
        });

        ui.menu_button("🔌 Plugins", |ui| {
            let p_action = plugin_manager.plugins_menu_ui(ui, ed_ctx);
            if p_action != PluginAction::None {
                plugin_action = p_action;
            }
        });

        // Other plugin menu extensions (e.g., custom menus like Help)
        let other_action = plugin_manager.menu_ui(ui, ed_ctx);
        if plugin_action == PluginAction::None {
            plugin_action = other_action;
        }
    });
    (action, plugin_action)
}
