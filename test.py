#!/usr/bin/env python3

import subprocess
import re
import os
import sys
import time
import shutil 
import select

try:
    import evdev
    from evdev import ecodes
except ImportError:
    print("The 'evdev' library is not installed. Please install it:")
    print("  pip install evdev")
    print("Or for your system's package manager (e.g., on Debian/Ubuntu):")
    print("  sudo apt install python3-evdev")
    sys.exit(1)

DRAWING_APP_PATH = "./target/debug/drawing_app_egui"
NEEDS_SUDO_FOR_APP = True
NEEDS_SUDO_FOR_EVDEV_LIB = True
NEEDS_SUDO_FOR_UDEVADM = True

HIGHLIGHT_GAMMA = "1.0:0.2:0.2"
RESTORE_GAMMA = "1.0:1.0:1.0"
HIGHLIGHT_DURATION_SEC = 2.5

def run_command(cmd, check=True, use_sudo=False, no_output_on_success=False):
    full_cmd = []
    actual_cmd_to_check = cmd[0]
    if use_sudo and os.geteuid() != 0:
        full_cmd.append("sudo")
        if not shutil.which("sudo"):
            print("Error: 'sudo' is required but not found in PATH.")
            sys.exit(1)
    full_cmd.extend(cmd)

    if not shutil.which(actual_cmd_to_check):
        print(f"Error: Command '{actual_cmd_to_check}' not found in PATH. Please ensure it's installed.")
        sys.exit(1)

    if not no_output_on_success:
        print(f"Executing: {' '.join(full_cmd)}")

    try:
        result = subprocess.run(full_cmd, capture_output=True, text=True, check=check)
        return result.stdout.strip()
    except FileNotFoundError:
        print(f"Error: Command '{full_cmd[0]}' (or a component) not found. Please ensure it's installed and in your PATH.")
        sys.exit(1)
    except subprocess.CalledProcessError as e:
        print(f"Error executing command: {' '.join(full_cmd)}")
        print(f"Return code: {e.returncode}")
        print(f"Stdout: {e.stdout}")
        print(f"Stderr: {e.stderr}")
        if check:
            sys.exit(1)
        return e.stdout.strip()

def get_monitors_from_xrandr():
    monitors = []
    try:
        xrandr_output = run_command(["xrandr"])
    except Exception:
        print("Error running xrandr. Is X11 running and xrandr installed?")
        return []

    monitor_pattern = re.compile(r"(\S+) connected (?:primary )?(\d+)x(\d+)\+(\d+)\+(\d+)")
    for line in xrandr_output.splitlines():
        match = monitor_pattern.search(line)
        if match:
            system_name, width, height, x_offset, y_offset = match.groups()
            monitors.append({
                'system_name': system_name,
                'resolution': f"{width}x{height}",
                'width': int(width),
                'height': int(height),
                'x_offset': int(x_offset),
                'y_offset': int(y_offset),
            })
    if not monitors:
        print("No connected monitors found by xrandr.")
    return monitors

def highlight_monitor(monitor_system_name, duration_sec):
    print(f"  Highlighting '{monitor_system_name}' (changing gamma to red)...")
    highlighted = False
    try:
        run_command(["xrandr", "--output", monitor_system_name, "--gamma", HIGHLIGHT_GAMMA], no_output_on_success=True)
        highlighted = True
        time.sleep(duration_sec)
    except Exception as e:
        print(f"    Error changing gamma for {monitor_system_name}: {e}")
        return False
    finally:
        if highlighted:
            try:
                run_command(["xrandr", "--output", monitor_system_name, "--gamma", RESTORE_GAMMA], no_output_on_success=True)
                print(f"  '{monitor_system_name}' highlight finished.")
            except Exception as e:
                print(f"    Error restoring gamma for {monitor_system_name}: {e}")
    return True

