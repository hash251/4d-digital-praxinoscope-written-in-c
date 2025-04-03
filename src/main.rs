use eframe::egui;
use eframe::egui::{Color32, Key, Pos2, Rect, Sense, Stroke as EguiStroke, Vec2};

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1080.0, 1920.0]),
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

#[derive(Clone)]
struct Stroke {
    points: Vec<Pos2>,
    color: Color32,
    size: f32,
}

struct PaintingApp {
    brush_color: Color32,
    brush_size: f32,
    frames: Vec<Vec<Stroke>>,
    current_frame: usize,
    current_stroke: Option<Stroke>,
    onion_skin_opacity: f32,
    show_onion_skin: bool,
    prev_onion_color: Color32,
    next_onion_color: Color32,
    copied_frame: Option<Vec<Stroke>>,
    export_url: String,
    canvas_aspect_ratio: f32,
    canvas_rect: Option<Rect>,
    playing_animation: bool,
    animation_speed: f32,
    last_frame_time: f64,
    original_canvas_rect: Option<Rect>,

    undo_history: Vec<Vec<Vec<Stroke>>>,
    redo_history: Vec<Vec<Vec<Stroke>>>,
    eraser_mode: bool,
}

impl Default for PaintingApp {
    fn default() -> Self {
        let mut frames = Vec::new();

        for _ in 0..8 {
            frames.push(Vec::new());
        }

        Self {
            brush_color: Color32::BLACK,
            brush_size: 5.0,
            frames,
            current_frame: 0,
            current_stroke: None,
            onion_skin_opacity: 0.3,
            show_onion_skin: true,
            prev_onion_color: Color32::RED,
            next_onion_color: Color32::BLUE,
            copied_frame: None,
            export_url: "http://localhost:1337/upload".to_string(), // TODO: server ip
            canvas_aspect_ratio: 3.0 / 4.0, // 3:4 aspect ratio
            canvas_rect: None,
            playing_animation: false,
            animation_speed: 10.0, // in fps
            last_frame_time: 0.0,
            original_canvas_rect: None,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            eraser_mode: false,
        }
    }
}

