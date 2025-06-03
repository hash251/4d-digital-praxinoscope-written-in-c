import subprocess
import re
import os
import sys
import time
import select

try:
    import evdev
    from evdev import ecodes
except ImportError:
    print("The 'evdev' library is not installed. Please install it:")
    print("  pip install evdev")
    sys.exit(1)

ARTIST_TO_MONITOR_MAPPING = {
    1: "DP-3",
    2: "DP-2", 
    3: "DP-1",
    4: "DP-4"
}

DRAWING_APP_PATH = "/home/softdev/programming/project/target/release/drawing_app_egui"
NEEDS_SUDO_FOR_APP = True
TOUCH_LISTEN_TIMEOUT_SEC = 60

def run_command(cmd, check=True, use_sudo=False, silent=False):
    full_cmd = ["sudo", "-S"] + cmd if use_sudo and os.geteuid() != 0 else cmd
    if not silent:
        print(f"Executing: {' '.join(full_cmd)}")
    try:
        result = subprocess.run(full_cmd, capture_output=True, text=True, check=check)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"Error executing: {' '.join(full_cmd)}")
        if e.stderr:
            print(f"Error: {e.stderr.strip()}")
        if check:
            sys.exit(1)
        return ""

def get_connected_monitors():
    monitors = {}
    try:
        xrandr_output = run_command(["xrandr"], silent=True)
    except:
        print("Error: Cannot run xrandr. Is X11 running?")
        return {}
    
    pattern = re.compile(r"(\S+) connected (?:primary )?(\d+)x(\d+)\+(\d+)\+(\d+)")
    for line in xrandr_output.splitlines():
        match = pattern.search(line)
        if match:
            name, width, height, x_offset, y_offset = match.groups()
            monitors[name] = {
                'width': int(width),
                'height': int(height), 
                'x_offset': int(x_offset),
                'y_offset': int(y_offset)
            }
    return monitors

def get_touchscreen_devices():
    touchscreens = []
    try:
        xinput_output = run_command(["xinput", "list"], silent=True)
    except:
        print("Error: Cannot run xinput")
        return []
    
    device_pattern = re.compile(r".*?([\w\s\-':]+?)\s+id=(\d+)\s+\[slave\s+pointer")
    for line in xinput_output.splitlines():
        match = device_pattern.search(line)
        if match:
            device_name = match.group(1).strip()
            xinput_id = match.group(2)
            try:
                props_output = run_command(["xinput", "list-props", xinput_id], silent=True)
                node_match = re.search(r"Device Node \((\d+)\):\s*\"(/dev/input/event\d+)\"", props_output)
                if node_match:
                    event_node = node_match.group(2)
                    udev_output = run_command(["udevadm", "info", "-q", "property", "-n", event_node], use_sudo=True, silent=True)
                    if "ID_INPUT_TOUCHSCREEN=1" in udev_output or "ID_INPUT_TABLET=1" in udev_output:
                        touchscreens.append({
                            'name': device_name,
                            'xinput_id': xinput_id,
                            'event_node': event_node
                        })
            except:
                continue
    return touchscreens

def wait_for_touch(touchscreens, timeout_sec):
    devices = {}
    fd_to_device = {}
    for ts in touchscreens:
        try:
            if not os.access(ts['event_node'], os.R_OK):
                continue
            device = evdev.InputDevice(ts['event_node'])
            devices[ts['event_node']] = device
            fd_to_device[device.fd] = ts
            while True:
                ready, _, _ = select.select([device.fd], [], [], 0)
                if not ready:
                    break
                for _ in device.read():
                    pass
        except:
            continue

    if not devices:
        print("No accessible touchscreen devices.")
        return None

    print(f"Listening for touch on {len(devices)} device(s)...")
    start_time = time.time()
    try:
        while time.time() - start_time < timeout_sec:
            remaining = timeout_sec - (time.time() - start_time)
            ready_fds, _, _ = select.select(list(fd_to_device.keys()), [], [], min(1.0, remaining))
            for fd in ready_fds:
                if fd in fd_to_device:
                    try:
                        for event in devices[fd_to_device[fd]['event_node']].read():
                            if (event.type == ecodes.EV_KEY and 
                                event.code in [ecodes.BTN_TOUCH, ecodes.BTN_STYLUS, ecodes.BTN_TOOL_PEN] and 
                                event.value == 1):
                                return fd_to_device[fd]
                    except:
                        continue
    finally:
        for device in devices.values():
            try:
                device.close()
            except:
                pass
    return None

