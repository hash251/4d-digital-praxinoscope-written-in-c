use eframe::egui::{self, Color32, Key, Pos2, Rect, Vec2, RichText, FontId, FontFamily};
use crate::models::{Stroke, Notification};
use crate::ui::{draw_left_panel, draw_frame_panel, draw_canvas};
use crate::utils::{distance_to_line_segment, get_local_ip_address};
use crate::input::InputHandler;
use std::collections::HashMap;
use crate::models::Stroke as DrawingStroke;

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

    pub undo_history: Vec<(Vec<Vec<Stroke>>, Rect)>,
    pub redo_history: Vec<(Vec<Vec<Stroke>>, Rect)>,
    pub tool_mode: ToolMode,

    pub left_panel_open: bool,
    pub show_admin_panel: bool,
    pub local_ip_address: Option<String>,

    pub notifications: Vec<Notification>,
    pub next_notification_id: u64,
    pub exporting: bool,
    pub export_cooldown: f64,
    pub last_export_time: f64,

    pub input_handler: Option<InputHandler>,
    pub active_touches: HashMap<u32, DrawingStroke>,
    pub invert_input: bool,
    pub target_position: Pos2,
}

impl PaintingApp {
    pub fn new(input_device_path_option: Option<String>, invert_input: bool, target_position: Pos2) -> Self {
        let mut frames = Vec::new();
        for _ in 0..8 {
            frames.push(Vec::new());
        }

        let input_handler = match input_device_path_option {
            Some(path_str) => {
                match InputHandler::new(&path_str) {
                    Ok(handler) => {
                        log::info!("InputHandler initialized successfully with device: {}.", path_str);
                        Some(handler)
                    }
                    Err(e) => {
                        log::error!("Failed to initialize InputHandler with device {}: {}. Touch input may be disabled.", path_str, e);
                        None
                    }
                }
            }
            None => {
                let default_path = "/dev/input/by-id/usb-Elo_Touch_Solutions_Elo_Touch_Solutions_Pcap_USB_Interface-event-if00";
                log::warn!(
                    "No input device path provided via --input. Attempting to use default path: {}",
                    default_path
                );
                match InputHandler::new(default_path) {
                    Ok(handler) => {
                        log::info!("InputHandler initialized successfully with default device.");
                        Some(handler)
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to initialize InputHandler with default device: {}. Touch input may be disabled. Consider providing an input device path via --input if touch is not working.",
                            e
                        );
                        None
                    }
                }
            }
        };

        let local_ip = get_local_ip_address();
        if local_ip.is_none() {
            log::warn!("Could not determine local IP address. Admin link will use a default (127.0.0.1).");
        }

        Self {
            brush_color: Color32::BLACK,
            brush_size: 5.0,
            frames,
            current_frame: 0,
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
            export_cooldown: 0.1, 
            last_export_time: 0.0,
            input_handler,
            active_touches: HashMap::new(),
            left_panel_open: false,
            invert_input,
            target_position,
            show_admin_panel: false,
            local_ip_address: local_ip,
        }
    }
}

impl eframe::App for PaintingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.2);

        self.update_notifications(ctx);

        let current_pos = ctx.input(|i| i.viewport().clone()).outer_rect.unwrap().min;
        if (current_pos.x - self.target_position.x).abs() > 300.0 || (current_pos.y - self.target_position.y).abs() > 100.0 {
            log::warn!(
                "Window position drifted to ({}, {}). Resetting to target position ({}, {})",
                current_pos.x, current_pos.y, self.target_position.x, self.target_position.y
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(self.target_position));
        }

        log::info!("Actual window position: ({}, {})", current_pos.x, current_pos.y);

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

        if self.left_panel_open {
            egui::SidePanel::left("left_panel")
                .min_width(300.0)
                .default_width(350.0)
                .show(ctx, |ui| {
                    draw_left_panel(self, ctx, ui);
                });
        }

        draw_frame_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::top("menu_toggle_bar")
                .show_inside(ui, |bar_ui| {
                    bar_ui.horizontal_centered(|hbar_ui| {
                        let button_text_str = if self.left_panel_open { "☰ Close Tools" } else { "☰ Open Tools" };
                        
                        let rich_button_text = RichText::new(button_text_str)
                            .font(FontId::new(18.0, FontFamily::Proportional));

                        if hbar_ui.button(rich_button_text).clicked() {
                            self.left_panel_open = !self.left_panel_open;
                        }
                    });
                });
            
            draw_canvas(self, ui);
        });

        self.draw_notifications(ctx);
    }
}

