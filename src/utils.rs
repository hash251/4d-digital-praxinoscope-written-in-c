use eframe::egui::Pos2;
use std::net::UdpSocket;

pub fn distance_to_line_segment(p: Pos2, v: Pos2, w: Pos2) -> f32 {
    let l2 = v.distance_sq(w);

    if l2 == 0.0 {
        return p.distance(v);
    }

    let t = ((p - v).dot(w - v) / l2).clamp(0.0, 1.0);
    let projection = v + t * (w - v);

    p.distance(projection)
}

pub fn get_local_ip_address() -> Option<String> {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            match socket.connect("8.8.8.8:80") {
                Ok(_) => {
                    match socket.local_addr() {
                        Ok(local_addr) => {
                            log::info!("Successfully determined local IP: {}", local_addr.ip());
                            Some(local_addr.ip().to_string())
                        },
                        Err(e) => {
                            log::error!("Failed to get local_addr for IP discovery: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to connect UDP socket for IP discovery (network may be unreachable or DNS blocked): {}. Consider using a local router IP for connect.", e);
                    None
                }
            }
        }
        Err(e) => {
            log::error!("Failed to bind UDP socket for IP discovery: {}", e);
            None
        }
    }
}