def main():
    print("=== Touchscreen to Monitor Mapper ===")
    if not os.path.exists(DRAWING_APP_PATH):
        print(f"Error: Drawing app not found at {DRAWING_APP_PATH}")
        sys.exit(1)

    monitors = get_connected_monitors()
    print(f"Found {len(monitors)} connected monitor(s)")

    missing = [f"Artist {a} -> {ARTIST_TO_MONITOR_MAPPING[a]}" for a in ARTIST_TO_MONITOR_MAPPING if ARTIST_TO_MONITOR_MAPPING[a] not in monitors]
    if missing:
        print("Error: Missing monitors:")
        for m in missing:
            print(f"  {m}")
        print("Available monitors:", list(monitors.keys()))
        sys.exit(1)

    touchscreens = get_touchscreen_devices()
    print(f"Found {len(touchscreens)} touchscreen(s)")
    if not touchscreens:
        sys.exit(1)

    print("\nTouch Mapping Phase: Tap in order of Artist 1–4")

    mappings = []
    available = list(touchscreens)
    for artist in range(1, 5):
        monitor = ARTIST_TO_MONITOR_MAPPING[artist]
        if not available:
            break
        print(f"\nArtist {artist} (Monitor: {monitor}) — Tap now...")
        device = wait_for_touch(available, TOUCH_LISTEN_TIMEOUT_SEC)
        if device:
            print(f"Detected touch from: {device['name']}")
            mappings.append({
                'artist_num': artist,
                'monitor_name': monitor,
                'monitor_info': monitors[monitor],
                'touchscreen': device
            })
            available.remove(device)
        else:
            print(f"No touch detected for Artist {artist}")

    if not mappings:
        print("No mappings created. Exiting.")
        sys.exit(1)

    print("\nFinal Mappings:")
    for m in mappings:
        print(f"  Artist {m['artist_num']}: {m['touchscreen']['name']} -> {m['monitor_name']}")

    print("\nLaunching Applications...")
    launched = []
    for m in mappings:
        try:
            run_command(["xinput", "map-to-output", m['touchscreen']['xinput_id'], m['monitor_name']])
            cmd = [
                os.path.abspath(DRAWING_APP_PATH),
                "--input", m['touchscreen']['event_node'],
                "--x-offset", str(m['monitor_info']['x_offset']),
                "--invert"
            ]
            env = os.environ.copy()
            env["DISPLAY"] = os.environ.get("DISPLAY", ":0")
            env["XAUTHORITY"] = os.environ.get("XAUTHORITY", os.path.expanduser("~/.Xauthority"))
            proc = subprocess.Popen(cmd, env=env)
            launched.append({'artist_num': m['artist_num'], 'process': proc, 'pid': proc.pid})
            print(f"Launched app for Artist {m['artist_num']} (PID: {proc.pid})")
        except Exception as e:
            print(f"Failed to launch for Artist {m['artist_num']}: {e}")

    print("\nWaiting for all applications to close...")
    for app in launched:
        try:
            code = app['process'].wait()
            print(f"Artist {app['artist_num']} app exited (code {code})")
        except Exception as e:
            print(f"Error waiting for Artist {app['artist_num']} app: {e}")

    print("All done.")

if __name__ == "__main__":
    main()
