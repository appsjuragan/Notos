use crate::plugin::NotosPlugin;
use egui::{Context, Window};

pub struct StatsPlugin {
    open: bool,
}

impl Default for StatsPlugin {
    fn default() -> Self {
        Self { open: false }
    }
}

impl NotosPlugin for StatsPlugin {
    fn id(&self) -> &str {
        "about"
    }

    fn name(&self) -> &str {
        "About"
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("About").clicked() {
            self.open = !self.open;
            ui.close_menu();
        }
    }

    fn ui(&mut self, ctx: &Context) {
        if self.open {
            Window::new("About Notos")
                .open(&mut self.open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Notos Text Editor");
                    ui.label(format!("Version: {}\n Yet another text editor to compete with Windows Notepad.", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("GitHub:");
                        ui.hyperlink("https://github.com/appsjuragan");
                    });
                });
        }
    }
}
