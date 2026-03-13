#![windows_subsystem = "windows"] // Hide console window on Windows

mod app;
mod dialogs;
mod editor;
mod plugin;
mod ui;
mod undo_manager;
mod utils;

use app::NotosApp;
use utils::load_icon;

fn main() -> eframe::Result<()> {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`)

    let icon = load_icon();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(icon),
        ..Default::default()
    };

    let args: Vec<String> = std::env::args()
        .skip(1)
        .map(|a| {
            std::fs::canonicalize(&a)
                .unwrap_or_else(|_| std::path::PathBuf::from(a))
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    // Single instance logic: try to bind to a local port
    let (tx, rx) = std::sync::mpsc::channel();
    let port = 55123;
    let addr = format!("127.0.0.1:{}", port);

    match std::net::TcpListener::bind(&addr) {
        Ok(listener) => eframe::run_native(
            &format!("Notos Text Editor v{}", env!("CARGO_PKG_VERSION")),
            native_options,
            Box::new(move |cc| {
                let ctx = cc.egui_ctx.clone();
                std::thread::spawn(move || {
                    for mut stream in listener.incoming().flatten() {
                        let mut buffer = String::new();
                        use std::io::Read;
                        if stream.read_to_string(&mut buffer).is_ok() {
                            let paths: Vec<String> = buffer
                                .split('\n')
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string())
                                .collect();
                            for path in paths {
                                let _ = tx.send(path);
                            }
                            ctx.request_repaint();
                        }
                    }
                });
                Ok(Box::new(NotosApp::new(cc, args, rx)))
            }),
        ),
        Err(_) => {
            // Another instance is likely running, send the paths and exit
            if !args.is_empty() {
                use std::io::Write;
                if let Ok(mut stream) = std::net::TcpStream::connect(&addr) {
                    let abs_paths: Vec<String> = args
                        .iter()
                        .map(|a| {
                            std::fs::canonicalize(a)
                                .unwrap_or_else(|_| std::path::PathBuf::from(a))
                                .to_string_lossy()
                                .into_owned()
                        })
                        .collect();
                    let data = abs_paths.join("\n");
                    let _ = stream.write_all(data.as_bytes());
                }
            }
            Ok(())
        }
    }
}