impl eframe::App for PaintingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::C)) {
            self.copy_current_frame();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::V)) {
            self.paste_to_current_frame();
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Z)) {
            self.undo();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Y)) {
            self.redo();
        }

        if self.playing_animation {
            ctx.request_repaint();
            let now = ctx.input(|i| i.time);
            let frame_duration = 1.0 / self.animation_speed as f64;

            if now - self.last_frame_time >= frame_duration {
                self.current_frame = (self.current_frame + 1) % self.frames.len();
                self.last_frame_time = now;
            }
        }

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
            
            let mut style = (*ctx.style()).clone();
            style.text_styles = [
                (egui::TextStyle::Heading, egui::FontId::new(20.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Body, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Small, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
            ]
            .into();
            ctx.set_style(style);
            
            let larger_font = egui::FontId::new(20.0, egui::FontFamily::Proportional);


            ui.add_space(10.0);
            ui.heading("Tools");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Brush Color:");
                ui.color_edit_button_srgba(&mut self.brush_color);
            });

            ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=20.0).text("Brush Size"));

            ui.horizontal(|ui| {
                let brush_btn = ui.add(egui::SelectableLabel::new(
                    !self.eraser_mode, 
                    egui::RichText::new("Brush").font(larger_font.clone())
                ));
                
                let eraser_btn = ui.add(egui::SelectableLabel::new(
                    self.eraser_mode, 
                    egui::RichText::new("Eraser").font(larger_font.clone())
                ));

                if brush_btn.clicked() {
                    self.eraser_mode = false;
                }
                if eraser_btn.clicked() {
                    self.eraser_mode = true;
                }
            });

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("↩ Undo").clicked() {
                    self.undo();
                }
                if ui.button("↪ Redo").clicked() {
                    self.redo();
                }
            });

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(4.0);

            ui.heading("Animation");
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                if ui
                    .button(if self.playing_animation {
                        "⏹ Stop "
                    } else {
                        "▶ Play"
                    })
                    .clicked()
                {
                    self.playing_animation = !self.playing_animation;
                    self.last_frame_time = ui.ctx().input(|i| i.time);
                }
            });

            ui.add(egui::Slider::new(&mut self.animation_speed, 1.0..=24.0).text("FPS"));

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(4.0);

            ui.heading("Onion Skinning");
            ui.add_space(4.0);

            ui.checkbox(&mut self.show_onion_skin, "Show Onion Skin");
            ui.add(egui::Slider::new(&mut self.onion_skin_opacity, 0.0..=1.0).text("Opacity"));

            // not sure how useful / necessary this is
            ui.horizontal(|ui| {
                ui.label("Previous Frame Color:");
                ui.color_edit_button_srgba(&mut self.prev_onion_color);
            });

            ui.horizontal(|ui| {
                ui.label("Next Frame Color:");
                ui.color_edit_button_srgba(&mut self.next_onion_color);
            });

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(4.0);

            ui.heading("Frame Operations");
            ui.add_space(5.0);

            egui::Grid::new("frame_ops_grid")
                .num_columns(2)
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    if ui.button("Clear Frame").clicked() {
                        self.save_state_for_undo();
                        self.frames[self.current_frame].clear();
                    }
                    
                    if ui.button("Reset All Frames").clicked() {
                        self.save_state_for_undo();
                        self.current_frame = 0;

                        for frame in &mut self.frames {
                            frame.clear();
                        }
                    }
                    ui.end_row();

                    if ui.button("Copy Frame").clicked() {
                        self.copy_current_frame();
                    }

                    if ui.button("Paste Frame").clicked() {
                        self.save_state_for_undo();
                        self.paste_to_current_frame();
                    }
                    ui.end_row();
                });

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(4.0);


            let export = ui.add(egui::SelectableLabel::new(
                true, 
                egui::RichText::new("Export").font(larger_font)
            ));

            if export.clicked() {
                self.export_animation();
            }
        });

        let current_frame = self.current_frame;
        egui::TopBottomPanel::bottom("frame_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for i in 0..self.frames.len() {
                    ui.vertical(|ui| {
                        let is_selected = current_frame == i;

                        let frame_size = 60.0;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(frame_size, frame_size),
                            egui::Sense::click(),
                        );

                        if response.clicked() {
                            self.current_frame = i;

                            self.playing_animation = false;
                        }

                        ui.painter().rect_filled(
                            rect,
                            0.0,
                            if is_selected {
                                Color32::RED
                            } else {
                                Color32::GRAY
                            },
                        );

                        let inner_rect = rect.shrink(3.0);
                        ui.painter().rect_filled(inner_rect, 0.0, Color32::WHITE);

                        let content_rect = self.calculate_thumbnail_rect(inner_rect.shrink(2.0));

                        self.draw_thumbnail_content(i, ui.painter(), content_rect);

                        ui.label(format!("Frame {}", i + 1));
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            let canvas_rect = self.calculate_aspect_ratio_rect(panel_rect);

            if let Some(old_rect) = self.canvas_rect {
                if old_rect != canvas_rect {
                    if self.original_canvas_rect.is_none() {
                        self.original_canvas_rect = Some(old_rect);
                    }

                    self.recalculate_stroke_positions(old_rect);
                }
            } else if self.original_canvas_rect.is_none() {
                self.original_canvas_rect = Some(canvas_rect);
            }

            self.canvas_rect = Some(canvas_rect);

            let (response, painter) = ui.allocate_painter(panel_rect.size(), Sense::drag());

            ui.painter()
                .rect_filled(panel_rect, 0.0, Color32::DARK_GRAY);

            painter.rect_filled(canvas_rect, 0.0, Color32::WHITE);

            painter.rect_stroke(canvas_rect, 0.0, EguiStroke::new(1.0, Color32::BLACK));

            if self.show_onion_skin && !self.playing_animation {
                self.draw_onion_skins(&painter);
            }

            for stroke in &self.frames[self.current_frame] {
                self.draw_stroke(&painter, stroke);
            }

            if !self.playing_animation && response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if pos.x >= canvas_rect.min.x
                        && pos.x <= canvas_rect.max.x
                        && pos.y >= canvas_rect.min.y
                        && pos.y <= canvas_rect.max.y
                    {
                        if self.eraser_mode {
                            let eraser_size = self.brush_size * 2.0;
                            self.erase_strokes_at_position(pos, eraser_size);
                        } else {
                            if self.current_stroke.is_none() {
                                self.current_stroke = Some(Stroke {
                                    points: vec![pos],
                                    color: self.brush_color,
                                    size: self.brush_size,
                                });
                            } else if let Some(stroke) = &mut self.current_stroke {
                                stroke.points.push(pos);
                            }
                        }
                    }
                }
            } else if response.drag_stopped() || (self.current_stroke.is_some() && !response.dragged()) {
                if let Some(stroke) = self.current_stroke.take() {
                    if !stroke.points.is_empty() {
                        self.save_state_for_undo();
                        self.frames[self.current_frame].push(stroke);
                    }
                }
            }

            if !self.playing_animation && !self.eraser_mode {
                if let Some(stroke) = &self.current_stroke {
                    self.draw_stroke(&painter, stroke);
                }
            } else if !self.playing_animation && self.eraser_mode {
                if let Some(pos) = response.hover_pos() {
                    if pos.x >= canvas_rect.min.x
                        && pos.x <= canvas_rect.max.x
                        && pos.y >= canvas_rect.min.y
                        && pos.y <= canvas_rect.max.y
                    {
                        let radius = self.brush_size * 2.0;
                        painter.circle_stroke(pos, radius, EguiStroke::new(2.0, Color32::RED));
                    }
                }
            }

            if self.playing_animation {
                let text = format!(
                    "Playing: Frame {} of {}",
                    self.current_frame + 1,
                    self.frames.len()
                );
                let text_pos = Pos2::new(canvas_rect.min.x + 10.0, canvas_rect.min.y + 20.0);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::new(16.0, egui::FontFamily::Proportional),
                    Color32::BLACK,
                );
            }
        });
    }
}

