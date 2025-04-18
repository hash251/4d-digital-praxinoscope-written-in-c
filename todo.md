# todo - server
- [x] communicate with multiple display clients
- [x] implemented websockets and POST requests for images
- [x] image queue system, could just be time-reliant for now

# todo - display client
- [ ] fullscreen main monitor
- [ ] handle websocket errors when the server is offline
- [x] store variable to keep track of monitor index
- [x] add a next-image element which caches the next image and shows it 
- [x] add a queue

# todo - drawing app
- [ ] **modularization**
- [ ] add an exporting cooldown
- [ ] fix flickering after drawing stroke
- [ ] make left panel fixed width to fix resizing problems
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
