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
            if !events.is_empty() {
                log::debug!("[Canvas] Received {} touch events from input_handler", events.len());
            }

            for event in events {
                log::debug!("[Canvas] Processing event: id={}, state={:?}, norm_pos=({:.2},{:.2})",
                    event.id, event.state, event.pos.x, event.pos.y);

                let screen_rect = ui.ctx().screen_rect();
                let pos_on_screen = Pos2::new(
                    screen_rect.min.x + event.pos.x * screen_rect.width(),
                    screen_rect.min.y + event.pos.y * screen_rect.height(),
                );
                log::debug!("[Canvas] Event mapped to screen_pos: ({:.2},{:.2}). Canvas rect: {:?}",
                    pos_on_screen.x, pos_on_screen.y, current_draw_canvas_rect);

                if app.tool_mode == ToolMode::Brush {
                    match event.state {
                        TouchState::Began => {
                            if current_draw_canvas_rect.contains(pos_on_screen) {
                                log::debug!("[Canvas] Brush Began: id={}, pos=({:.2},{:.2})", event.id, pos_on_screen.x, pos_on_screen.y);
                                let stroke = DrawingStroke {
                                    points: vec![pos_on_screen],
                                    color: app.brush_color,
                                    size: app.brush_size,
                                    stroke_type: StrokeType::Draw,
                                };
                                app.active_touches.insert(event.id, stroke);
                            } else {
                                log::warn!("[Canvas] Brush Began event (id={}) was outside canvas. Ignoring.", event.id);
                            }
                        }
                        TouchState::Moved => {
                            if let Some(stroke) = app.active_touches.get_mut(&event.id) {
                                if current_draw_canvas_rect.contains(pos_on_screen) {
                                    stroke.points.push(pos_on_screen);
                                    log::trace!("[Canvas] Brush Moved: id={}, point added. New count={}", event.id, stroke.points.len());
                                } else {
                                    log::trace!("[Canvas] Brush Moved: id={}, point outside canvas. Not adding to stroke.", event.id);
                                }
                                let temp_stroke = stroke.clone();
                                app.draw_stroke(&painter, &temp_stroke);
                            } else {
                                if current_draw_canvas_rect.contains(pos_on_screen) {
                                     log::warn!("[Canvas] Brush Moved event for id={} (pos inside canvas) but no active stroke found.", event.id);
                                }
                            }
                        }
                        TouchState::Ended => {
                            log::debug!("[Canvas] Brush Ended event received for id={}. Lift-off screen_pos: ({:.2},{:.2}), norm_pos: ({:.2},{:.2})",
                                event.id, pos_on_screen.x, pos_on_screen.y, event.pos.x, event.pos.y);
                            if let Some(stroke_to_finalize) = app.active_touches.remove(&event.id) {
                                if !stroke_to_finalize.points.is_empty() {
                                    log::info!("[Canvas] Finalizing stroke for id={}, points: {}.", event.id, stroke_to_finalize.points.len());
                                    
                                    app.draw_stroke(&painter, &stroke_to_finalize);

                                    app.save_state_for_undo();
                                    app.frames[app.current_frame].push(stroke_to_finalize);
                                } else {
                                    log::info!("[Canvas] Ended stroke for id={} was empty (no points).", event.id);
                                }
                            } else {
                                 log::warn!("[Canvas] Brush Ended event for id={}, but no active stroke found (or already ended/never started).", event.id);
                            }
                        }
                    }
                }
                else if app.tool_mode == ToolMode::Eraser {
                    if current_draw_canvas_rect.contains(pos_on_screen) {
                        log::trace!("[Canvas] Eraser event (id={}, state={:?}) pos inside canvas.", event.id, event.state);
                        match event.state {
                            TouchState::Moved | TouchState::Began => {
                                let eraser_size = app.brush_size * 2.0;
                                log::debug!("[Canvas] Eraser Active: pos=({:.2},{:.2}), size={}", pos_on_screen.x, pos_on_screen.y, eraser_size);
                                app.erase_strokes_at_position(pos_on_screen, eraser_size);
                            }
                            TouchState::Ended => {
                                // shouldn't matter if the stroke ends
                                log::debug!("[Canvas] Eraser Ended event for id={}", event.id);
                            }
                        }
                        let radius = app.brush_size * 2.0;
                        painter.circle_stroke(pos_on_screen, radius, EguiStroke::new(2.0, Color32::RED));
                    } else {
                        log::warn!("[Canvas] Eraser event (id={}, state={:?}) pos outside canvas. Ignoring action.", event.id, event.state);
                    }
                }
            }
        } else {
            log::trace!("[Canvas] Input handler not available.");
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