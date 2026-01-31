use eframe::egui::IconData;

pub fn load_icon() -> IconData {
    // We no longer load the PNG icon to save binary size (~1MB).
    // The application icon is handled by winres in build.rs for Windows.
    IconData::default()
}
