#!/bin/bash

# Store PIDs of all background processes
PIDS=()

# Function to kill all child processes on Ctrl+C
cleanup() {
    echo "Stopping all processes..."
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null
    done
    exit 0
}

# Trap Ctrl+C (SIGINT)
trap cleanup SIGINT

# Start each process in the background and save its PID
sudo ./target/debug/drawing_app_egui --input /dev/input/by-id/usb-Elo_Touchscreen_3-event --monitor 0 --invert &
PIDS+=($!)

sudo ./target/debug/drawing_app_egui --input /dev/input/by-id/usb-Elo_Touchscreen_2-event --monitor 1  --invert &
PIDS+=($!)

sudo ./target/debug/drawing_app_egui --input /dev/input/by-id/usb-Elo_Touchscreen_4-event --monitor 2  --invert &
PIDS+=($!)

sudo ./target/debug/drawing_app_egui --input /dev/input/by-id/usb-Elo_Touchscreen_1-event --monitor 3 --invert &
PIDS+=($!)

wait




wait
