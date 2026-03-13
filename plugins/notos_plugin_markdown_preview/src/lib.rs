use notos_sdk::{EditorContext, NotosPlugin, PluginAction};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::sync::{Arc, RwLock};

struct MarkdownState {
    open: bool,
    cache: CommonMarkCache,
}

struct MarkdownPreviewPlugin {
    state: Arc<RwLock<MarkdownState>>,
}

impl MarkdownPreviewPlugin {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MarkdownState {
                open: false,
                cache: CommonMarkCache::default(),
            })),
        }
    }
}

impl NotosPlugin for MarkdownPreviewPlugin {
    fn id(&self) -> &str {
        "notos_markdown_preview"
    }

    fn name(&self) -> &str {
        "Markdown Live Preview"
    }

    fn plugins_menu_ui(&mut self, ui: &mut egui::Ui, _ed: &EditorContext) -> PluginAction {
        let mut s = self.state.write().unwrap();
        if ui.checkbox(&mut s.open, "📖 Show Markdown Preview").changed() {
            // Repaint to show/hide
            ui.ctx().request_repaint();
        }
        PluginAction::None
    }

    fn ui(&mut self, ctx: &egui::Context, ed: &EditorContext) -> PluginAction {
        let is_open = self.state.read().unwrap().open;
        if is_open {
            let state_clone = Arc::clone(&self.state);
            let content = ed.content.to_string();
            
            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("markdown_preview_window"),
                egui::ViewportBuilder::default()
                    .with_title("📝 Markdown Preview")
                    .with_inner_size([600.0, 800.0]),
                move |ctx, _class| {
                    let mut s = state_clone.write().unwrap();
                    
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("md_preview_scroll")
                            .show(ui, |ui| {
                                CommonMarkViewer::new()
                                    .show(ui, &mut s.cache, &content);
                            });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        s.open = false;
                    }
                },
            );
        }
        PluginAction::None
    }
}

#[no_mangle]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(MarkdownPreviewPlugin::new());
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
