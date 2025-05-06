use eframe::egui::{self, Color32, Key, Pos2, Rect, Vec2};
use crate::models::{Stroke, Notification};
use crate::ui::{draw_left_panel, draw_frame_panel, draw_canvas};
use crate::utils::distance_to_line_segment;

#[derive(PartialEq, Clone)]
pub enum ToolMode {
    Brush,
    Eraser
}

pub struct PaintingApp {
    pub brush_color: Color32,
    pub brush_size: f32,
    pub frames: Vec<Vec<Stroke>>,
    pub current_frame: usize,
    pub current_stroke: Option<Stroke>,
    pub onion_skin_opacity: f32,
    pub show_onion_skin: bool,
    pub prev_onion_color: Color32,
    pub next_onion_color: Color32,
    pub copied_frame: Option<Vec<Stroke>>,
    pub export_url: String,
    pub canvas_aspect_ratio: f32,
    pub canvas_rect: Option<Rect>,
    pub playing_animation: bool,
    pub animation_speed: f32,
    pub last_frame_time: f64,
    pub original_canvas_rect: Option<Rect>,

    pub undo_history: Vec<Vec<Vec<Stroke>>>,
    pub redo_history: Vec<Vec<Vec<Stroke>>>,
    pub tool_mode: ToolMode,

    pub notifications: Vec<Notification>,
    pub next_notification_id: u64,
    pub exporting: bool, 
    pub export_cooldown: f64,
    pub last_export_time: f64,
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
            export_url: "http://localhost:1337/upload".to_string(),
            canvas_aspect_ratio: 3.0 / 4.0,
            canvas_rect: None,
            playing_animation: false,
            animation_speed: 10.0,
            last_frame_time: 0.0,
            original_canvas_rect: None,
            undo_history: Vec::new(),  
            redo_history: Vec::new(),
            tool_mode: ToolMode::Brush,
            notifications: Vec::new(),
            next_notification_id: 0,
            exporting: false,
            export_cooldown: 0.1, // chgange back to 10 seconds
            last_export_time: 0.0,
        }
    }
}

impl eframe::App for PaintingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.2);

        self.update_notifications(ctx);

        if self.exporting {
            self.exporting = false;
            
            let frames = self.frames.clone();
            let export_url = self.export_url.clone();
            let canvas_rect = self.canvas_rect.clone();
            
            let ctx_clone = ctx.clone();
            let next_id = self.next_notification_id;
            self.next_notification_id += 1;
            
            std::thread::spawn(move || {
                Self::export_animation_threaded(frames, export_url, canvas_rect, next_id, ctx_clone);
            });
        }

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

        self.update_animation(ctx);

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            draw_left_panel(self, ctx, ui);
        });

        draw_frame_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_canvas(self, ui);
        });

        self.draw_notifications(ctx);
    }
}

impl PaintingApp {
    pub fn save_state_for_undo(&mut self) {
        let current_state = self.frames.clone();
        self.undo_history.push(current_state);
        self.redo_history.clear();

        if self.undo_history.len() > 30 {
            self.undo_history.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if !self.undo_history.is_empty() {
            let current_state = self.frames.clone();
            self.redo_history.push(current_state);

            let previous_state = self.undo_history.pop().unwrap();
            self.frames = previous_state;
        }
    }

    pub fn redo(&mut self) {
        if !self.redo_history.is_empty() {
            let current_state = self.frames.clone();
            self.undo_history.push(current_state);

            let next_state = self.redo_history.pop().unwrap();
            self.frames = next_state;
        }
    }

    pub fn calculate_aspect_ratio_rect(&self, available_rect: Rect) -> Rect {
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

    pub fn calculate_thumbnail_rect(&self, available_rect: Rect) -> Rect {
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

    pub fn copy_current_frame(&mut self) {
        self.copied_frame = Some(self.frames[self.current_frame].clone());
    }

    pub fn paste_to_current_frame(&mut self) {
        let copied_frame = if let Some(ref copied) = self.copied_frame {
            Some(copied.clone())
        } else {
            None
        };
        
        if let Some(frame) = copied_frame {
            self.save_state_for_undo();
            self.frames[self.current_frame] = frame;
        }
    }

    pub fn erase_strokes_at_position(&mut self, pos: Pos2, radius: f32) {
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

    pub fn recalculate_stroke_positions(&mut self, new_rect: Rect) {
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

    pub fn add_notification(&mut self, message: String, color: Color32, duration: f64, ctx: &egui::Context) {
        let notification = Notification {
            id: self.next_notification_id,
            message,
            color,
            created_at: ctx.input(|i| i.time),
            duration,
        };
        self.next_notification_id += 1;
        self.notifications.push(notification);
    }

    pub fn update_notifications(&mut self, ctx: &egui::Context) {
        ctx.data_mut(|d| {
            let notifications = d.get_temp_mut_or_default::<Vec<Notification>>(egui::Id::new("global_notifications"));
            if !notifications.is_empty() {
                let mut to_move = Vec::new();
                std::mem::swap(notifications, &mut to_move);
                self.notifications.append(&mut to_move);
            }
        });
        
        let current_time = ctx.input(|i| i.time);
        self.notifications.retain(|notification| {
            current_time - notification.created_at < notification.duration
        });
        
        if !self.notifications.is_empty() {
            ctx.request_repaint();
        }
    }

    pub fn draw_notifications(&self, ctx: &egui::Context) {
        if self.notifications.is_empty() {
            return;
        }
        
        let screen_rect = ctx.screen_rect();
        let notification_width = screen_rect.width() * 0.3;
        let mut y_offset = 20.0;
        
        for notification in &self.notifications {
            let fade_time = 0.5;
            let time_alive = ctx.input(|i| i.time) - notification.created_at;
            let time_left = notification.duration - time_alive;
            
            let opacity = if time_alive < fade_time {
                time_alive / fade_time
            } else if time_left < fade_time {
                time_left / fade_time
            } else {
                1.0
            };
            
            let alpha = (opacity * 255.0) as u8;
            let bg_color = Color32::from_rgba_unmultiplied(40, 40, 40, alpha);
            let text_color = Color32::from_rgba_unmultiplied(
                notification.color.r(),
                notification.color.g(),
                notification.color.b(),
                alpha,
            );
            
            let window_id = egui::Id::new(format!("notification_{}", notification.id));
            
            egui::Window::new(format!("Notification {}", notification.id))
                .id(window_id)
                .fixed_size([notification_width, 0.0])
                .fixed_pos([screen_rect.right() - notification_width - notification_width / 20.0, y_offset])
                .title_bar(false)
                .frame(egui::Frame::none().fill(bg_color).rounding(8.0).stroke(egui::Stroke::new(1.0, text_color)))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.colored_label(text_color, &notification.message);
                        ui.add_space(10.0);
                    });
                });
            
            y_offset += 50.0;
        }
    }
}