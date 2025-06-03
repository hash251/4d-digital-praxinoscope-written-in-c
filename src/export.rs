use crate::app::PaintingApp;
use eframe::egui::{self, Color32, Rect};
use crate::models::{Stroke, Notification};

impl PaintingApp {
    pub fn start_export_animation(&mut self, ctx: &egui::Context) {
        let current_time = ctx.input(|i| i.time);
        
        if current_time - self.last_export_time < self.export_cooldown {
            let remaining = self.export_cooldown - (current_time - self.last_export_time);
            self.add_notification(
                format!("Please wait {:.1} seconds before exporting again", remaining),
                Color32::YELLOW,
                2.0,
                ctx,
            );
            return;
        }
        
        self.last_export_time = current_time;
        
        self.add_notification(
            "Rendering frames and exporting animation...".to_string(),
            Color32::YELLOW,
            3.0,
            ctx,
        );
        
        self.exporting = true;
    }

    pub fn export_animation_threaded(
        frames: Vec<Vec<Stroke>>, 
        export_url: String, 
        canvas_rect_opt: Option<Rect>,
        notification_id: u64,
        ctx: egui::Context
    ) {
        let temp_dir = std::path::Path::new("/home/softdev/programming/project/target/release/temp_frames");
        if !temp_dir.exists() {
            std::fs::create_dir_all(temp_dir).expect("failed to create temp directory");
        }

        println!("exporting animation to {}", export_url);

        let canvas_rect = if let Some(rect) = canvas_rect_opt {
            rect
        } else {
            println!("err: no canvas rect found");
            Self::add_notification_static(
                "Error: Could not export animation".to_string(),
                Color32::RED,
                5.0,
                notification_id + 1,
                &ctx,
            );
            return;
        };

        let width = canvas_rect.width() as u32;
        let height = canvas_rect.height() as u32;

        let client = reqwest::blocking::Client::new();
        let mut form = reqwest::blocking::multipart::Form::new();

        for (frame_index, strokes) in frames.iter().enumerate() {
            let mut imgbuf = image::RgbaImage::new(width, height);

            for pixel in imgbuf.pixels_mut() {
                *pixel = image::Rgba([255, 255, 255, 255]);
            }

            let mut ctx_skia = tiny_skia::Pixmap::new(width, height).expect("failed to create pixmap");

            let mut paint = tiny_skia::Paint::default();
            paint.set_color(tiny_skia::Color::WHITE);
            ctx_skia.fill_rect(
                tiny_skia::Rect::from_xywh(0.0, 0.0, width as f32, height as f32).unwrap(),
                &paint,
                tiny_skia::Transform::identity(),
                None,
            );

            for stroke in strokes {
                if stroke.points.len() < 2 {
                    if let Some(point) = stroke.points.first() {
                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color(tiny_skia::Color::from_rgba8(
                            stroke.color.r(),
                            stroke.color.g(),
                            stroke.color.b(),
                            stroke.color.a(),
                        ));

                        let x = point.x - canvas_rect.min.x;
                        let y = point.y - canvas_rect.min.y;

                        let path =
                            tiny_skia::PathBuilder::from_circle(x, y, stroke.size / 2.0).unwrap();

                        ctx_skia.fill_path(
                            &path,
                            &paint,
                            tiny_skia::FillRule::Winding,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                } else {
                    let mut path = tiny_skia::PathBuilder::new();
                    let mut first = true;

                    for point in &stroke.points {
                        let x = point.x - canvas_rect.min.x;
                        let y = point.y - canvas_rect.min.y;

                        if first {
                            path.move_to(x, y);
                            first = false;
                        } else {
                            path.line_to(x, y);
                        }
                    }

                    let path = path.finish().expect("failed to create path");

                    let mut stroke_paint = tiny_skia::Paint::default();
                    stroke_paint.set_color(tiny_skia::Color::from_rgba8(
                        stroke.color.r(),
                        stroke.color.g(),
                        stroke.color.b(),
                        stroke.color.a(),
                    ));

                    let mut stroke_style = tiny_skia::Stroke::default();
                    stroke_style.width = stroke.size;
                    stroke_style.line_cap = tiny_skia::LineCap::Round;
                    stroke_style.line_join = tiny_skia::LineJoin::Round;

                    ctx_skia.stroke_path(
                        &path,
                        &stroke_paint,
                        &stroke_style,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }

            for y in 0..height {
                for x in 0..width {
                    let pixel = ctx_skia.pixel(x, y).unwrap();
                    imgbuf.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            pixel.red(),
                            pixel.green(),
                            pixel.blue(),
                            pixel.alpha()
                        ]),
                    );
                }
            }

            let frame_file = format!("/home/softdev/programming/project/target/release/temp_frames/{:01}.png", frame_index);
            imgbuf
                .save_with_format(&frame_file, image::ImageFormat::Png)
                .expect("failed to save frame as png");

            let file_part = reqwest::blocking::multipart::Part::file(frame_file.clone())
                .expect("failed to create file part")
                .file_name(format!("{:01}.png", frame_index))
                .mime_str("image/png")
                .expect("failed to set mime type");

            form = form.part(format!("{}", frame_index), file_part);
        }

        match client.post(&export_url).multipart(form).send() {
            Ok(response) => {
                if response.status().is_success() {
                    println!("animation exported successfully!");
                    Self::add_notification_static(
                        "Animation exported successfully!".to_string(),
                        Color32::GREEN,
                        5.0,
                        notification_id + 1,
                        &ctx,
                    );
                } else {
                    println!(
                        "failed to export animation. server returned: {}",
                        response.status()
                    );
                    Self::add_notification_static(
                        format!("Export failed: Server returned {}", response.status()),
                        Color32::RED,
                        5.0,
                        notification_id + 1,
                        &ctx,
                    );
                }
            }
            Err(e) => {
                println!("failed to export animation: {}", e);
                Self::add_notification_static(
                    format!("Export failed: {}", e),
                    Color32::RED,
                    5.0,
                    notification_id + 1,
                    &ctx,
                );
            }
        }

        if temp_dir.exists() {
            std::fs::remove_dir_all(temp_dir).expect("failed to clean up temp directory");
        }
    }

    pub fn add_notification_static(message: String, color: Color32, duration: f64, id: u64, ctx: &egui::Context) {
        let notification = Notification {
            id,
            message,
            color,
            created_at: ctx.input(|i| i.time),
            duration,
        };
        
        ctx.push_notification(notification);
        ctx.request_repaint();
    }
}

pub trait ContextExt {
    fn push_notification(&self, notification: Notification);
}

impl ContextExt for egui::Context {
    fn push_notification(&self, notification: Notification) {
        self.data_mut(|d| {
            let notifications = d.get_temp_mut_or_default::<Vec<Notification>>(egui::Id::new("global_notifications"));
            notifications.push(notification);
        });
        self.request_repaint();
    }
}