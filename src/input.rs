use evdev::{AbsoluteAxisCode, Device, EventType};
use eframe::egui::Pos2;
use std::path::Path;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::error::Error;

pub struct TouchInput {
    pub id: u32,
    pub pos: Pos2,
    pub state: TouchState,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TouchState {
    Began,
    Moved,
    Ended,
}

pub struct InputHandler {
    event_receiver: Receiver<TouchInput>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl InputHandler {
    pub fn new(device_path: &str) -> Result<Self, Box<dyn Error>> {
        log::info!("[InputHandler] Attempting to open device: {}", device_path);
        let device = Device::open(Path::new(device_path))?;
        log::info!("[InputHandler] Device opened successfully: {}", device.name().unwrap_or("Unknown Device"));

        let (sender, receiver) = channel();

        let thread_handle = thread::spawn(move || {
            if let Err(e) = Self::read_events(device, sender) {
                log::error!("[InputHandler] Event thread error: {}", e);
            }
        });

        Ok(Self {
            event_receiver: receiver,
            thread_handle: Some(thread_handle),
        })
    }

    fn read_events(mut device: Device, sender: Sender<TouchInput>) -> Result<(), Box<dyn Error>> {
        let mut current_touch_id: Option<u32> = None;
        let mut current_pos = Pos2::new(0.0, 0.0);
        let mut new_touch_began_pending_for_id: Option<u32> = None; // Stores ID if a Began event needs to be sent on next SYN

        let mut min_x = 0;
        let mut max_x = 1;
        let mut min_y = 0;
        let mut max_y = 1;

        match device.get_absinfo() {
            Ok(abs_iter) => {
                for (axis_code, info) in abs_iter {
                    match axis_code {
                        AbsoluteAxisCode::ABS_MT_POSITION_X => {
                            min_x = info.minimum();
                            max_x = info.maximum();
                            log::info!("[InputHandler] Found ABS_MT_POSITION_X: min={}, max={}", min_x, max_x);
                        }
                        AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                            min_y = info.minimum();
                            max_y = info.maximum();
                            log::info!("[InputHandler] Found ABS_MT_POSITION_Y: min={}, max={}", min_y, max_y);
                        }
                        _ => { /* log::debug!("[InputHandler] Other axis: {:?}, info: {:?}", axis_code, info); */ }
                    }
                }
            }
            Err(e) => {
                log::error!("[InputHandler] Failed to get absinfo: {}. Using default ranges 0-1.", e);
            }
        }
        
        if max_x <= min_x {
            log::warn!("[InputHandler] X_MT axis range invalid or not properly read (min: {}, max: {}). Defaulting to 0-1 for normalization.", min_x, max_x);
            min_x = 0; max_x = 1;
        }
        if max_y <= min_y {
            log::warn!("[InputHandler] Y_MT axis range invalid or not properly read (min: {}, max: {}). Defaulting to 0-1 for normalization.", min_y, max_y);
            min_y = 0; max_y = 1;
        }
        log::info!("[InputHandler] Using Normalization Ranges: X({}-{}), Y({}-{})", min_x, max_x, min_y, max_y);


        loop {
            for event in device.fetch_events()? {
                match event.event_type() {
                    EventType::ABSOLUTE => {
                        match AbsoluteAxisCode(event.code()) {
                            AbsoluteAxisCode::ABS_MT_TRACKING_ID => {
                                let id_val = event.value();
                                if id_val == -1 || id_val as u32 == u32::MAX { 
                                    if let Some(id) = current_touch_id {
                                        if new_touch_began_pending_for_id == Some(id) {
                                            log::debug!("[InputHandler] Touch Ended for id={} which had a Began pending. Cancelling Began.", id);
                                            new_touch_began_pending_for_id = None;
                                        }
                                        log::debug!("[InputHandler] Touch Ended by TRACKING_ID: id={}, pos=({:.2},{:.2})", id, current_pos.x, current_pos.y);
                                        sender.send(TouchInput {
                                            id,
                                            pos: current_pos,
                                            state: TouchState::Ended,
                                        }).unwrap_or_else(|e| log::error!("Failed to send TouchEnded event: {}",e));
                                        current_touch_id = None;
                                    } else {
                                        log::warn!("[InputHandler] Received TRACKING_ID -1 but no current_touch_id was set.");
                                    }
                                } else {
                                    let new_id = id_val as u32;
                                    if let Some(old_id) = current_touch_id {
                                        if old_id != new_id {
                                            log::warn!("[InputHandler] New TRACKING_ID {} received while {} was active. Implicitly ending {}.", new_id, old_id, old_id);
                                            sender.send(TouchInput {
                                                id: old_id,
                                                pos: current_pos,
                                                state: TouchState::Ended,
                                            }).unwrap_or_else(|e| log::error!("Failed to send implicit TouchEnded for old_id {}: {}", old_id, e));
                                        }
                                    }
                                    
                                    current_touch_id = Some(new_id);
                                    new_touch_began_pending_for_id = Some(new_id);
                                    log::debug!("[InputHandler] New TRACKING_ID detected: {}. Began event pending.", new_id);
                                    // DO NOT send Began event yet (wait for SYN_REPORT with updated coords) 
                                }
                            }
                            AbsoluteAxisCode::ABS_MT_POSITION_X => {
                                let raw_x = event.value();
                                if max_x > min_x { // div by 0
                                    current_pos.x = (raw_x - min_x) as f32 / (max_x - min_x) as f32;
                                } else {
                                    current_pos.x = 0.0;
                                }
                            }
                            AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                                let raw_y = event.value();
                                if max_y > min_y {
                                    current_pos.y = (raw_y - min_y) as f32 / (max_y - min_y) as f32;
                                } else {
                                    current_pos.y = 0.0;
                                }
                            }
                            _ => {}
                        }
                    }
                    EventType::SYNCHRONIZATION => {
                        if let Some(id) = current_touch_id {
                            if new_touch_began_pending_for_id == Some(id) {
                                log::debug!("[InputHandler] Sending Pending Touch Began (SYN): id={}, pos=({:.2},{:.2})", id, current_pos.x, current_pos.y);
                                sender.send(TouchInput {
                                    id,
                                    pos: current_pos,
                                    state: TouchState::Began,
                                }).unwrap_or_else(|e| log::error!("Failed to send TouchBegan event: {}",e));
                                new_touch_began_pending_for_id = None; // Clear the pending flag

                                log::debug!("[InputHandler] Sending Initial Touch Moved (SYN): id={}, pos=({:.2},{:.2})", id, current_pos.x, current_pos.y);
                                sender.send(TouchInput {
                                    id,
                                    pos: current_pos,
                                    state: TouchState::Moved,
                                }).unwrap_or_else(|e| log::error!("Failed to send initial TouchMoved event: {}",e));
                            } else {
                                log::trace!("[InputHandler] Touch Moved (SYN): id={}, pos=({:.2},{:.2})", id, current_pos.x, current_pos.y);
                                sender.send(TouchInput {
                                    id,
                                    pos: current_pos,
                                    state: TouchState::Moved,
                                }).unwrap_or_else(|e| log::error!("Failed to send TouchMoved event: {}",e));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn get_events(&self) -> Vec<TouchInput> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Drop for InputHandler {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            drop(handle);
        }
    }
}