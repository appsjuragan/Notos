use egui::Context;
use notos_sdk::{EditorContext, NotosPlugin, PluginAction};
use std::path::PathBuf;

/// Maximum file size we'll load into the hex viewer (128 MB).
const MAX_HEX_FILE_SIZE: u64 = 128 * 1024 * 1024;

/// Number of bytes displayed per row in the hex view.
const BYTES_PER_ROW: usize = 16;

/// Number of rows to display per page.
const ROWS_PER_PAGE: usize = 256;

use std::sync::{Arc, RwLock};

struct HexViewerState {
    /// Whether the hex viewer window is open.
    open: bool,
    /// The raw bytes loaded from the file.
    data: Vec<u8>,
    /// The path of the file currently loaded into the viewer.
    loaded_path: Option<PathBuf>,
    /// Status/error message shown in the viewer.
    status: String,
    /// Current scroll offset in rows.
    current_offset: usize,
    /// Search hex string input.
    search_hex: String,
    /// Byte offset of the last search match (highlighted).
    search_match: Option<usize>,
    /// Length of the last search match in bytes.
    search_match_len: usize,
    /// Go-to offset input string.
    goto_offset_str: String,
}

struct HexViewerPlugin {
    state: Arc<RwLock<HexViewerState>>,
}

impl HexViewerState {

