use notos_sdk::{EditorContext, NotosPlugin, PluginAction};

struct JsonFormatPlugin;

impl JsonFormatPlugin {
    fn new() -> Self {
        Self
    }

    fn format_json(&self, text: &str) -> Option<String> {
        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(value) => {
                if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                    return Some(pretty);
                }
            }
            Err(e) => {
                log::warn!("Invalid JSON: {}", e);
            }
        }
        None
    }
}

impl NotosPlugin for JsonFormatPlugin {
    fn id(&self) -> &str {
        "notos_json_format"
    }

    fn name(&self) -> &str {
        "JSON Formatter"
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        let mut action = PluginAction::None;

        ui.menu_button("Plugins", |ui| {
            if ui.button("Format JSON").clicked() {
                if let Some((start, end)) = ed.selection {
                    if start != end {
                        // Format selection
                        if let Some(selected_text) = ed.content.get(start..end) {
                            if let Some(formatted) = self.format_json(selected_text) {
                                action = PluginAction::ReplaceSelection(formatted);
                            }
                        }
                    } else {
                        // Format entire file
                        if let Some(formatted) = self.format_json(ed.content) {
                            action = PluginAction::ReplaceAll(formatted);
                        }
                    }
                } else {
                    // Format entire file
                    if let Some(formatted) = self.format_json(ed.content) {
                        action = PluginAction::ReplaceAll(formatted);
                    }
                }
                ui.close_menu();
            }
        });

        action
    }
}

/// Dynamic library entry point for creation
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(JsonFormatPlugin::new());
    let wrapper = Box::new(plugin);
    Box::into_raw(wrapper) as *mut std::ffi::c_void
}

/// Dynamic library entry point for destruction
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _destroy_plugin(ptr: *mut std::ffi::c_void) {
    if !ptr.is_null() {
        let wrapper: Box<Box<dyn NotosPlugin>> = Box::from_raw(ptr as *mut Box<dyn NotosPlugin>);
        drop(wrapper);
    }
}
