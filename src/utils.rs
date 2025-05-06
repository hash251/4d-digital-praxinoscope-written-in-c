use eframe::egui::Pos2;

pub fn distance_to_line_segment(p: Pos2, v: Pos2, w: Pos2) -> f32 {
    let l2 = v.distance_sq(w);

    if l2 == 0.0 {
        return p.distance(v);
    }

    let t = ((p - v).dot(w - v) / l2).clamp(0.0, 1.0);
    let projection = v + t * (w - v);

    p.distance(projection)
}