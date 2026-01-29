#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release

mod app;
mod editor;
mod plugin;
mod ui;

use app::NotosApp;

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`)

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // We can load an icon here later
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon.png")[..]).ok()
            ),
        ..Default::default()
    };

    eframe::run_native(
        "Notos Text Editor",
        native_options,
        Box::new(|cc| Ok(Box::new(NotosApp::new(cc)))),
    )
}
