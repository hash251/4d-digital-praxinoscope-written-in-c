use eframe::egui::{self, Color32, Pos2, Sense, Stroke};
use crate::app::{PaintingApp, ToolMode};
use crate::models::{Stroke as DrawingStroke, StrokeType};

pub fn draw_canvas(app: &mut PaintingApp, ui: &mut egui::Ui) {
    let panel_rect = ui.max_rect();
    let canvas_rect = app.calculate_aspect_ratio_rect(panel_rect);

    if let Some(old_rect) = app.canvas_rect {
        if old_rect != canvas_rect {
            if app.original_canvas_rect.is_none() {
                app.original_canvas_rect = Some(old_rect);
            }

            app.recalculate_stroke_positions(old_rect);
        }
    } else if app.original_canvas_rect.is_none() {
        app.original_canvas_rect = Some(canvas_rect);
    }

    app.canvas_rect = Some(canvas_rect);

    let (response, painter) = ui.allocate_painter(panel_rect.size(), Sense::click_and_drag());

    ui.painter().rect_filled(panel_rect, 0.0, Color32::DARK_GRAY);
    painter.rect_filled(canvas_rect, 0.0, Color32::WHITE);
    painter.rect_stroke(canvas_rect, 0.0, Stroke::new(1.0, Color32::BLACK));

    if app.show_onion_skin && !app.playing_animation {
        app.draw_onion_skins(&painter);
    }

    for stroke in &app.frames[app.current_frame] {
        app.draw_stroke(&painter, stroke);
    }

    if !app.playing_animation {
        if let Some(pos) = response.interact_pointer_pos() {
            if pos.x >= canvas_rect.min.x
                && pos.x <= canvas_rect.max.x
                && pos.y >= canvas_rect.min.y
                && pos.y <= canvas_rect.max.y
            {
                match app.tool_mode {
                    ToolMode::Brush => {
                        if response.dragged() {
                            if app.current_stroke.is_none() {
                                app.current_stroke = Some(DrawingStroke {
                                    points: vec![pos],
                                    color: app.brush_color,
                                    size: app.brush_size,
                                    stroke_type: StrokeType::Draw,
                                });
                            } else if let Some(stroke) = &mut app.current_stroke {
                                stroke.points.push(pos);
                            }
                        }
                    },
                    ToolMode::Eraser => {
                        if response.dragged() {
                            let eraser_size = app.brush_size * 2.0;
                            app.erase_strokes_at_position(pos, eraser_size);
                        }
                        
                        let radius = app.brush_size * 2.0;
                        painter.circle_stroke(pos, radius, Stroke::new(2.0, Color32::RED));
                    }
                }
            }
        }
    }

    if response.drag_stopped() || (app.current_stroke.is_some() && !response.dragged()) {
        if let Some(stroke) = app.current_stroke.take() {
            if !stroke.points.is_empty() {
                app.save_state_for_undo();
                app.frames[app.current_frame].push(stroke);
            }
        }
    }

    if !app.playing_animation && app.tool_mode == ToolMode::Brush {
        if let Some(stroke) = &app.current_stroke {
            app.draw_stroke(&painter, stroke);
        }
    }

    if app.playing_animation {
        let text = format!(
            "Playing: Frame {} of {}",
            app.current_frame + 1,
            app.frames.len()
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
}