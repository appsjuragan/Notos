use egui::{Ui, RichText, Color32};
use crate::editor::EditorTab;

pub fn tab_bar(ui: &mut Ui, tabs: &mut Vec<EditorTab>, active_tab_id: &mut Option<uuid::Uuid>) {
    ui.horizontal(|ui| {
        let mut tab_to_close = None;

        for (index, tab) in tabs.iter().enumerate() {
            let is_active = Some(tab.id) == active_active_tab_id(active_tab_id);
            
            let title = if tab.is_dirty {
                format!("* {}", tab.title)
            } else {
                tab.title.clone()
            };

            let text = if is_active {
                RichText::new(title).strong().color(Color32::WHITE)
            } else {
                RichText::new(title)
            };

            let response = ui.selectable_label(is_active, text);
            if response.clicked() {
                *active_tab_id = Some(tab.id);
            }
            
            // Middle click to close
            if response.middle_clicked() {
                tab_to_close = Some(index);
            }

            // Context menu for tabs
            response.context_menu(|ui| {
                if ui.button("Close").clicked() {
                    tab_to_close = Some(index);
                    ui.close_menu();
                }
                if ui.button("Close Others").clicked() {
                    // Implement close others
                    ui.close_menu();
                }
            });
        }

        if let Some(index) = tab_to_close {
            let removed = tabs.remove(index);
            if Some(removed.id) == *active_tab_id {
                // If we closed the active tab, select the last one or none
                *active_tab_id = tabs.last().map(|t| t.id);
            }
        }
        
        // New Tab Button
        if ui.button("+").clicked() {
            let new_tab = EditorTab::default();
            *active_tab_id = Some(new_tab.id);
            tabs.push(new_tab);
        }
    });
}

fn active_active_tab_id(id: &Option<uuid::Uuid>) -> Option<uuid::Uuid> {
    *id
}