def get_potential_touchscreens():
    touchscreens = []
    try:
        xinput_list_output = run_command(["xinput", "list"])
    except Exception:
        print("Error running xinput (is it installed?)")
        return []

    device_pattern = re.compile(r".*?([\w\s\-':]+?)\s+id=(\d+)\s+\[slave\s+pointer")
    for line in xinput_list_output.splitlines():
        match = device_pattern.search(line)
        if match:
            device_name = match.group(1).strip().replace("\\t", "").replace("\t", "")
            xinput_id = match.group(2)

            try:
                props_output = run_command(["xinput", "list-props", xinput_id])
            except Exception:
                continue

            node_match = re.search(r"Device Node \((\d+)\):\s*\"(/dev/input/event\d+)\"", props_output)
            if node_match:
                event_node = node_match.group(2)
                try:
                    udev_cmd = ["udevadm", "info", "-q", "property", "-n", event_node]
                    udev_output = run_command(udev_cmd, use_sudo=NEEDS_SUDO_FOR_UDEVADM)

                    if "ID_INPUT_TOUCHSCREEN=1" in udev_output or "ID_INPUT_TABLET=1" in udev_output:
                         touchscreens.append({
                            'xinput_id': xinput_id,
                            'name': device_name,
                            'event_node': event_node,
                            'identifier_string': f"{device_name} (id: {xinput_id}, node: {event_node})"
                        })
                except Exception as e:
                    print(f"udevadm query failed for {event_node}. Error: {e}")
    if not touchscreens:
        print("No potential touch input devices found via xinput and udevadm.")
    return touchscreens

def user_select_item_from_list(items_with_identifiers, item_type_name, prompt_message, allow_skip=False):
    if not items_with_identifiers:
        print(f"No {item_type_name}s available to select.")
        return None
    print(f"\n{prompt_message}")
    for i, item in enumerate(items_with_identifiers):
        print(f"  {i+1}: {item['identifier_string']}")

    skip_option = " (or 0 to skip)" if allow_skip else ""
    while True:
        try:
            choice_str = input(f"Select {item_type_name} by number{skip_option}: ")
            choice_idx = int(choice_str)
            if allow_skip and choice_idx == 0:
                return None
            if 1 <= choice_idx <= len(items_with_identifiers):
                return items_with_identifiers[choice_idx - 1]
            else:
                print("Invalid choice. Please try again.")
        except ValueError:
            print("Please enter a number.")

def wait_for_touch_on_device(event_node_path, device_display_name):
    print(f"\n\n>>> Please touch the physical screen associated with '{device_display_name}' now <<<")
    print(f"    (Listening for any touch/stylus input on {event_node_path})")

    if not os.path.exists(event_node_path):
        print(f"Error: Device path {event_node_path} does not exist.")
        return False
    if not os.access(event_node_path, os.R_OK):
        print(f"Read permission denied for {event_node_path}.")
        if os.geteuid() != 0 and NEEDS_SUDO_FOR_EVDEV_LIB:
            print("  This script might need to be run with 'sudo' for evdev access,")
            print(f"  or your user '{os.getlogin()}' needs to be in the 'input' group.")
        return False

    try:
        device = evdev.InputDevice(event_node_path)
        while True:
            r, _, _ = select.select([device.fd], [], [], 0)
            if not r:
                break
            for event in device.read():
                pass
        print("    Device buffer cleared. Ready for new touch.")

        for event in device.read_loop():
            if event.type == ecodes.EV_KEY and \
               (event.code == ecodes.BTN_TOUCH or \
                event.code == ecodes.BTN_STYLUS or \
                event.code == ecodes.BTN_TOOL_PEN) and \
               event.value == 1:
                print(f"    Touch/Stylus press detected on {event_node_path}!")
                return True
            elif event.type == ecodes.EV_ABS and \
                 (event.code == ecodes.ABS_X or event.code == ecodes.ABS_MT_POSITION_X or \
                  event.code == ecodes.ABS_Y or event.code == ecodes.ABS_MT_POSITION_Y):
                 active_keys = device.active_keys()
                 if ecodes.BTN_TOUCH in active_keys or \
                    ecodes.BTN_STYLUS in active_keys or \
                    ecodes.BTN_TOOL_PEN in active_keys:
                    print(f"    Movement while touch/stylus active on {event_node_path}!")
                    return True
            time.sleep(0.01)
    except FileNotFoundError:
        print(f"Error: Device {event_node_path} not found (disconnected?).")
    except PermissionError:
        print(f"Error: Permission denied for {event_node_path}.")
        if os.geteuid() != 0 and NEEDS_SUDO_FOR_EVDEV_LIB:
             print("  Try running with 'sudo' or add user to 'input' group.")
    except OSError as e:
        if e.errno == 19:
            print(f"Error: Device {event_node_path} no longer available (ENODEV).")
        elif e.errno == 11:
            print(f"    Device {event_node_path} busy or not ready. Please try touching again.")
        else:
            print(f"An OS error occurred while listening to {event_node_path}: {e}")
    except Exception as e:
        print(f"An unexpected error occurred while listening to {event_node_path}: {e}")
    return False

