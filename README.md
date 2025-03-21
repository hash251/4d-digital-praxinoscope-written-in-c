# 4d Digital Praxinoscope Written in C
Funny drawing app project


## Todo
- **Figure out how managing images will work**
- Potentially add a screen to view the queue and remove people from queue if necessary
- Enforce a cooldown to the export button on the drawing client

## Architecture

Drawing Client:
- Rust drawing app written with `egui`/`eframe`, exporting images is done with reqwests for http requests alongside tiny-skia for the final drawing render.

Server:
- Create the server with Express
- Gets the images from clients with POST requests

Display Tablets:
- Communicate with websockets to receive the images
- Use a display which fetches the images from the main server and displays them on an Electron GUI.
