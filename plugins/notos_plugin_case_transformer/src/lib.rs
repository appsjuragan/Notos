use notos_sdk::{EditorContext, NotosPlugin, PluginAction};
use heck::{ToShoutySnakeCase, ToSnakeCase, ToKebabCase, ToPascalCase, ToLowerCamelCase, ToTitleCase};

struct CaseTransformerPlugin;

impl CaseTransformerPlugin {
    fn new() -> Self {
        Self
    }
}

impl NotosPlugin for CaseTransformerPlugin {
    fn id(&self) -> &str {
        "notos_case_transformer"
    }

    fn name(&self) -> &str {
        "Case Transformer"
    }

    fn context_menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        self.draw_case_menu(ui, ed)
    }

    fn plugins_menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        self.draw_case_menu(ui, ed)
    }
}

impl CaseTransformerPlugin {
    fn draw_case_menu(&self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        let mut action = PluginAction::None;
        
        ui.menu_button("🔠 Transform Case", |ui| {
            if ui.button("UPPERCASE").clicked() {
                action = self.apply_transform(ed, |s| s.to_uppercase());
                ui.close_menu();
            }
            if ui.button("lowercase").clicked() {
                action = self.apply_transform(ed, |s| s.to_lowercase());
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Title Case").clicked() {
                action = self.apply_transform(ed, |s| s.to_title_case());
                ui.close_menu();
            }
            if ui.button("PascalCase").clicked() {
                action = self.apply_transform(ed, |s| s.to_pascal_case());
                ui.close_menu();
            }
            if ui.button("camelCase").clicked() {
                action = self.apply_transform(ed, |s| s.to_lower_camel_case());
                ui.close_menu();
            }
            if ui.button("snake_case").clicked() {
                action = self.apply_transform(ed, |s| s.to_snake_case());
                ui.close_menu();
            }
            if ui.button("SCREAMING_SNAKE_CASE").clicked() {
                action = self.apply_transform(ed, |s| s.to_shouty_snake_case());
                ui.close_menu();
            }
            if ui.button("kebab-case").clicked() {
                action = self.apply_transform(ed, |s| s.to_kebab_case());
                ui.close_menu();
            }
        });

        action
    }

    fn apply_transform<F>(&self, ed: &EditorContext, transform: F) -> PluginAction 
    where F: Fn(&str) -> String {
        if let Some((s, e)) = ed.selection {
            let (start, end) = (s.min(e), s.max(e));
            if start != end {
                if let Some(selected_text) = ed.content.get(start..end) {
                    return PluginAction::ReplaceSelection(transform(selected_text));
                }
            }
        }
        // If no selection, transform the whole content? 
        // Usually, case transformation is better restricted to selection to avoid accidents.
        // But for consistency with base64 plugin, we can do ReplaceAll if no selection.
        if !ed.content.is_empty() {
             return PluginAction::ReplaceAll(transform(ed.content));
        }
        PluginAction::None
    }
}

#[no_mangle]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(CaseTransformerPlugin::new());
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
