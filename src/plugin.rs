use egui::Context;
use notos_sdk::{CreatePluginFn, DestroyPluginFn, EditorContext, NotosPlugin, PluginAction};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// A loaded plugin instance.
struct PluginInstance {
    // This is a Box<Box<dyn NotosPlugin>>
    raw_wrapper: *mut std::ffi::c_void,
    destroyer: DestroyPluginFn,
}

impl PluginInstance {
    /// Access the plugin trait object safely.
    unsafe fn as_plugin_mut(&mut self) -> &mut dyn NotosPlugin {
        let box_ptr = self.raw_wrapper as *mut Box<dyn NotosPlugin>;
        &mut **box_ptr
    }
}

impl Drop for PluginInstance {
    fn drop(&mut self) {
        log::debug!("Destroying plugin instance");
        unsafe {
            // Memory allocated in DLL must be freed in DLL
            (self.destroyer)(self.raw_wrapper);
        }
    }
}

/// Manages the lifecycle of plugins.
pub struct PluginManager {
    plugins: Vec<PluginInstance>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Load all plugins from the "plugins" directory relative to executable.
    pub fn load_plugins(&mut self) {
        log::info!("Scanning for plugins...");

        let mut loaded_filenames = HashSet::new();

        // Find plugins directory
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let plugins_dir = exe_dir.join("plugins");

        if !plugins_dir.exists() {
            log::info!("Plugins folder not found at {:?}", plugins_dir);
            return;
        }

        if let Ok(entries) = fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "dll" || ext_str == "so" || ext_str == "dylib" {
                        if let Some(filename) = path.file_name() {
                            let filename_str = filename.to_string_lossy().to_string();
                            if loaded_filenames.insert(filename_str) {
                                unsafe {
                                    self.load_plugin_from_file(&path);
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!("Loaded {} plugins.", self.plugins.len());
    }

    unsafe fn load_plugin_from_file(&mut self, path: &PathBuf) {
        log::info!("Loading plugin DLL: {:?}", path);

        match libloading::Library::new(path) {
            Ok(lib) => {
                let create_func: libloading::Symbol<CreatePluginFn> =
                    match lib.get(b"_create_plugin") {
                        Ok(f) => f,
                        Err(e) => {
                            log::warn!("Missing _create_plugin in {:?}: {}", path, e);
                            return;
                        }
                    };

                let destroy_func: libloading::Symbol<DestroyPluginFn> =
                    match lib.get(b"_destroy_plugin") {
                        Ok(f) => f,
                        Err(e) => {
                            log::warn!("Missing _destroy_plugin in {:?}: {}", path, e);
                            return;
                        }
                    };

                let destroyer = *destroy_func;
                let raw_wrapper = create_func();

                self.plugins.push(PluginInstance {
                    raw_wrapper,
                    destroyer,
                });

                // LEAK the library handle.
                std::mem::forget(lib);

                log::info!("Plugin successfully loaded and locked in memory.");
            }
            Err(e) => {
                log::error!("Failed to load library {:?}: {}", path, e);
            }
        }
    }

    pub fn on_load(&mut self, ctx: &Context) {
        for p in &mut self.plugins {
            unsafe {
                p.as_plugin_mut().on_load(ctx);
            }
        }
    }

    pub fn on_unload(&mut self) {
        for p in &mut self.plugins {
            unsafe {
                p.as_plugin_mut().on_unload();
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, ed: &EditorContext) -> PluginAction {
        let mut result = PluginAction::None;
        for p in &mut self.plugins {
            unsafe {
                let action = p.as_plugin_mut().ui(ctx, ed);
                if action != PluginAction::None {
                    result = action;
                }
            }
        }
        result
    }

    pub fn menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        let mut result = PluginAction::None;
        for p in &mut self.plugins {
            unsafe {
                let action = p.as_plugin_mut().menu_ui(ui, ed);
                if action != PluginAction::None {
                    result = action;
                }
            }
        }
        result
    }
}
