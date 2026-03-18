use egui::Context;
use notos_sdk::{EditorContext, NotosPlugin, PluginAction};

struct UrlDetectorPlugin {
    enabled: bool,
}

impl UrlDetectorPlugin {
    fn new() -> Self {
        Self { enabled: true }
    }

    /// Check whether a string looks like a domain-based URL (no protocol required).
    /// Matches patterns like `www.example.com`, `github.com/repo`, `sub.domain.co.uk/path`.
    fn looks_like_url(s: &str) -> bool {
        // Must have a protocol or at least one dot for a domain
        if s.starts_with("http://") || s.starts_with("https://") {
            return true;
        }

        // Find the host part (everything before the first '/')
        let host = s.split('/').next().unwrap_or(s);

        // Must contain at least one dot
        if !host.contains('.') {
            return false;
        }

        // The TLD (last segment after last dot) must be 2-13 alphabetic chars
        if let Some(tld) = host.rsplit('.').next() {
            if tld.len() >= 2 && tld.len() <= 13 && tld.chars().all(|c| c.is_ascii_alphabetic()) {
                return true;
            }
        }
        false
    }

    /// Ensure the URL has a protocol prefix for opening in a browser.
    fn ensure_protocol(url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        }
    }

    fn find_url_range_at_index(content: &str, index: usize) -> Option<(usize, usize, String)> {
        if content.is_empty() || index > content.len() {
            return None;
        }

        let mut start = index;
        let mut end = index;

        let is_url_char = |c: char| {
            c.is_alphanumeric()
                || c == ':'
                || c == '/'
                || c == '.'
                || c == '-'
                || c == '_'
                || c == '?'
                || c == '='
                || c == '&'
                || c == '#'
                || c == '%'
                || c == '@'
                || c == '+'
                || c == '~'
        };

        // Find start of potential URL
        let chars: Vec<(usize, char)> = content.char_indices().collect();
        let mut char_idx_opt = chars.iter().position(|&(i, _)| i >= index);

        // If index is past last char but text exists
        if char_idx_opt.is_none() && !chars.is_empty() {
            char_idx_opt = Some(chars.len() - 1);
        }

        if let Some(char_idx) = char_idx_opt {
            let mut s_idx = char_idx;
            while s_idx > 0 && is_url_char(chars[s_idx - 1].1) {
                s_idx -= 1;
            }
            start = chars[s_idx].0;

            let mut e_idx = char_idx;
            while e_idx < chars.len() && is_url_char(chars[e_idx].1) {
                e_idx += 1;
            }
            end = if e_idx < chars.len() {
                chars[e_idx].0
            } else {
                content.len()
            };
        }

        if start < end {
            let extracted = &content[start..end];
            // Strip trailing dots/punctuation that are unlikely part of the URL
            let trimmed = extracted.trim_end_matches('.');
            if trimmed.is_empty() {
                return None;
            }
            let trimmed_end = start + trimmed.len();

            if Self::looks_like_url(trimmed) {
                return Some((start, trimmed_end, trimmed.to_string()));
            }
        }
        None
    }
}

impl NotosPlugin for UrlDetectorPlugin {
    fn id(&self) -> &str {
        "notos_url_detector"
    }

    fn name(&self) -> &str {
        "URL Detector"
    }

    fn plugins_menu_ui(&mut self, ui: &mut egui::Ui, _ed: &EditorContext) -> PluginAction {
        ui.checkbox(&mut self.enabled, "Enable URL Detection");
        PluginAction::None
    }

    fn ui(&mut self, ctx: &Context, ed: &EditorContext) -> PluginAction {
        if !self.enabled {
            return PluginAction::None;
        }

        // Handle Hover Underline and Cursor
        let mut action = PluginAction::None;
        if let Some(hovered_idx) = ed.hovered_char_idx {
            if let Some((start, end, _)) = Self::find_url_range_at_index(ed.content, hovered_idx) {
                // Change cursor and underline only if CTRL is held
                if ctx.input(|i| i.modifiers.ctrl) {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    action = PluginAction::UnderlineRegion(start, end);
                }
            }
        }

        // Handle Ctrl+Click
        if ctx.input(|i| i.modifiers.ctrl && i.pointer.primary_clicked()) {
            // we will use the hovered index if available to open, falling back to selection start
            let target_idx = ed.hovered_char_idx.or_else(|| ed.selection.map(|(s, _)| s));

            if let Some(idx) = target_idx {
                if let Some((_, _, url)) = Self::find_url_range_at_index(ed.content, idx) {
                    let url_str = Self::ensure_protocol(&url);
                    std::thread::spawn(move || {
                        #[cfg(target_os = "windows")]
                        let result = {
                            use std::os::windows::process::CommandExt;
                            const CREATE_NO_WINDOW: u32 = 0x08000000;
                            std::process::Command::new("cmd")
                                .creation_flags(CREATE_NO_WINDOW)
                                .args(["/c", "start", "", &url_str])
                                .spawn()
                        };

                        #[cfg(target_os = "macos")]
                        let result = std::process::Command::new("open").arg(&url_str).spawn();

                        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
                        let result = std::process::Command::new("xdg-open").arg(&url_str).spawn();

                        if let Err(e) = result {
                            eprintln!("Failed to open URL '{}': {}", url_str, e);
                        }
                    });
                }
            }
        }

        action
    }
}

/// Dynamic library entry point for creation
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(UrlDetectorPlugin::new());
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
