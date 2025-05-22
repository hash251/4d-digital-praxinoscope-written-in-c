xinput map-to-output 8 DP-4
xinput map-to-output 10 DP-2
xinput map-to-output 14 DP-1
xinput map-to-output 16 DP-3

sudo ./target/debug/drawing_app_egui --monitor 1 --input /dev/input/by-id/usb-Elo_Touchscreen_4-event
sudo ./target/debug/drawing_app_egui --monitor 2 --input /dev/input/by-id/usb-Elo_Touchscreen_2-event
sudo ./target/debug/drawing_app_egui --monitor 3 --input /dev/input/by-id/usb-Elo_Touchscreen_3-event
sudo ./target/debug/drawing_app_egui --monitor 4 --input /dev/input/by-id/usb-Elo_Touchscreen_1-event