use crate::app::PaintingApp;
use eframe::egui::{self, Color32, Stroke, Pos2, Rect};
use crate::models::{Stroke as DrawingStroke, StrokeType};

impl PaintingApp {
    pub fn update_animation(&mut self, ctx: &egui::Context) {
        if self.playing_animation {
            ctx.request_repaint();
            let now = ctx.input(|i| i.time);
            let frame_duration = 1.0 / self.animation_speed as f64;

            if now - self.last_frame_time >= frame_duration {
                self.current_frame = (self.current_frame + 1) % self.frames.len();
                self.last_frame_time = now;
            }
        }
    }

    pub fn draw_onion_skins(&self, painter: &egui::Painter) {
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

    pub fn draw_onion_skin_stroke(
        &self,
        painter: &egui::Painter,
        stroke: &DrawingStroke,
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

            painter.line_segment([p1, p2], Stroke::new(stroke.size, onion_color));
        }
    }

    pub fn draw_stroke(&self, painter: &egui::Painter, stroke: &DrawingStroke) {
        match stroke.stroke_type {
            StrokeType::Draw => {
                if stroke.points.len() == 1 {
                    let point = stroke.points[0];
                    painter.circle_filled(point, stroke.size / 2.0, stroke.color);
                } else if stroke.points.len() >= 2 {
                    for window in stroke.points.windows(2) {
                        let p1 = window[0];
                        let p2 = window[1];
                        painter.line_segment([p1, p2], Stroke::new(stroke.size, stroke.color));
                    }
                }
            }
        }
    }
    
    pub fn draw_thumbnail_content(
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
                
                match stroke.stroke_type {
                    StrokeType::Draw => {
                        if scaled_points.len() < 2 {
                            if let Some(point) = scaled_points.first() {
                                painter.circle_filled(*point, stroke.size * scale * 0.5, stroke.color);
                            }
                        } else {
                            for window in scaled_points.windows(2) {
                                painter.line_segment(
                                    [window[0], window[1]],
                                    Stroke::new(stroke.size * scale, stroke.color),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}