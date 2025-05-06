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

Start drawing clients:
```
./sync.sh
```
## Architecture

Drawing Client:
- Rust drawing app written with `egui`/`eframe`, exporting images is done with reqwests for http requests alongside tiny-skia for the final drawing render.

Server:
- Create the server with Express
- Gets the images from clients with POST requests

Display Tablets:
- Communicate with websockets to receive the images
- Use a display which fetches the images from the main server and displays them on an Electron GUI.