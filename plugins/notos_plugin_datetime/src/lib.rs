use notos_sdk::{EditorContext, NotosPlugin, PluginAction};
use chrono::Local;

struct DateTimePlugin;

impl DateTimePlugin {
    fn new() -> Self {
        Self
    }
}

impl NotosPlugin for DateTimePlugin {
    fn id(&self) -> &str {
        "notos_datetime"
    }

    fn name(&self) -> &str {
        "Customizable Date/Time"
    }

    fn plugins_menu_ui(&mut self, ui: &mut egui::Ui, _ed: &EditorContext) -> PluginAction {
        let mut action = PluginAction::None;

        ui.menu_button("📅 Insert Date/Time", |ui| {
            let now = Local::now();
            
            if ui.button(format!("Standard: {}", now.format("%Y-%m-%d %H:%M:%S"))).clicked() {
                action = PluginAction::ReplaceSelection(now.format("%Y-%m-%d %H:%M:%S").to_string());
                ui.close_menu();
            }
            
            if ui.button(format!("ISO 8601: {}", now.to_rfc3339())).clicked() {
                action = PluginAction::ReplaceSelection(now.to_rfc3339());
                ui.close_menu();
            }

            if ui.button(format!("Date Only: {}", now.format("%Y-%m-%d"))).clicked() {
                action = PluginAction::ReplaceSelection(now.format("%Y-%m-%d").to_string());
                ui.close_menu();
            }

            if ui.button(format!("Time Only: {}", now.format("%H:%M:%S"))).clicked() {
                action = PluginAction::ReplaceSelection(now.format("%H:%M:%S").to_string());
                ui.close_menu();
            }

            ui.separator();

            if ui.button(format!("Unix Timestamp: {}", now.timestamp())).clicked() {
                action = PluginAction::ReplaceSelection(now.timestamp().to_string());
                ui.close_menu();
            }
        });

        action
    }
}

#[no_mangle]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(DateTimePlugin::new());
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
