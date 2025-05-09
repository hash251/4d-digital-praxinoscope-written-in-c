
## server
- [ ] dynamically generate qr code for admin panel
- [x] queue management with the admin panel
- [x] communicate with multiple display clients
- [x] implemented websockets and POST requests for images
- [x] image queue system, could just be time-reliant for now

## display client
- [x] fullscreen main monitor
- [x] handle websocket errors when the server is offline
- [x] store variable to keep track of monitor index
- [x] add a next-image element which caches the next image and shows it 
- [x] add a queue


## drawing app
- [ ] **custom touchscreen touch event handling with /dev/input devices**
    - [ ] map /dev/input/by-id to each respective monitor (hard code?)
    - [ ] write an api to expose events to each specific window on the touchscreen devices

- [ ] **fix flickering after drawing stroke**
- [ ] make left panel fixed width to fix resizing problems
- [ ] fill tool
- [x] add an exporting cooldown
- [x] **modularization**
- [x] exporting progress updates
- [x] live stroke display
- [x] frames represented as indexes
- [x] copying and pasting frames
- [x] onion skinning opacity from previous and next frames
- [x] position of strokes recalculate correctly
- [x] add undo and redo buttons
- [x] exporting and uploading over POST to the server
- [x] thumbnails for frames
- [x] clear all frames button
- [x] make buttons bigger
- [x] **notification system**

## admin panel
- [x] implement frontend admin panel