    /// Load raw bytes from the given file path asynchronously.
    fn load_file_async(state: Arc<RwLock<Self>>, path: PathBuf, ctx: egui::Context) {
        {
            let mut s = state.write().unwrap();
            s.data.clear();
            s.search_match = None;
            s.current_offset = 0;
            s.status = format!("Loading {}...", path.display());
            s.loaded_path = Some(path.clone());
        }

        std::thread::spawn(move || {
            match std::fs::metadata(&path) {
                Ok(meta) => {
                    let file_size = meta.len();
                    if file_size > MAX_HEX_FILE_SIZE {
                        if let Ok(mut s) = state.write() {
                            if s.loaded_path.as_ref() == Some(&path) {
                                s.status = format!(
                                    "File too large ({:.2} MB). Max is {} MB.",
                                    file_size as f64 / (1024.0 * 1024.0),
                                    MAX_HEX_FILE_SIZE / (1024 * 1024)
                                );
                            }
                        }
                        ctx.request_repaint();
                        return;
                    }

                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            if let Ok(mut s) = state.write() {
                                if s.loaded_path.as_ref() == Some(&path) {
                                    s.status = format!(
                                        "Loaded {} bytes from {}",
                                        bytes.len(),
                                        path.display()
                                    );
                                    s.data = bytes;
                                }
                            }
                        }
                        Err(e) => {
                            if let Ok(mut s) = state.write() {
                                if s.loaded_path.as_ref() == Some(&path) {
                                    s.status = format!("Failed to read file: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Ok(mut s) = state.write() {
                        if s.loaded_path.as_ref() == Some(&path) {
                            s.status = format!("Failed to get file metadata: {}", e);
                        }
                    }
                }
            }
            ctx.request_repaint();
        });
    }

    /// Decode a hex search string into bytes.
    fn decode_hex(hex_str: &str) -> Option<Vec<u8>> {
        let clean: String = hex_str.chars().filter(|c| !c.is_whitespace()).collect();
        if clean.len() % 2 != 0 {
            return None;
        }
        let mut bytes = Vec::with_capacity(clean.len() / 2);
        for chunk in clean.as_bytes().chunks(2) {
            let s = std::str::from_utf8(chunk).ok()?;
            let b = u8::from_str_radix(s, 16).ok()?;
            bytes.push(b);
        }
        Some(bytes)
    }

    /// Find needle in data starting from `from` offset.
    fn find_bytes(data: &[u8], needle: &[u8], from: usize) -> Option<usize> {
        if needle.is_empty() || needle.len() > data.len() {
            return None;
        }
        let end = data.len() - needle.len() + 1;
        let start = from.min(end);
        for i in start..end {
            if data[i..i + needle.len()] == *needle {
                return Some(i);
            }
        }
        // Wrap around
        for i in 0..start.min(end) {
            if data[i..i + needle.len()] == *needle {
                return Some(i);
            }
        }
        None
    }

    /// Total number of rows for the current data.
    fn total_rows(&self) -> usize {
        if self.data.is_empty() {
            0
        } else {
            (self.data.len() + BYTES_PER_ROW - 1) / BYTES_PER_ROW
        }
    }

    /// Render the hex viewer window as a native OS window.
    fn show_window(state: Arc<RwLock<Self>>, ctx: &Context) {
        let is_open = state.read().unwrap().open;
        if !is_open {
            return;
        }

        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("hex_viewer_window"),
            egui::ViewportBuilder::default()
                .with_title("🔢 HEX Viewer")
                .with_inner_size([685.0, 550.0])
                .with_min_inner_size([600.0, 300.0]),
            move |ctx, class| {
                if class == egui::ViewportClass::Deferred {
                    let mut s = state.write().unwrap();
                    let mut open = s.open;
                    egui::CentralPanel::default().show(ctx, |ui| {
                        s.render_ui(ui);
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }
                    s.open = open;
                }
            },
        );
    }

    /// The actual UI content of the hex viewer.
    fn render_ui(&mut self, ui: &mut egui::Ui) {
        let is_dark = ui.visuals().dark_mode;

        // Toolbar
        ui.horizontal(|ui| {
            // Status label
            ui.label(
                egui::RichText::new(&self.status)
                    .small()
                    .color(if is_dark {
                        egui::Color32::from_rgb(150, 150, 150)
                    } else {
                        egui::Color32::from_rgb(100, 100, 100)
                    }),
            );
        });

        ui.separator();

        // Search and Go-to row
        ui.horizontal(|ui| {
            ui.label("Search (hex):");
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.search_hex)
                    .desired_width(160.0)
                    .hint_text("e.g. 48 65 6C 6C 6F")
                    .font(egui::TextStyle::Monospace),
            );
            if ui.button("🔍 Find").clicked()
                || (search_response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.do_search();
            }

            ui.separator();

            ui.label("Go to offset:");
            let goto_response = ui.add(
                egui::TextEdit::singleline(&mut self.goto_offset_str)
                    .desired_width(100.0)
                    .hint_text("0x or decimal")
                    .font(egui::TextStyle::Monospace),
            );
            if ui.button("⏩ Go").clicked()
                || (goto_response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.do_goto();
            }
        });

        ui.separator();

        if self.data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No file loaded. Open a file and click\n\"🔢 Hex Viewer\" from the Plugins menu.")
                        .size(14.0)
                        .color(if is_dark {
                            egui::Color32::from_rgb(160, 160, 160)
                        } else {
                            egui::Color32::from_rgb(100, 100, 100)
                        }),
                );
            });
            return;
        }

        // Navigation
        let total_rows = self.total_rows();
        let max_offset = total_rows.saturating_sub(ROWS_PER_PAGE);

        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.current_offset > 0, egui::Button::new("⏮ First"))
                .clicked()
            {
                self.current_offset = 0;
            }
            if ui
                .add_enabled(self.current_offset > 0, egui::Button::new("◀ Prev"))
                .clicked()
            {
                self.current_offset = self.current_offset.saturating_sub(ROWS_PER_PAGE);
            }
            if ui
                .add_enabled(
                    self.current_offset < max_offset,
                    egui::Button::new("Next ▶"),
                )
                .clicked()
            {
                self.current_offset =
                    (self.current_offset + ROWS_PER_PAGE).min(max_offset);
            }
            if ui
                .add_enabled(
                    self.current_offset < max_offset,
                    egui::Button::new("Last ⏭"),
                )
                .clicked()
            {
                self.current_offset = max_offset;
            }

            ui.separator();

            let page = if total_rows > 0 {
                self.current_offset / ROWS_PER_PAGE + 1
            } else {
                0
            };
            let total_pages = if total_rows > 0 {
                (total_rows + ROWS_PER_PAGE - 1) / ROWS_PER_PAGE
            } else {
                0
            };
            ui.label(format!(
                "Page {}/{} | Rows {}-{} of {} | {:.2} KB",
                page,
                total_pages,
                self.current_offset,
                (self.current_offset + ROWS_PER_PAGE).min(total_rows),
                total_rows,
                self.data.len() as f64 / 1024.0,
            ));
        });

        ui.separator();

        // Hex table header
        egui::ScrollArea::vertical()
            .id_salt("hex_viewer_vscroll")
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("hex_viewer_hscroll")
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 1.0);

                        let mono_font = egui::FontId::monospace(12.0);

                        // Header
                        let mut header = String::with_capacity(80);
                        header.push_str("  Offset   │");
                        for i in 0..BYTES_PER_ROW {
                            if i == 8 {
                                header.push_str("  ");
                            }
                            header.push_str(&format!(" {:02X}", i));
                        }
                        header.push_str(" │ ASCII");
                        ui.label(
                            egui::RichText::new(&header)
                                .font(mono_font.clone())
                                .color(if is_dark {
                                    egui::Color32::from_rgb(100, 180, 255)
                                } else {
                                    egui::Color32::from_rgb(0, 100, 200)
                                }),
                        );

                        // Separator
                        let sep_len = header.len();
                        ui.label(
                            egui::RichText::new("─".repeat(sep_len))
                                .font(mono_font.clone())
                                .color(if is_dark {
                                    egui::Color32::from_rgb(80, 80, 80)
                                } else {
                                    egui::Color32::from_rgb(200, 200, 200)
                                }),
                        );

                        // Data rows
                        let end_row =
                            (self.current_offset + ROWS_PER_PAGE).min(total_rows);

                        for row_idx in self.current_offset..end_row {
                            let row_start = row_idx * BYTES_PER_ROW;
                            let row_end = (row_start + BYTES_PER_ROW).min(self.data.len());
                            let row_bytes = &self.data[row_start..row_end];

                            let is_highlighted_row = self.search_match.map_or(false, |m| {
                                let match_end = m + self.search_match_len;
                                row_start < match_end && row_end > m
                            });

                            // Build row string
                            let mut row_str = format!(" {:08X} │", row_start);
                            for (i, &byte) in row_bytes.iter().enumerate() {
                                if i == 8 {
                                    row_str.push_str("  ");
                                }

                                let is_match_byte = self.search_match.map_or(false, |m| {
                                    let abs_pos = row_start + i;
                                    abs_pos >= m && abs_pos < m + self.search_match_len
                                });

                                if is_match_byte {
                                    row_str.push_str(&format!("[{:02X}", byte));
                                    row_str.push(']');
                                } else {
                                    row_str.push_str(&format!(" {:02X}", byte));
                                }
                            }
                            // Pad remaining
                            for i in row_bytes.len()..BYTES_PER_ROW {
                                if i == 8 {
                                    row_str.push_str("  ");
                                }
                                row_str.push_str("   ");
                            }

                            // ASCII column
                            row_str.push_str(" │ ");
                            for &byte in row_bytes {
                                if byte >= 0x20 && byte <= 0x7E {
                                    row_str.push(byte as char);
                                } else {
                                    row_str.push('.');
                                }
                            }

                            let text_color = if is_highlighted_row {
                                if is_dark {
                                    egui::Color32::from_rgb(255, 220, 100)
                                } else {
                                    egui::Color32::from_rgb(200, 140, 0)
                                }
                            } else if row_idx % 2 == 0 {
                                if is_dark {
                                    egui::Color32::from_rgb(210, 210, 210)
                                } else {
                                    egui::Color32::from_rgb(40, 40, 40)
                                }
                            } else {
                                if is_dark {
                                    egui::Color32::from_rgb(180, 180, 180)
                                } else {
                                    egui::Color32::from_rgb(90, 90, 90)
                                }
                            };

                            ui.label(
                                egui::RichText::new(&row_str)
                                    .font(mono_font.clone())
                                    .color(text_color),
                            );
                        }
                    });
            });
    }


    fn do_search(&mut self) {
        if self.search_hex.is_empty() {
            self.search_match = None;
            return;
        }

        if let Some(needle) = Self::decode_hex(&self.search_hex) {
            let search_from = self
                .search_match
                .map(|m| m + 1)
                .unwrap_or(0);
            if let Some(pos) = Self::find_bytes(&self.data, &needle, search_from) {
                self.search_match = Some(pos);
                self.search_match_len = needle.len();
                // Jump to the row containing the match
                self.current_offset = pos / BYTES_PER_ROW;
                self.status = format!("Found match at offset 0x{:08X} ({} bytes)", pos, pos);
            } else {
                self.search_match = None;
                self.status = "No match found.".to_string();
            }
        } else {
            self.status = "Invalid hex string. Use pairs like: 48 65 6C 6C 6F".to_string();
        }
    }

    fn do_goto(&mut self) {
        let input = self.goto_offset_str.trim();
        let offset = if input.starts_with("0x") || input.starts_with("0X") {
            usize::from_str_radix(&input[2..], 16).ok()
        } else {
            input.parse::<usize>().ok()
        };

        if let Some(off) = offset {
            if off < self.data.len() {
                self.current_offset = off / BYTES_PER_ROW;
                self.status = format!("Jumped to offset 0x{:08X}", off);
            } else {
                self.status = format!(
                    "Offset 0x{:X} is beyond file size ({} bytes).",
                    off,
                    self.data.len()
                );
            }
        } else {
            self.status = "Invalid offset. Use decimal or 0x-prefixed hex.".to_string();
        }
    }
}

