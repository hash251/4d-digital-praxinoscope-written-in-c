# 4d Digital Praxinoscope Written in C
Funny drawing app project

## Usage

Start Node.js server:
```
npm start
```

Start Rust drawing apps:
```
cargo run --release
```

Sync new code to drawing clients:
```
./sync.sh
```

## Some Notes

### Starting Apps on Login

There are two systemctl services which allow the computer to run the apps on startup. These are located in `/etc/systemd/system/npm-start.service` and `/etc/systemd/system/python-start.service`.

The `npm-start` services runs the `server.sh` bash script, which simply runs the backend server, while the `python-start` services starts the Python monitor mapping script in `start.py`.
If you have any issues, check the logs like so:

```
journalctl -u npm-start -b # or python-start
```


### Disabling Touch Gestures
A common issue with the drawing app setup is that swiping with 3 fingers will result in the app panel opening and an exploded view of the open apps on the computer, similar to pressing the Super key on the computer. I installed a gnome-extension to disable these gestures.

```bash
~ » gnome-extensions enable disable-gestures-2021@verycrazydog.gmail.com
```
This didn't actually work, for reasons untold. This implementation is left as an exercise to future years. (In particular, it seemed to disable touch events).
