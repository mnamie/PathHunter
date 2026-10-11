// No console window behind the GUI in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod clean_view;
mod theme;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Path Hunter")
            .with_inner_size([1180.0, 740.0])
            .with_min_inner_size([760.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Path Hunter",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(app::App::new()))
        }),
    )
}