def main():
    print("--- Interactive Touchscreen to Monitor Mapper ---")

    if os.geteuid() != 0 and (NEEDS_SUDO_FOR_APP or NEEDS_SUDO_FOR_UDEVADM or NEEDS_SUDO_FOR_EVDEV_LIB):
        print("\nThis script performs operations that may require root privileges (sudo).")
        print("If you encounter permission errors, try running the script with 'sudo'.")

    raw_monitors = get_monitors_from_xrandr()
    if not raw_monitors:
        sys.exit(1)

    print("\n--- Step 1: Monitor Identification & Artist Assignment ---")
    print("We will highlight your physical monitors one by one. For each, you will assign an 'Artist Number'.")

    identified_monitors = []
    available_artist_numbers = list(range(1, len(raw_monitors) + 1))

    for i, mon_data in enumerate(raw_monitors):
        sys_name = mon_data['system_name']
        base_id_string = f"{sys_name} ({mon_data['resolution']} at +{mon_data['x_offset']},{mon_data['y_offset']})"

        print(f"\nIdentifying monitor {i+1}/{len(raw_monitors)}: System Name '{sys_name}'")
        input(f"  Press Enter when you are ready to see '{sys_name}' highlighted...")

        if not highlight_monitor(sys_name, HIGHLIGHT_DURATION_SEC):
            print(f"  Could not visually identify '{sys_name}'. This monitor will be skipped.")
            continue

        print(f"  Monitor '{sys_name}' ({base_id_string}) was just highlighted.")

        if not available_artist_numbers:
            print("  Error: No more artist numbers available. This shouldn't happen if highlighting worked for all.")
            break

        while True:
            options_display_list = [str(num) for num in available_artist_numbers]
            print(f"  Available Artist Numbers to assign: {', '.join(options_display_list)}")
            choice_str = input(f"  Assign an Artist Number to the highlighted monitor '{sys_name}': ").strip()
            try:
                chosen_artist_num = int(choice_str)
                if chosen_artist_num in available_artist_numbers:
                    mon_data_copy = mon_data.copy()
                    mon_data_copy['artist_number'] = chosen_artist_num
                    mon_data_copy['user_given_name'] = f"Artist {chosen_artist_num}"
                    mon_data_copy['identifier_string'] = f"Artist {chosen_artist_num} [{base_id_string}]"
                    identified_monitors.append(mon_data_copy)
                    available_artist_numbers.remove(chosen_artist_num)
                    print(f"  Monitor '{sys_name}' is now assigned as 'Artist {chosen_artist_num}'.")
                    break
                else:
                    print(f"  Invalid selection. Choose from: {', '.join(options_display_list)}.")
            except ValueError:
                print("  Invalid input. Please enter a number.")

    if not identified_monitors:
        print("\nNo monitors were successfully identified. Exiting.")
        sys.exit(1)

    identified_monitors.sort(key=lambda m: m['artist_number'])
    print("\n--- Monitor Identification Summary ---")
    for mon in identified_monitors:
        print(f"  - {mon['identifier_string']} (System Name: {mon['system_name']})")

    all_touchscreens = get_potential_touchscreens()
    if not all_touchscreens:
        sys.exit(1)

    print("\n--- Step 2: Touchscreen to Monitor Association ---")
    final_mappings = []
    auto_select_monitor = None
    if len(identified_monitors) == 1:
        auto_select_monitor = identified_monitors[0]
        print(f"Only one monitor ('{auto_select_monitor['user_given_name']}') identified. Will auto-select.")

    for i, ts_device in enumerate(all_touchscreens):
        print(f"\n--- Identifying Touchscreen {i+1}/{len(all_touchscreens)}: {ts_device['identifier_string']} ---")
        if wait_for_touch_on_device(ts_device['event_node'], ts_device['name']):
            selected_monitor = None
            if auto_select_monitor:
                print(f"Touch detected from '{ts_device['name']}'. Auto-selecting monitor '{auto_select_monitor['user_given_name']}'.")
                selected_monitor = auto_select_monitor
            else:
                selected_monitor = user_select_item_from_list(
                    identified_monitors,
                    "monitor",
                    f"Touch detected from '{ts_device['name']}'.\nWhich Artist Monitor did you just touch?"
                )
            if selected_monitor:
                final_mappings.append({'touchscreen': ts_device, 'monitor': selected_monitor})
                print(f"  Associated '{ts_device['name']}' with monitor '{selected_monitor['user_given_name']}'.")
            else:
                print(f"  No monitor selected for '{ts_device['name']}'. Skipping.")
        else:
            print(f"  No touch detected (or error) for '{ts_device['name']}'. Skipping.")

    if not final_mappings:
        print("\nNo touchscreens were mapped. Exiting.")
        sys.exit(0)

    print("\n--- Step 3: Applying X11 Mappings and Launching Drawing Applications ---")
    if not os.path.exists(DRAWING_APP_PATH):
        abs_path = os.path.abspath(DRAWING_APP_PATH)
        print(f"Error: Drawing application not found at '{DRAWING_APP_PATH}' (resolved to '{abs_path}')")
        sys.exit(1)
    if not os.access(DRAWING_APP_PATH, os.X_OK):
        print(f"Error: Drawing application at '{DRAWING_APP_PATH}' is not executable (try chmod +x).")
        sys.exit(1)

    launched_processes_info = []

    for mapping_idx, mapping in enumerate(final_mappings):
        ts = mapping['touchscreen']
        mon = mapping['monitor']

        print(f"\nConfiguring for: Touchscreen '{ts['name']}' on Monitor '{mon['user_given_name']}' (System: {mon['system_name']})")

        print(f"  Mapping '{ts['name']}' (xinput ID: {ts['xinput_id']}) to monitor output '{mon['system_name']}'...")
        map_cmd = ["xinput", "map-to-output", ts['xinput_id'], mon['system_name']]
        run_command(map_cmd, check=True)
        print("  X11 mapping successful.")

        app_cmd_list_base = [
            os.path.abspath(DRAWING_APP_PATH),
            "--input", ts['event_node'],
            "--x-offset", str(mon['x_offset']),
            "--invert"
        ]
        
        full_app_cmd = []
        resolved_app_path = shutil.which(app_cmd_list_base[0]) or app_cmd_list_base[0]

        if NEEDS_SUDO_FOR_APP and os.geteuid() != 0:
            if not shutil.which("sudo"):
                print("Error: 'sudo' is required for the app but not found. Cannot launch.")
                continue
            full_app_cmd.append("sudo")
            full_app_cmd.append(resolved_app_path)
            full_app_cmd.extend(app_cmd_list_base[1:])
        else:
            full_app_cmd.append(resolved_app_path)
            full_app_cmd.extend(app_cmd_list_base[1:])


        print(f"  Preparing to launch drawing app: {' '.join(full_app_cmd)}")

        try:
            process = subprocess.Popen(full_app_cmd)
            launched_processes_info.append({
                'process': process,
                'ts_name': ts['name'],
                'mon_name': mon['user_given_name'],
                'pid': process.pid
            })
            print(f"  Launched app for '{ts['name']}' on '{mon['user_given_name']}' (PID: {process.pid}).")
        except FileNotFoundError:
            cmd_failed = full_app_cmd[0]
            print(f"    Error: Command '{cmd_failed}' not found when trying to launch the drawing app. Please check the path and permissions.")
        except PermissionError:
            print(f"    Error: Permission denied when trying to launch '{' '.join(full_app_cmd)}'.")
            if NEEDS_SUDO_FOR_APP and os.geteuid() != 0 and "sudo" not in full_app_cmd :
                print(f"    The application might require 'sudo' and it wasn't used, or file permissions are incorrect.")
        except Exception as e:
            print(f"    An unexpected error occurred while launching the drawing app for '{ts['name']}': {e}")

    if not launched_processes_info:
        print("\nNo drawing applications were successfully launched.")
        sys.exit(0)

    print(f"\n--- {len(launched_processes_info)} Drawing Application(s) Launched ---")
    print("The applications are now running. You can interact with them.")
    print("This script will now wait until all launched drawing applications are closed.")
    print("Close each drawing application window when you are finished with it.")

    for app_info in launched_processes_info:
        process = app_info['process']
        ts_name = app_info['ts_name']
        mon_name = app_info['mon_name']
        pid = app_info['pid']
        try:
            return_code = process.wait()
            if return_code == 0:
                print(f"  Application for '{ts_name}' on '{mon_name}' (PID: {pid}) closed normally.")
            else:
                print(f"  Application for '{ts_name}' on '{mon_name}' (PID: {pid}) closed with exit code {return_code}.")
        except Exception as e: 
            print(f"  Error waiting for application '{ts_name}' on '{mon_name}' (PID: {pid}): {e}")

    print("\n--- All drawing applications have been closed. Script finished. ---")

if __name__ == "__main__":
    main()