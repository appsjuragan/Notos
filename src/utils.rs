use eframe::egui::IconData;
use image::GenericImageView;

pub fn load_icon() -> IconData {
    let icon_bytes = include_bytes!("../assets/journal-alt.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon from memory");
    let (width, height) = image.dimensions();
    let rgba = image.to_rgba8().into_raw();
    
    IconData {
        rgba,
        width,
        height,
    }
}
