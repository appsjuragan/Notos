use egui::Context;
use std::any::Any;

/// Actions that a plugin can request the main application to perform.
#[derive(Debug, PartialEq, Eq)]
pub enum PluginAction {
    /// Do nothing.
    None,
    /// Replace the entire content of the active tab.
    ReplaceAll(String),
    /// Replace the currently selected text in the active tab.
    ReplaceSelection(String),
}

/// Information about the current editor state passed to plugins.
pub struct EditorContext<'a> {
    pub content: &'a str,
    pub selection: Option<(usize, usize)>,
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
    fn ui(&mut self, _ctx: &egui::Context, _ed: &EditorContext) -> PluginAction {
        PluginAction::None
    }

    /// Called to extend the main menu.
    fn menu_ui(&mut self, _ui: &mut egui::Ui, _ed: &EditorContext) -> PluginAction {
        PluginAction::None
    }

    /// Called when the application is shutting down.
    fn on_unload(&mut self) {}
}

/// Type of the function that plugins must export to be loaded.
/// Returns a pointer to a Box<dyn NotosPlugin> (a thin pointer to a fat pointer).
pub type CreatePluginFn = unsafe extern "C" fn() -> *mut std::ffi::c_void;

/// Type of the function that plugins must export to be destroyed.
/// Takes the pointer returned by CreatePluginFn.
pub type DestroyPluginFn = unsafe extern "C" fn(*mut std::ffi::c_void);
