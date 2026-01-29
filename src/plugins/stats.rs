use crate::plugin::NotosPlugin;
use egui::{Context, Window};

pub struct StatsPlugin {
    open: bool,
}

impl Default for StatsPlugin {
    fn default() -> Self {
        Self { open: true }
    }
}

impl NotosPlugin for StatsPlugin {
    fn id(&self) -> &str {
        "stats"
    }

    fn name(&self) -> &str {
        "Statistics"
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Statistics").clicked() {
            self.open = !self.open;
            ui.close_menu();
        }
    }

    fn ui(&mut self, ctx: &Context) {
        if self.open {
            Window::new("Statistics")
                .open(&mut self.open)
                .show(ctx, |ui| {
                    ui.label("This is a plugin window.");
                    ui.label("It can access the context and show info.");
                    // In a real plugin, we would access the app state via a shared context
                    // or message passing. For now, just a demo.
                });
        }
    }
}
