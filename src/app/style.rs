use eframe::egui;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Try to load system fonts to avoid bundling them (saves ~1.5MB - 2MB)
    let mut font_data = std::collections::BTreeMap::new();

    #[cfg(target_os = "windows")]
    {
        let font_dir = std::path::Path::new("C:\\Windows\\Fonts");

        // Proportional: Segoe UI
        if let Ok(bytes) = std::fs::read(font_dir.join("segoeui.ttf")) {
            font_data.insert("Segoe UI".to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "Segoe UI".to_owned());
        } else if let Ok(bytes) = std::fs::read(font_dir.join("arial.ttf")) {
            font_data.insert("Arial".to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "Arial".to_owned());
        }

        // Monospace: Consolas
        if let Ok(bytes) = std::fs::read(font_dir.join("consola.ttf")) {
            font_data.insert("Consolas".to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "Consolas".to_owned());
        } else if let Ok(bytes) = std::fs::read(font_dir.join("lucon.ttf")) {
            // Lucida Console
            font_data.insert(
                "Lucida Console".to_owned(),
                egui::FontData::from_owned(bytes),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "Lucida Console".to_owned());
        }

        // Icons & Emojis: Segoe UI Symbol & Emoji (Crucial for UI icons like ↩, ↪, 🔍)
        if let Ok(bytes) = std::fs::read(font_dir.join("seguisym.ttf")) {
            font_data.insert(
                "Segoe UI Symbol".to_owned(),
                egui::FontData::from_owned(bytes),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .push("Segoe UI Symbol".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("Segoe UI Symbol".to_owned());
        }
        if let Ok(bytes) = std::fs::read(font_dir.join("seguiemj.ttf")) {
            font_data.insert(
                "Segoe UI Emoji".to_owned(),
                egui::FontData::from_owned(bytes),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .push("Segoe UI Emoji".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("Segoe UI Emoji".to_owned());
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Simple search for common Linux fonts
        let paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];
        for path in paths {
            if let Ok(bytes) = std::fs::read(path) {
                font_data.insert("System Sans".to_owned(), egui::FontData::from_owned(bytes));
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "System Sans".to_owned());
                break;
            }
        }

        let mono_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ];
        for path in mono_paths {
            if let Ok(bytes) = std::fs::read(path) {
                font_data.insert("System Mono".to_owned(), egui::FontData::from_owned(bytes));
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "System Mono".to_owned());
                break;
            }
        }
    }

    if font_data.is_empty() {
        log::warn!("No system fonts found. UI might be invisible or broken.");
    }

    fonts.font_data = font_data;
    ctx.set_fonts(fonts);
}

pub fn setup_custom_style(ctx: &egui::Context, dark_mode: bool) {
    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    if dark_mode {
        // Deep dark background for the editor
        let editor_bg = egui::Color32::from_gray(28);
        // Slightly lighter for the panels
        let panel_bg = egui::Color32::from_gray(38);

        visuals.panel_fill = panel_bg;
        visuals.window_fill = panel_bg;
        visuals.extreme_bg_color = editor_bg;

        // Ensure buttons and non-interactive areas have correct background
        visuals.widgets.noninteractive.bg_fill = panel_bg;
        visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(45);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(55);
        visuals.widgets.active.bg_fill = egui::Color32::from_gray(65);

        // Contrasty text colors
        visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_gray(220);
        visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_gray(230);
        visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
        visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;

        visuals.window_shadow.color = egui::Color32::from_black_alpha(100);
    } else {
        visuals.widgets.noninteractive.bg_fill = egui::Color32::WHITE;
        visuals.window_fill = egui::Color32::WHITE;
        visuals.panel_fill = egui::Color32::WHITE;
        visuals.extreme_bg_color = egui::Color32::WHITE;

        visuals.selection.bg_fill = egui::Color32::from_rgb(202, 227, 255);
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 215));
    }

    ctx.set_visuals(visuals.clone());

    // Update the whole style to ensure everything propagates
    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    // Add some padding to widgets for a more modern look
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    style.spacing.button_padding = egui::vec2(4.0, 2.0);
    style.spacing.menu_margin = egui::Margin::same(4.0);
    ctx.set_style(style);
}