impl PaintingApp {
    fn save_state_for_undo(&mut self) {
        let current_state = self.frames.clone();

        self.undo_history.push(current_state);

        self.redo_history.clear();

        if self.undo_history.len() > 30 {
            self.undo_history.remove(0);
        }
    }

    fn undo(&mut self) {
        if !self.undo_history.is_empty() {
            let current_state = self.frames.clone();
            self.redo_history.push(current_state);

            let previous_state = self.undo_history.pop().unwrap();
            self.frames = previous_state;
        }
    }

    fn redo(&mut self) {
        if !self.redo_history.is_empty() {
            let current_state = self.frames.clone();
            self.undo_history.push(current_state);

            let next_state = self.redo_history.pop().unwrap();
            self.frames = next_state;
        }
    }

    fn calculate_aspect_ratio_rect(&self, available_rect: Rect) -> Rect {
        let available_width = available_rect.width();
        let available_height = available_rect.height();

        let (canvas_width, canvas_height) =
            if available_width / self.canvas_aspect_ratio <= available_height {
                (
                    available_width * 0.95,
                    available_width * 0.95 / self.canvas_aspect_ratio,
                )
            } else {
                (
                    available_height * 0.95 * self.canvas_aspect_ratio,
                    available_height * 0.95,
                )
            };

        let center_x = available_rect.center().x;
        let center_y = available_rect.center().y;

        Rect::from_center_size(
            Pos2::new(center_x, center_y),
            Vec2::new(canvas_width, canvas_height),
        )
    }

    fn calculate_thumbnail_rect(&self, available_rect: Rect) -> Rect {
        let available_width = available_rect.width();
        let available_height = available_rect.height();

        let (thumb_width, thumb_height) =
            if available_width / self.canvas_aspect_ratio <= available_height {
                (available_width, available_width / self.canvas_aspect_ratio)
            } else {
                (
                    available_height * self.canvas_aspect_ratio,
                    available_height,
                )
            };

        Rect::from_center_size(
            available_rect.center(),
            Vec2::new(thumb_width, thumb_height),
        )
    }

    fn copy_current_frame(&mut self) {
        self.copied_frame = Some(self.frames[self.current_frame].clone());
    }

