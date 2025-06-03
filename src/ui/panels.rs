use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Vec2};
use crate::app::{PaintingApp, ToolMode};

pub fn draw_left_panel(app: &mut PaintingApp, ctx: &egui::Context, panel_ui: &mut egui::Ui) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
        (egui::TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
        (egui::TextStyle::Small, FontId::new(14.0, FontFamily::Proportional)),
    ]
    .into();
    ctx.set_style(style);

    if app.show_admin_panel {
        panel_ui.spacing_mut().item_spacing.y = 8.0;
        panel_ui.spacing_mut().button_padding = Vec2::new(5.0, 3.0);

        panel_ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("⚠ For Administrators Only ⚠")
                    .color(Color32::RED)
                    .strong()
                    .size(16.0),
            );
        });
        panel_ui.add_space(10.0);

        if panel_ui.button("⬅ Back to Tools").clicked() {
            app.show_admin_panel = false;
        }
        panel_ui.separator();
        panel_ui.add_space(10.0);

        panel_ui.heading("Credits");
        panel_ui.label("Joey Malvinni © 2025");
        panel_ui.add_space(15.0);

        panel_ui.heading("Admin Access");
        let base_url = app.local_ip_address.as_deref().unwrap_or("127.0.0.1");
        let admin_url = format!("http://{}:1337/admin", base_url);
        panel_ui.horizontal(|ui| {
            ui.label("Link:");
            ui.hyperlink_to(&admin_url, admin_url.clone());
        });
        panel_ui.small(
            if app.local_ip_address.is_some() {
                ""
            } else {
                "Note: Could not determine local IP."
            }
        );

    } else {
        let larger_font = FontId::new(18.0, FontFamily::Proportional);

        egui::TopBottomPanel::bottom("admin_credits_button_panel_in_left")
            .resizable(false)
            .show_inside(panel_ui, |bottom_ui| {
                bottom_ui.spacing_mut().item_spacing.y = 8.0;
                bottom_ui.spacing_mut().button_padding = Vec2::new(5.0, 3.0);
                bottom_ui.add_space(2.0);
                bottom_ui.horizontal(|ui_button_row| {
                    if ui_button_row.button("⚙").clicked() {
                        app.show_admin_panel = true;
                    }
                });
                bottom_ui.add_space(5.0);
            });

        egui::CentralPanel::default()
            .show_inside(panel_ui, |tools_area_ui| {
                tools_area_ui.spacing_mut().item_spacing.y = 8.0;
                tools_area_ui.spacing_mut().button_padding = Vec2::new(5.0, 3.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(tools_area_ui, |scroll_ui| {
                        scroll_ui.add_space(5.0); 
                        scroll_ui.heading("Tools");
                        scroll_ui.add_space(2.0);

                        scroll_ui.horizontal(|ui| {
                            ui.label("Brush Color:");
                            ui.color_edit_button_srgba(&mut app.brush_color);
                        });

                        scroll_ui.add(egui::Slider::new(&mut app.brush_size, 1.0..=20.0).text("Brush Size"));

                        scroll_ui.horizontal(|ui| {
                            let brush_btn = ui.add(egui::SelectableLabel::new(
                                matches!(app.tool_mode, ToolMode::Brush),
                                RichText::new("Brush").font(larger_font.clone()),
                            ));

                            let eraser_btn = ui.add(egui::SelectableLabel::new(
                                matches!(app.tool_mode, ToolMode::Eraser),
                                RichText::new("Eraser").font(larger_font.clone()),
                            ));

                            if brush_btn.clicked() {
                                app.tool_mode = ToolMode::Brush;
                            }
                            if eraser_btn.clicked() {
                                app.tool_mode = ToolMode::Eraser;
                            }
                        });
                        scroll_ui.add_space(1.0);
                        scroll_ui.separator();
                        scroll_ui.add_space(1.0);

                        scroll_ui.horizontal(|ui| {
                            if ui.button("↩ Undo").clicked() {
                                app.undo();
                            }
                            if ui.button("↪ Redo").clicked() {
                                app.redo();
                            }
                        });

                        scroll_ui.add_space(1.0);
                        scroll_ui.separator();
                        scroll_ui.add_space(1.0);

                        scroll_ui.heading("Animation");
                        scroll_ui.add_space(1.0);

                        scroll_ui.horizontal(|ui| {
                            if ui
                                .button(if app.playing_animation {
                                    "⏹ Stop"
                                } else {
                                    "▶ Play"
                                })
                                .clicked()
                            {
                                app.playing_animation = !app.playing_animation;
                                app.last_frame_time = ctx.input(|i| i.time);
                            }
                        });

                        scroll_ui.add(egui::Slider::new(&mut app.animation_speed, 1.0..=24.0).text("FPS"));

                        scroll_ui.add_space(1.0);
                        scroll_ui.separator();
                        scroll_ui.add_space(1.0);

                        scroll_ui.heading("Onion Skinning");
                        scroll_ui.add_space(1.0);

                        scroll_ui.checkbox(&mut app.show_onion_skin, "Show Onion Skin");
                        scroll_ui.add(egui::Slider::new(&mut app.onion_skin_opacity, 0.0..=1.0).text("Opacity"));

                        scroll_ui.horizontal(|ui| {
                            ui.label("Previous Frame Color:");
                            ui.color_edit_button_srgba(&mut app.prev_onion_color);
                        });

                        scroll_ui.horizontal(|ui| {
                            ui.label("Next Frame Color:");
                            ui.color_edit_button_srgba(&mut app.next_onion_color);
                        });

                        scroll_ui.add_space(1.0);
                        scroll_ui.separator();
                        scroll_ui.add_space(1.0);

                        scroll_ui.heading("Frame Operations");
                        scroll_ui.add_space(1.0);

                        egui::Grid::new("frame_ops_grid")
                            .num_columns(2)
                            .spacing([4.0, 4.0])
                            .show(scroll_ui, |ui| {
                                if ui.button("Clear Frame").clicked() {
                                    app.save_state_for_undo();
                                    app.frames[app.current_frame].clear();
                                }

                                if ui.button("Reset All Frames").clicked() {
                                    app.save_state_for_undo();
                                    app.current_frame = 0;
                                    for frame in &mut app.frames {
                                        frame.clear();
                                    }
                                }
                                ui.end_row();

                                if ui.button("Copy Frame").clicked() {
                                    app.copy_current_frame();
                                }

                                if ui.button("Paste Frame").clicked() {
                                    app.paste_to_current_frame();
                                }
                                ui.end_row();
                            });

                        scroll_ui.add_space(1.0);
                        scroll_ui.separator();
                        scroll_ui.add_space(1.0);

                        let current_time = ctx.input(|i| i.time); 
                        let cooldown_remaining = app.export_cooldown - (current_time - app.last_export_time);
                        let in_cooldown = cooldown_remaining > 0.0;

                        let export_button_text = if in_cooldown {
                            ctx.request_repaint();
                            RichText::new(format!("Export ({:.1}s)", cooldown_remaining))
                                .font(larger_font.clone())
                                .color(Color32::DARK_GRAY)
                        } else {
                            RichText::new("Export").font(larger_font.clone())
                        };

                        if scroll_ui.add_enabled(!in_cooldown, egui::Button::new(export_button_text)).clicked() {
                            app.start_export_animation(ctx);
                        }
                        scroll_ui.add_space(10.0);
                    });
            });
    }
}

pub fn draw_frame_panel(app: &mut PaintingApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("frame_panel").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            for i in 0..app.frames.len() {
                ui.vertical(|ui| {
                    let is_selected = app.current_frame == i;

                    let frame_size = 60.0;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(frame_size, frame_size),
                        egui::Sense::click(),
                    );

                    if response.clicked() {
                        app.current_frame = i;
                        app.playing_animation = false;
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

                    let content_rect = app.calculate_thumbnail_rect(inner_rect.shrink(2.0));
                    app.draw_thumbnail_content(i, ui.painter(), content_rect);

                    ui.label(format!("Frame {}", i + 1));
                });
            }
        });
    });
}