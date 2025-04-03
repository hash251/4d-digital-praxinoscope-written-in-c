# todo - server
- [ ] image queue system, could just be time-reliant for now
- [ ] communicate with display clients -- need to keep track of which IP corresponds to the correct relative image number

# todo - drawing app
- [ ] **notification system**
- [ ] fix flickering after drawing stroke
- [ ] make left panel fixed width to fix resizing problems
- [ ] exporting progress updates
- [ ] 16:9 resolution canvas [exporting]
- [ ] exporting cooldown -> notifications


# todo - display client
- [ ] Eventually needs to know which monitor relatively corresponds to it's real life position, and be able to offset this with the pi number -> multible Pis

# done
## drawing app
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

## server 
- [x] implemented websockets and POST requests for images

## display client
- [x] add a next-image element which caches the next image and shows it 
- [x] add a queue