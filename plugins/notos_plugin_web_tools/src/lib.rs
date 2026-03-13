use notos_sdk::{EditorContext, NotosPlugin, PluginAction};
use minifier::js::minify as js_minify;
use minifier::css::minify as css_minify;

struct WebToolsPlugin;

impl WebToolsPlugin {
    fn new() -> Self {
        Self
    }
}

impl NotosPlugin for WebToolsPlugin {
    fn id(&self) -> &str {
        "notos_web_tools"
    }

    fn name(&self) -> &str {
        "Web Tools (Minify)"
    }

    fn plugins_menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        let mut action = PluginAction::None;

        ui.menu_button("🌐 Web Tools", |ui| {
            if ui.button("🚀 Minify JS").clicked() {
                action = self.apply_to_selection_or_all(ed, |t| js_minify(t).to_string());
                ui.close_menu();
            }
            if ui.button("🎨 Minify CSS").clicked() {
                action = self.apply_to_selection_or_all(ed, |t| {
                    match css_minify(t) {
                        Ok(m) => m.to_string(),
                        Err(_) => t.to_string(),
                    }
                });
                ui.close_menu();
            }
            
            ui.separator();
            ui.label("Format (Simple)");
            
            if ui.button("🧹 Basic Format CSS").clicked() {
                action = self.apply_to_selection_or_all(ed, |t| self.simple_css_unminify(t));
                ui.close_menu();
            }
        });

        action
    }
}

impl WebToolsPlugin {
    fn apply_to_selection_or_all<F>(&self, ed: &EditorContext, transform: F) -> PluginAction 
    where F: Fn(&str) -> String {
        if let Some((s, e)) = ed.selection {
            let (start, end) = (s.min(e), s.max(e));
            if start != end {
                if let Some(selected_text) = ed.content.get(start..end) {
                    return PluginAction::ReplaceSelection(transform(selected_text));
                }
            }
        }
        if !ed.content.is_empty() {
            return PluginAction::ReplaceAll(transform(ed.content));
        }
        PluginAction::None
    }

    fn simple_css_unminify(&self, css: &str) -> String {
        // Very rudimentary unminifier: add newlines and indentation
        let mut result = String::new();
        let mut indent = 0;
        for c in css.chars() {
            match c {
                '{' => {
                    result.push_str(" {\n");
                    indent += 1;
                    result.push_str(&"    ".repeat(indent));
                }
                '}' => {
                    indent = indent.saturating_sub(1);
                    result.push_str("\n");
                    result.push_str(&"    ".repeat(indent));
                    result.push_str("}\n");
                    if indent > 0 {
                        result.push_str(&"    ".repeat(indent));
                    }
                }
                ';' => {
                    result.push_str(";\n");
                    result.push_str(&"    ".repeat(indent));
                }
                _ => result.push(c),
            }
        }
        result
    }
}

#[no_mangle]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(WebToolsPlugin::new());
    let wrapper = Box::new(plugin);
    Box::into_raw(wrapper) as *mut std::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn _destroy_plugin(ptr: *mut std::ffi::c_void) {
    if !ptr.is_null() {
        let wrapper: Box<Box<dyn NotosPlugin>> = Box::from_raw(ptr as *mut Box<dyn NotosPlugin>);
        drop(wrapper);
    }
}
