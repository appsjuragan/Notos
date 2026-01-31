use egui::Context;
use notos_sdk::NotosPlugin;

struct AboutPlugin {
    open: bool,
}

impl AboutPlugin {
    fn new() -> Self {
        Self { open: false }
    }
}

impl NotosPlugin for AboutPlugin {
    fn id(&self) -> &str {
        "notos_about"
    }

    fn name(&self) -> &str {
        "About Plugin"
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("About Notos").clicked() {
                self.open = true;
                ui.close_menu();
            }
        });
    }

    fn ui(&mut self, ctx: &Context) {
        if self.open {
            egui::Window::new("About Notos")
                .open(&mut self.open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Notos Text Editor");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.label("Developed by appsjuragan");
                    ui.hyperlink("https://github.com/appsjuragan/Notos");
                });
        }
    }
}

/// Dynamic library entry point for creation
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    // Wrap the trait object in ANOTHER box so we can pass a thin pointer to it.
    // *mut dyn NotosPlugin is 16 bytes (fat), but *mut Box<dyn NotosPlugin> is 8 bytes (thin).
    let plugin: Box<dyn NotosPlugin> = Box::new(AboutPlugin::new());
    let wrapper = Box::new(plugin);
    Box::into_raw(wrapper) as *mut std::ffi::c_void
}

/// Dynamic library entry point for destruction
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _destroy_plugin(ptr: *mut std::ffi::c_void) {
    if !ptr.is_null() {
        // Re-wrap and let it drop
        let wrapper: Box<Box<dyn NotosPlugin>> = Box::from_raw(ptr as *mut Box<dyn NotosPlugin>);
        drop(wrapper);
    }
}