impl HexViewerPlugin {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HexViewerState {
                open: false,
                data: Vec::new(),
                loaded_path: None,
                status: String::new(),
                current_offset: 0,
                search_hex: String::new(),
                search_match: None,
                search_match_len: 0,
                goto_offset_str: String::new(),
            })),
        }
    }
}

impl NotosPlugin for HexViewerPlugin {
    fn id(&self) -> &str {
        "notos_hex_viewer"
    }

    fn name(&self) -> &str {
        "HEX Viewer"
    }

    fn plugins_menu_ui(&mut self, ui: &mut egui::Ui, ed: &EditorContext) -> PluginAction {
        if ui.button("🔢 Hex Viewer").clicked() {
            let mut needs_reload = false;
            let mut target_path = None;

            {
                let s = self.state.read().unwrap();
                if let Some(path) = ed.file_path {
                    needs_reload = s
                        .loaded_path
                        .as_ref()
                        .map_or(true, |loaded| loaded.as_path() != path);
                    if needs_reload {
                        target_path = Some(path.to_path_buf());
                    }
                }
            }

            {
                let mut s = self.state.write().unwrap();
                s.open = true;
                if ed.file_path.is_none() {
                    // No file path — show hex of the current content's raw UTF-8 bytes
                    s.data = ed.content.as_bytes().to_vec();
                    s.loaded_path = None;
                    s.current_offset = 0;
                    s.search_match = None;
                    s.status = format!("Showing in-memory content ({} bytes)", s.data.len());
                }
            }

            if needs_reload {
                if let Some(path) = target_path {
                    let ctx = ui.ctx().clone();
                    HexViewerState::load_file_async(self.state.clone(), path, ctx);
                }
            }

            ui.close_menu();
        }

        PluginAction::None
    }

    fn ui(&mut self, ctx: &Context, _ed: &EditorContext) -> PluginAction {
        let state_clone = Arc::clone(&self.state);
        HexViewerState::show_window(state_clone, ctx);
        PluginAction::None
    }
}

/// Dynamic library entry point for creation
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _create_plugin() -> *mut std::ffi::c_void {
    let plugin: Box<dyn NotosPlugin> = Box::new(HexViewerPlugin::new());
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
