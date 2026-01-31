use base64::{engine::general_purpose, Engine as _};
use notos_sdk::{EditorContext, NotosPlugin, PluginAction};

struct Base64Plugin;

impl Base64Plugin {
    fn new() -> Self {
        Self
    }

    fn encode(&self, text: &str) -> String {
        general_purpose::STANDARD.encode(text)
    }

    fn decode(&self, text: &str) -> Option<String> {
        let trimmed = text.trim();
        match general_purpose::STANDARD.decode(trimmed) {
            Ok(bytes) => String::from_utf8(bytes).ok(),
            Err(e) => {
                log::warn!("Base64 decode error: {}", e);
                None
            }
        }
    }
}

impl NotosPlugin for Base64Plugin {
    fn id(&self) -> &str {
        "notos_base64"
    }

    fn name(&self) -> &str {
        "Base64 Tool"
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        let mut action = PluginAction::None;

        ui.menu_button("Plugins", |ui| {
            if ui.button("Base64 Encode").clicked() {
                if let Some((start, end)) = ed.selection {
                    if start != end {
                        if let Some(selected_text) = ed.content.get(start..end) {
                            action = PluginAction::ReplaceSelection(self.encode(selected_text));
                        }
                    } else if !ed.content.is_empty() {
                        action = PluginAction::ReplaceAll(self.encode(ed.content));
                    }
                } else if !ed.content.is_empty() {
                    action = PluginAction::ReplaceAll(self.encode(ed.content));
                }
                ui.close_menu();
            }

            if ui.button("Base64 Decode").clicked() {
                if let Some((start, end)) = ed.selection {
                    if start != end {
                        if let Some(selected_text) = ed.content.get(start..end) {
                            if let Some(decoded) = self.decode(selected_text) {
                                action = PluginAction::ReplaceSelection(decoded);
                            }
                        }
                    } else if !ed.content.is_empty() {
                        if let Some(decoded) = self.decode(ed.content) {
                            action = PluginAction::ReplaceAll(decoded);
                        }
                    }
                } else if !ed.content.is_empty() {
                    if let Some(decoded) = self.decode(ed.content) {
                        action = PluginAction::ReplaceAll(decoded);
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
    let plugin: Box<dyn NotosPlugin> = Box::new(Base64Plugin::new());
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
