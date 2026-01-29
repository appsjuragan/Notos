use eframe::egui::IconData;
use std::path::Path;

pub fn load_icon() -> IconData {
    let icon_path = Path::new("assets/journal-alt.ico");
    
    // In a real app, you might want to embed the icon bytes using `include_bytes!`
    // or handle the error more gracefully.
    // For now, we try to load from file, and fallback to default if it fails.
    
    match image::open(icon_path) {
        Ok(image) => {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            IconData {
                rgba,
                width,
                height,
            }
        }
        Err(e) => {
            log::warn!("Failed to load icon: {}", e);
            IconData::default()
        }
    }
}
