use eframe::egui::Color32;

#[derive(Clone)]
pub struct Notification {
    pub id: u64,
    pub message: String,
    pub color: Color32,
    pub created_at: f64,
    pub duration: f64, // in seconds
}