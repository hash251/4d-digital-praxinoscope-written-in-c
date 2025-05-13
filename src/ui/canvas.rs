use eframe::egui::{self, Color32, Pos2, Sense, Stroke as EguiStroke};
use crate::app::{PaintingApp, ToolMode};
use crate::models::{Stroke as DrawingStroke, StrokeType};
use crate::input::TouchState;

pub fn draw_canvas(app: &mut PaintingApp, ui: &mut egui::Ui) {
    let panel_rect = ui.available_rect_before_wrap();

    let calculated_canvas_rect = app.calculate_aspect_ratio_rect(panel_rect);

    if app.canvas_rect.is_none() {
        app.canvas_rect = Some(calculated_canvas_rect);
        app.original_canvas_rect = Some(calculated_canvas_rect);
        log::info!("Canvas Initialized: rect={:?}, original_rect={:?}", app.canvas_rect, app.original_canvas_rect);
    } else {
        let current_basis_rect = app.original_canvas_rect.unwrap();
        if current_basis_rect != calculated_canvas_rect {
            log::info!("Canvas resized. Old basis: {:?}, New target: {:?}", current_basis_rect, calculated_canvas_rect);
            app.recalculate_stroke_positions(calculated_canvas_rect);
            app.original_canvas_rect = Some(calculated_canvas_rect);
            log::info!("Strokes recalculated. New basis: {:?}", app.original_canvas_rect);
        }
        app.canvas_rect = Some(calculated_canvas_rect);
    }
    let current_draw_canvas_rect = app.canvas_rect.unwrap();

    let (_response, painter) = ui.allocate_painter(panel_rect.size(), Sense::click_and_drag());

    ui.painter().rect_filled(panel_rect, 0.0, Color32::DARK_GRAY);
    painter.rect_filled(current_draw_canvas_rect, 0.0, Color32::WHITE);
    painter.rect_stroke(current_draw_canvas_rect, 0.0, EguiStroke::new(1.0, Color32::BLACK));

    if app.show_onion_skin && !app.playing_animation {
        app.draw_onion_skins(&painter);
    }

    for stroke in &app.frames[app.current_frame] {
        app.draw_stroke(&painter, stroke);
    }

    if !app.playing_animation {
        if let Some(input_handler) = &app.input_handler {
            let events = input_handler.get_events();

            for event in events {
                let original_norm_pos = event.pos;

                let screen_rect = ui.ctx().screen_rect();
                let mut final_norm_pos = original_norm_pos;

                let is_portrait = screen_rect.height() > screen_rect.width();
                if is_portrait {
                    final_norm_pos.x = original_norm_pos.y;
                    final_norm_pos.y = 1.0 - original_norm_pos.x;
                }

                let pos_on_screen = Pos2::new(
                    screen_rect.min.x + final_norm_pos.x * screen_rect.width(),
                    screen_rect.min.y + final_norm_pos.y * screen_rect.height(),
                );

                if app.tool_mode == ToolMode::Brush {
                    match event.state {
                        TouchState::Began => {
                            if current_draw_canvas_rect.contains(pos_on_screen) {
                                let stroke = DrawingStroke {
                                    points: vec![pos_on_screen],
                                    color: app.brush_color,
                                    size: app.brush_size,
                                    stroke_type: StrokeType::Draw,
                                };
                                app.active_touches.insert(event.id, stroke);
                            }
                        }
                        TouchState::Moved => {
                            if let Some(stroke) = app.active_touches.get_mut(&event.id) {
                                if current_draw_canvas_rect.contains(pos_on_screen) {
                                    stroke.points.push(pos_on_screen);
                                }
                            }
                        }
                        TouchState::Ended => {
                            if let Some(mut stroke_to_finalize) = app.active_touches.remove(&event.id) {
                                if current_draw_canvas_rect.contains(pos_on_screen) {
                                    if stroke_to_finalize.points.last() != Some(&pos_on_screen) {
                                        stroke_to_finalize.points.push(pos_on_screen);
                                    }
                                }

                                if !stroke_to_finalize.points.is_empty() {
                                    
                                    app.draw_stroke(&painter, &stroke_to_finalize); // FLICKER FIX

                                    app.save_state_for_undo();
                                    app.frames[app.current_frame].push(stroke_to_finalize);
                                }
                            }
                        }
                    }
                }
                else if app.tool_mode == ToolMode::Eraser {
                    if current_draw_canvas_rect.contains(pos_on_screen) {
                        log::trace!("[Canvas] Eraser event (id={}, state={:?}) pos_on_screen=({:.2},{:.2}) inside canvas.", event.id, event.state, pos_on_screen.x, pos_on_screen.y);
                        match event.state {
                            TouchState::Moved | TouchState::Began => {
                                let eraser_size = app.brush_size * 2.0;
                                log::debug!("[Canvas] Eraser Active: pos_on_screen=({:.2},{:.2}), size={}", pos_on_screen.x, pos_on_screen.y, eraser_size);
                                app.erase_strokes_at_position(pos_on_screen, eraser_size);
                            }
                            TouchState::Ended => {
                                log::debug!("[Canvas] Eraser Ended event for id={}", event.id);
                            }
                        }
                        let radius = app.brush_size * 1.0;
                        painter.circle_stroke(pos_on_screen, radius, EguiStroke::new(1.0, Color32::from_rgba_premultiplied(255,0,0,100)));
                    }
                }
            }
        } else { log::trace!("[Canvas] Input handler not available."); }

        for (_id, stroke) in &app.active_touches {
            if app.tool_mode == ToolMode::Brush {
                app.draw_stroke(&painter, stroke);
            }
        }
    }


    if app.playing_animation {
        let text = format!("Playing: Frame {}/{}", app.current_frame + 1, app.frames.len());
        painter.text(
            current_draw_canvas_rect.min + egui::vec2(10.0, 20.0),
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::proportional(16.0),
            Color32::BLACK,
        );
    }
}