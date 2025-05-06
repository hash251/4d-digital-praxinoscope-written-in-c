use eframe::egui::{Color32, Pos2};

#[derive(Clone, PartialEq)]
pub enum StrokeType {
    Draw,
    // Fill 
}

impl Default for StrokeType {
    fn default() -> Self {
        StrokeType::Draw
    }
}

#[derive(Clone)]
pub struct Stroke {
    pub points: Vec<Pos2>,
    pub color: Color32,
    pub size: f32,
    pub stroke_type: StrokeType,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            color: Color32::BLACK,
            size: 1.0,
            stroke_type: StrokeType::Draw,
        }
    }
}