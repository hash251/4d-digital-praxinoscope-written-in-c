mod app;
mod models;
mod ui;
mod animation;
mod export;
mod utils;

use app::PaintingApp;
use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 1920.0])
            .with_maximized(true)
            .with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "Drawing app",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<PaintingApp>::default())
        }),
    )
}