impl PaintingApp {
    pub fn save_state_for_undo(&mut self) {
        if let Some(current_original_rect) = self.original_canvas_rect {
            let current_state = self.frames.clone();
            self.undo_history.push((current_state, current_original_rect));
            self.redo_history.clear();

            if self.undo_history.len() > 30 {
                self.undo_history.remove(0);
            }
        } else {
            log::warn!("[Undo] Attempted to save state for undo, but original_canvas_rect is None.");
        }
    }

    pub fn undo(&mut self) {
        if let Some((previous_frames_state, historical_original_rect)) = self.undo_history.pop() {
            if let Some(current_original_rect) = self.original_canvas_rect {
                self.redo_history.push((self.frames.clone(), current_original_rect));

                self.frames = previous_frames_state;

                if let Some(current_canvas_rect_for_drawing) = self.canvas_rect {
                    log::info!(
                        "[Undo] Recalculating strokes from historical_basis: {:?} to current_canvas: {:?}",
                        historical_original_rect,
                        current_canvas_rect_for_drawing
                    );
                    self.recalculate_strokes_relative_to(historical_original_rect, current_canvas_rect_for_drawing);
                    self.original_canvas_rect = Some(current_canvas_rect_for_drawing);
                } else {
                    log::warn!("[Undo] canvas_rect is None during undo. Strokes might be misaligned.");
                    self.original_canvas_rect = Some(historical_original_rect);
                }
            } else {
                 log::warn!("[Undo] original_canvas_rect is None. Cannot properly save current state for redo.");
                 self.undo_history.push((previous_frames_state, historical_original_rect));
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some((next_frames_state, historical_original_rect)) = self.redo_history.pop() {
             if let Some(current_original_rect) = self.original_canvas_rect {
                self.undo_history.push((self.frames.clone(), current_original_rect));

                self.frames = next_frames_state;

                if let Some(current_canvas_rect_for_drawing) = self.canvas_rect {
                     log::info!(
                        "[Redo] Recalculating strokes from historical_basis: {:?} to current_canvas: {:?}",
                        historical_original_rect,
                        current_canvas_rect_for_drawing
                    );
                    self.recalculate_strokes_relative_to(historical_original_rect, current_canvas_rect_for_drawing);
                    self.original_canvas_rect = Some(current_canvas_rect_for_drawing);
                } else {
                    log::warn!("[Redo] canvas_rect is None during redo. Strokes might be misaligned.");
                    self.original_canvas_rect = Some(historical_original_rect);
                }
            } else {
                log::warn!("[Redo] original_canvas_rect is None. Cannot properly save current state for undo.");
                self.redo_history.push((next_frames_state, historical_original_rect));
            }
        }
    }

    pub fn recalculate_strokes_relative_to(&mut self, from_basis: Rect, to_basis: Rect) {
        if from_basis.width() <= 0.0 || from_basis.height() <= 0.0 || to_basis.width() <= 0.0 || to_basis.height() <= 0.0 {
            log::warn!("[Recalculate] Invalid basis rect(s) provided. From: {:?}, To: {:?}. Skipping recalculation.", from_basis, to_basis);
            return;
        }

        for frame_strokes in &mut self.frames {
            for stroke in frame_strokes {
                for point in &mut stroke.points {
                    let rel_x = (point.x - from_basis.min.x) / from_basis.width();
                    let rel_y = (point.y - from_basis.min.y) / from_basis.height();

                    point.x = to_basis.min.x + (rel_x * to_basis.width());
                    point.y = to_basis.min.y + (rel_y * to_basis.height());
                }
            }
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
            
            y_offset += 70.0;
        }
    }
}