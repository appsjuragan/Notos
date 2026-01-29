use std::any::Any;
use egui::Context;

/// Context passed to plugins when they are initialized or executed.
#[allow(dead_code)]
pub struct PluginContext<'a> {
    pub ctx: &'a Context,
    // We can add more access to the app state here later (e.g., current document)
}

/// The trait that all plugins must implement.
pub trait NotosPlugin: Any + Send + Sync {
    /// Unique identifier for the plugin.
    fn id(&self) -> &str;

    /// Display name of the plugin.
    fn name(&self) -> &str;

    /// Called when the plugin is loaded.
    fn on_load(&mut self, _ctx: &Context) {}

    /// Called every frame. Use this to show windows or panels.
    fn ui(&mut self, _ctx: &egui::Context) {}

    /// Called to extend the main menu.
    fn menu_ui(&mut self, _ui: &mut egui::Ui) {}
    
    /// Called when the application is shutting down.
    fn on_unload(&mut self) {}
}

/// Manages the lifecycle of plugins.
pub struct PluginManager {
    plugins: Vec<Box<dyn NotosPlugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn NotosPlugin>) {
        log::info!("Registering plugin: {} (ID: {})", plugin.name(), plugin.id());
        self.plugins.push(plugin);
    }

    pub fn on_load(&mut self, ctx: &Context) {
        for plugin in &mut self.plugins {
            plugin.on_load(ctx);
        }
    }

    pub fn on_unload(&mut self) {
        for plugin in &mut self.plugins {
            plugin.on_unload();
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        for plugin in &mut self.plugins {
            plugin.ui(ctx);
        }
    }

    pub fn menu_ui(&mut self, ui: &mut egui::Ui) {
        for plugin in &mut self.plugins {
            plugin.menu_ui(ui);
        }
    }
}
