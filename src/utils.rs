use eframe::egui::IconData;

pub fn load_icon() -> IconData {
    let icon_bytes = include_bytes!("../assets/journal-alt.png");

    match image::load_from_memory(icon_bytes) {
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
            log::warn!("Failed to load embedded icon: {}", e);
            IconData::default()
        }
    }
}