    fn paste_to_current_frame(&mut self) {
        if let Some(ref copied) = self.copied_frame {
            self.frames[self.current_frame] = copied.clone();
        }
    }

    fn draw_onion_skins(&self, painter: &egui::Painter) {
        let prev_frame_index = if self.current_frame > 0 {
            self.current_frame - 1
        } else {
            self.frames.len() - 1
        };

        let next_frame_index = if self.current_frame < self.frames.len() - 1 {
            self.current_frame + 1
        } else {
            0
        };

        let prev_frame = &self.frames[prev_frame_index];
        for stroke in prev_frame {
            self.draw_onion_skin_stroke(
                painter,
                stroke,
                self.prev_onion_color,
                self.onion_skin_opacity,
            );
        }

        let next_frame = &self.frames[next_frame_index];
        for stroke in next_frame {
            self.draw_onion_skin_stroke(
                painter,
                stroke,
                self.next_onion_color,
                self.onion_skin_opacity,
            );
        }
    }

    fn draw_onion_skin_stroke(
        &self,
        painter: &egui::Painter,
        stroke: &Stroke,
        color: Color32,
        opacity: f32,
    ) {
        if stroke.points.len() < 2 {
            return;
        }

        let onion_color = Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            (opacity * 255.0) as u8,
        );

