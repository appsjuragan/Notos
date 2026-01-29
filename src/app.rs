use eframe::egui;
use crate::editor::EditorTab;
use crate::plugin::{PluginManager, NotosPlugin};
use crate::ui;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct NotosApp {
    tabs: Vec<EditorTab>,
    active_tab_id: Option<uuid::Uuid>,
    plugin_manager: PluginManager,
    // Settings, etc.
}

impl NotosApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize fonts/style here to match Notepad
        setup_custom_fonts(&cc.egui_ctx);
        setup_custom_style(&cc.egui_ctx);

        let mut app = Self {
            tabs: vec![EditorTab::default()],
            active_tab_id: None, // Will be set in init
            plugin_manager: PluginManager::new(),
        };
        
        if let Some(first) = app.tabs.first() {
            app.active_tab_id = Some(first.id);
        }

        // Load plugins here (we can add a default one for demonstration)
        // app.plugin_manager.register(Box::new(MyPlugin::default()));
        
        app.plugin_manager.on_load(&cc.egui_ctx);

        app
    }

    fn active_tab_mut(&mut self) -> Option<&mut EditorTab> {
        self.tabs.iter_mut().find(|t| Some(t.id) == self.active_tab_id)
    }

    fn open_file(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            match EditorTab::from_file(path) {
                Ok(tab) => {
                    self.active_tab_id = Some(tab.id);
                    self.tabs.push(tab);
                }
                Err(e) => {
                    log::error!("Failed to open file: {}", e);
                }
            }
        }
    }

    fn save_file(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            if tab.path.is_some() {
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                }
            } else {
                self.save_file_as();
            }
        }
    }

    fn save_file_as(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            if let Some(path) = FileDialog::new().save_file() {
                tab.path = Some(path);
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                }
            }
        }
    }
}

impl eframe::App for NotosApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Panel: Menu and Tabs
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Tab").clicked() {
                        let tab = EditorTab::default();
                        self.active_tab_id = Some(tab.id);
                        self.tabs.push(tab);
                        ui.close_menu();
                    }
                    if ui.button("Open").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As").clicked() {
                        self.save_file_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() { /* TODO */ }
                    if ui.button("Redo").clicked() { /* TODO */ }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Zoom In").clicked() { 
                        let zoom = ctx.zoom_factor();
                        ctx.set_zoom_factor(zoom + 0.1);
                    }
                    if ui.button("Zoom Out").clicked() { 
                         let zoom = ctx.zoom_factor();
                        ctx.set_zoom_factor((zoom - 0.1).max(0.2));
                    }
                    if ui.button("Reset Zoom").clicked() { 
                        ctx.set_zoom_factor(1.0);
                    }
                });
                
                // Plugin Menus
                self.plugin_manager.menu_ui(ui);
            });
            
            ui.add_space(4.0);
            ui::tab_bar(ui, &mut self.tabs, &mut self.active_tab_id);
        });

        // Bottom Panel: Status Bar
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(tab) = self.tabs.iter().find(|t| Some(t.id) == self.active_tab_id) {
                    let chars = tab.content.chars().count();
                    ui.label(format!("Length: {} chars", chars));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("UTF-8");
                        ui.label("Windows (CRLF)");
                        ui.label("100%");
                    });
                } else {
                     ui.label("Ready");
                }
            });
        });

        // Plugin UI (e.g. side panels, windows)
        // We call this before CentralPanel so plugins can add SidePanels that shrink the central area.
        self.plugin_manager.ui(ctx);

        // Central Panel: Editor
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tab) = self.active_tab_mut() {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let available_height = ui.available_height();
                    let available_width = ui.available_width();
                    ui.add_sized(
                        [available_width, available_height],
                        egui::TextEdit::multiline(&mut tab.content)
                            .frame(false) // Notepad-like look
                            .code_editor()
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                    );
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No open tabs. Press Ctrl+N to create a new one.");
                });
            }
        });
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // We could load "Consolas" or "Cascadia Code" here if we bundled them,
    // but for now we rely on default monospace.
    // In a real app, we'd load system fonts.
    
    // Example of configuring font families
    fonts.families.entry(egui::FontFamily::Monospace).or_default()
        .insert(0, "Hack".to_owned()); // If Hack was loaded
    
    ctx.set_fonts(fonts);
}

fn setup_custom_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    // Make it look clean and flat like Notepad
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::WHITE;
    style.visuals.window_fill = egui::Color32::WHITE;
    style.visuals.panel_fill = egui::Color32::WHITE;
    
    // Selection color
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    
    ctx.set_style(style);
}
