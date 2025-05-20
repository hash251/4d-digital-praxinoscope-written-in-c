mod app;
mod models;
mod ui;
mod animation;
mod export;
mod utils;
mod input;

use app::PaintingApp;
use eframe::egui;
use clap::Parser;
use display_info::DisplayInfo;
use log;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[arg(long)]
    input: Option<String>,
    
    #[arg(long, default_value_t = 0)]
    instance: u8,

    #[arg(long, help = "monitor index (0 indexed, 0 = primary)")]
    monitor: Option<u32>,
}

fn main() -> eframe::Result {
    env_logger::init();

    let args = Args::parse();
    let instance = args.instance;
    let input_device_path = args.input;

    let mut viewport_builder = egui::ViewportBuilder::default();

    match DisplayInfo::all() {
        Ok(displays) if !displays.is_empty() => {
            let target_display_info: &DisplayInfo = if let Some(monitor_index) = args.monitor {
                if let Some(display) = displays.get(monitor_index as usize) {
                    log::info!(
                        "User requested monitor index: {}. Found display: '{}' ({}x{}) at ({},{}).",
                        monitor_index, display.name, display.width, display.height, display.x, display.y
                    );
                    display
                } else {
                    log::warn!(
                        "Monitor index {} out of bounds ({} displays available). Falling back to primary.",
                        monitor_index, displays.len()
                    );
                    displays.iter().find(|d| d.is_primary).unwrap_or_else(|| {
                        log::warn!("No primary display found. Falling back to first display.");
                        &displays[0]
                    })
                }
            } else {
                let primary_display = displays.iter().find(|d| d.is_primary).unwrap_or_else(|| {
                    log::warn!("No primary display found. Falling back to first display.");
                    &displays[0]
                });
                log::info!(
                    "No monitor specified. Using primary/first display: '{}' ({}x{}) at ({},{}).",
                    primary_display.name, primary_display.width, primary_display.height, primary_display.x, primary_display.y
                );
                primary_display
            };

            viewport_builder = viewport_builder
                .with_position(egui::pos2(target_display_info.x as f32, target_display_info.y as f32))
                .with_inner_size([target_display_info.width as f32, target_display_info.height as f32])
                .with_fullscreen(true) 
                .with_decorations(false);
        }
        Ok(_) => { 
            log::error!("No displays found by display-info crate. Using default viewport settings.");
            viewport_builder = viewport_builder
                .with_inner_size([1080.0, 1920.0])
                .with_maximized(true)
                .with_fullscreen(true);
        }
        Err(e) => {
            log::error!("Failed to get display info: {}. Using default viewport settings.", e);
            viewport_builder = viewport_builder
                .with_inner_size([1080.0, 1920.0]) 
                .with_maximized(true)
                .with_fullscreen(true);
        }
    }
    
    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    eframe::run_native(
        &format!("Drawing App {}", instance),
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(PaintingApp::new(input_device_path)))
        }),
    )
}