        for window in stroke.points.windows(2) {
            let p1 = window[0];
            let p2 = window[1];

            painter.line_segment([p1, p2], EguiStroke::new(stroke.size, onion_color));
        }
    }

    fn draw_stroke(&self, painter: &egui::Painter, stroke: &Stroke) {
        if stroke.points.len() < 2 {
            if let Some(point) = stroke.points.first() {
                painter.circle_filled(*point, stroke.size / 2.0, stroke.color);
            }
            return;
        }

        for window in stroke.points.windows(2) {
            let p1 = window[0];
            let p2 = window[1];

            painter.line_segment([p1, p2], EguiStroke::new(stroke.size, stroke.color));

            painter.circle_filled(p1, stroke.size / 2.0, stroke.color);
            painter.circle_filled(p2, stroke.size / 2.0, stroke.color);
        }
    }

    fn draw_thumbnail_content(
        &self,
        frame_index: usize,
        painter: &egui::Painter,
        thumb_rect: Rect,
    ) {
        if frame_index >= self.frames.len() {
            return;
        }

        let strokes = &self.frames[frame_index];

        if strokes.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for stroke in strokes {
            for point in &stroke.points {
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
            }
        }

        if min_x <= max_x && min_y <= max_y {
            let content_width = max_x - min_x;
            let content_height = max_y - min_y;

            if content_width <= 0.0 || content_height <= 0.0 {
                return;
            }

            let thumb_width = thumb_rect.width();
            let thumb_height = thumb_rect.height();

            let scale_x = thumb_width / content_width;
            let scale_y = thumb_height / content_height;
            let scale = scale_x.min(scale_y);

            let content_scaled_width = content_width * scale;
            let content_scaled_height = content_height * scale;
            let x_offset = thumb_rect.min.x + (thumb_width - content_scaled_width) / 2.0;
            let y_offset = thumb_rect.min.y + (thumb_height - content_scaled_height) / 2.0;

            for stroke in strokes {
                let scaled_points: Vec<Pos2> = stroke
                    .points
                    .iter()
                    .map(|point| {
                        Pos2::new(
                            x_offset + (point.x - min_x) * scale,
                            y_offset + (point.y - min_y) * scale,
                        )
                    })
                    .collect();

                if scaled_points.len() < 2 {
                    if let Some(point) = scaled_points.first() {
                        painter.circle_filled(*point, stroke.size * scale * 0.5, stroke.color);
                    }
                } else {
                    for window in scaled_points.windows(2) {
                        painter.line_segment(
                            [window[0], window[1]],
                            EguiStroke::new(stroke.size * scale, stroke.color),
                        );
                    }
                }
            }
        }
    }

    fn erase_strokes_at_position(&mut self, pos: Pos2, radius: f32) {
        let mut to_remove = Vec::new();

        for (i, stroke) in self.frames[self.current_frame].iter().enumerate() {
            for point in &stroke.points {
                let distance = point.distance(pos);
                if distance <= radius {
                    to_remove.push(i);
                    break;
                }
            }

            if !to_remove.contains(&i) && stroke.points.len() >= 2 {
                for window in stroke.points.windows(2) {
                    let p1 = window[0];
                    let p2 = window[1];

                    let distance = distance_to_line_segment(pos, p1, p2);
                    if distance <= radius {
                        to_remove.push(i);
                        break;
                    }
                }
            }
        }

        if !to_remove.is_empty() {
            self.save_state_for_undo();

            to_remove.sort_unstable();
            to_remove.dedup();
            for i in to_remove.into_iter().rev() {
                self.frames[self.current_frame].remove(i);
            }
        }
    }

    fn recalculate_stroke_positions(&mut self, new_rect: Rect) {
        let original_rect = if let Some(rect) = self.original_canvas_rect {
            rect
        } else {
            return;
        };

        for frame in &mut self.frames {
            for stroke in frame {
                for point in &mut stroke.points {
                    let rel_x = (point.x - original_rect.min.x) / original_rect.width();
                    let rel_y = (point.y - original_rect.min.y) / original_rect.height();

                    point.x = new_rect.min.x + (rel_x * new_rect.width());
                    point.y = new_rect.min.y + (rel_y * new_rect.height());
                }
            }
        }
    }

    fn export_animation(&self) {
        let temp_dir = std::path::Path::new("./temp_frames");
        if !temp_dir.exists() {
            std::fs::create_dir_all(temp_dir).expect("failed to create temp directory");
        }

        println!("exporting animation to {}", self.export_url);

        let canvas_rect = if let Some(rect) = self.canvas_rect {
            rect
        } else {
            println!("err: no canvas rect found");
            return;
        };

        let width = canvas_rect.width() as u32;
        let height = canvas_rect.height() as u32;

        let client = reqwest::blocking::Client::new();

        let mut form = reqwest::blocking::multipart::Form::new();

        for (frame_index, strokes) in self.frames.iter().enumerate() {
            let mut imgbuf = image::RgbaImage::new(width, height);

            for pixel in imgbuf.pixels_mut() {
                *pixel = image::Rgba([255, 255, 255, 255]);
            }

            let mut ctx = tiny_skia::Pixmap::new(width, height).expect("failed to create pixmap");

            let mut paint = tiny_skia::Paint::default();
            paint.set_color(tiny_skia::Color::WHITE);
            ctx.fill_rect(
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

                        ctx.fill_path(
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

                    ctx.stroke_path(
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
                    let pixel = ctx.pixel(x, y).unwrap();
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

            let frame_file = format!("./temp_frames/frame_{:01}.png", frame_index);
            imgbuf
                .save_with_format(&frame_file, image::ImageFormat::Png)
                .expect("failed to save frame as png");

            let file_part = reqwest::blocking::multipart::Part::file(frame_file.clone())
                .expect("failed to create file part")
                .file_name(format!("frame_{:01}.png", frame_index))
                .mime_str("image/png")
                .expect("failed to set mime type");

            form = form.part(format!("frame_{}", frame_index), file_part);
        }

        match client.post(&self.export_url).multipart(form).send() {
            Ok(response) => {
                if response.status().is_success() {
                    println!("animation exported successfully!");
                } else {
                    println!(
                        "failed to export animation. server returned: {}",
                        response.status()
                    );
                }
            }
            Err(e) => println!("failed to export animation: {}", e),
        }

        if temp_dir.exists() {
            // std::fs::remove_dir_all(temp_dir).expect("failed to clean up temp directory");
        }
    } 
}

fn distance_to_line_segment(p: Pos2, v: Pos2, w: Pos2) -> f32 {
    let l2 = v.distance_sq(w);

    if l2 == 0.0 {
        return p.distance(v);
    }

    // we need to find projection of point p onto the current line segment parameterized as v + t (w - v)
    // t = [(p-v) . (w-v)] / |w-v|^2
    let t = ((p - v).dot(w - v) / l2).clamp(0.0, 1.0);

    let projection = v + t * (w - v);

    p.distance(projection)
}
