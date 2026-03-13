use crate::editor::{EditorTab, TabId};
use rfd::FileDialog;
use std::thread;

use super::NotosApp;

impl NotosApp {
    /// Internal helper to open a path. Returns true if successful.
    pub(crate) fn open_path(&mut self, path: std::path::PathBuf) -> bool {
        // Check if already open
        if let Some(pos) = self.tabs.iter().position(|t| {
            t.path.as_ref().map(|p| p.to_string_lossy()) == Some(path.to_string_lossy())
        }) {
            self.active_tab_id = Some(self.tabs[pos].id);
            self.tabs[pos].scroll_to_cursor = true;
            return true;
        }

        // Check if already loading
        if self.loading_paths.contains(&path) {
            return true;
        }

        self.loading_paths.insert(path.clone());
        let path_clone = path.clone();
        let tx = self.file_load_sender.clone();

        thread::spawn(move || {
            let res = EditorTab::from_file(path_clone.clone());
            let _ = tx.send((path_clone, res.map_err(|e| e.to_string())));
        });

        true
    }

    pub(crate) fn open_file(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            self.open_path(path);
        }
    }

    pub(crate) fn save_file(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            if tab.path.is_some() {
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                }
            } else {
                self.save_file_as();
            }
        }
    }

    pub(crate) fn save_file_as(&mut self) {
        if let Some(id) = self.active_tab_id {
            self.save_tab_as_by_id(id);
        }
    }

    pub(crate) fn save_tab_as_by_id(&mut self, id: TabId) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if let Some(path) = FileDialog::new()
                .add_filter("Text", &["txt", "md"])
                .add_filter("Rust", &["rs", "toml"])
                .add_filter("Python", &["py"])
                .add_filter("JavaScript", &["js", "ts"])
                .add_filter("HTML", &["html"])
                .add_filter("CSS", &["css"])
                .add_filter("All Files", &["*"])
                .set_file_name("untitled.txt")
                .save_file()
            {
                tab.set_path(path.clone());
                if let Err(e) = tab.save() {
                    log::error!("Failed to save file: {}", e);
                } else {
                    self.add_to_recent(path);
                }
            }
        }
    }

    pub(crate) fn close_tab(&mut self, id: TabId) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            if self.tabs[index].is_dirty {
                self.close_confirmation.open = true;
                self.close_confirmation.tab_id = Some(id);
                self.close_confirmation.closing_app = false;
            } else {
                self.tabs.remove(index);
                self.undo_manager.remove_tab(id);
                if self.active_tab_id == Some(id) {
                    self.active_tab_id = self.tabs.last().map(|t| t.id);
                }
            }
        }
    }

    pub(crate) fn add_to_recent(&mut self, path: std::path::PathBuf) {
        // Remove if already exists to move to top
        if let Some(pos) = self.recent_files.iter().position(|p| p == &path) {
            self.recent_files.remove(pos);
        }
        self.recent_files.insert(0, path);
        // Limit to 8
        self.recent_files.truncate(8);
    }
}
