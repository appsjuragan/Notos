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
    SelectAll,
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
        ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
        ui.spacing_mut().interact_size.y = 20.0; // Fixed height for all menu buttons
        ui.menu_button("File", |ui| {
            if ui
                .add(egui::Button::new("📄 New Tab").shortcut_text("Ctrl+T"))
                .clicked()
            {
                action = Some(MenuAction::NewTab);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("📂 Open").shortcut_text("Ctrl+O"))
                .clicked()
            {
                action = Some(MenuAction::Open);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("💾 Save").shortcut_text("Ctrl+S"))
                .clicked()
            {
                action = Some(MenuAction::Save);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("💾 Save As").shortcut_text("Ctrl+Shift+S"))
                .clicked()
            {
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
            if ui
                .add(egui::Button::new("↩ Undo").shortcut_text("Ctrl+Z"))
                .clicked()
            {
                action = Some(MenuAction::Undo);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("↪ Redo").shortcut_text("Ctrl+Y"))
                .clicked()
            {
                action = Some(MenuAction::Redo);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add(egui::Button::new("🔍 Find").shortcut_text("Ctrl+F"))
                .clicked()
            {
                action = Some(MenuAction::Find);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("🔄 Replace").shortcut_text("Ctrl+H"))
                .clicked()
            {
                action = Some(MenuAction::Replace);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("🎯 Go To...").shortcut_text("Ctrl+G"))
                .clicked()
            {
                action = Some(MenuAction::GotoLine);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("📅 Time/Date  F5").clicked() {
                action = Some(MenuAction::TimeDate);
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("✅ Select All").shortcut_text("Ctrl+A"))
                .clicked()
            {
                action = Some(MenuAction::SelectAll);
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
                let is_mono = editor_font_family == "Monospace";
                let mono_text = if is_mono {
                    egui::RichText::new("⌨ Monospace").color(egui::Color32::WHITE)
                } else {
                    egui::RichText::new("⌨ Monospace")
                };
                if ui.selectable_label(is_mono, mono_text).clicked() {
                    action = Some(MenuAction::ChangeFont("Monospace".to_string()));
                    ui.close_menu();
                }

                let is_prop = editor_font_family == "Proportional";
                let prop_text = if is_prop {
                    egui::RichText::new("🎨 Proportional").color(egui::Color32::WHITE)
                } else {
                    egui::RichText::new("🎨 Proportional")
                };
                if ui.selectable_label(is_prop, prop_text).clicked() {
                    action = Some(MenuAction::ChangeFont("Proportional".to_string()));
                    ui.close_menu();
                }

                if !custom_fonts.is_empty() {
                    ui.separator();
                    for name in custom_fonts.keys() {
                        let is_active = editor_font_family == name;
                        let text = if is_active {
                            egui::RichText::new(format!("🔠 {}", name)).color(egui::Color32::WHITE)
                        } else {
                            egui::RichText::new(format!("🔠 {}", name))
                        };
                        if ui.selectable_label(is_active, text).clicked() {
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

        ui.menu_button("Plugins", |ui| {
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
