const express = require('express');
const multer = require('multer');
const http = require('http');
const WebSocket = require('ws');
const path = require('path');
const fs = require('fs');
const crypto = require('crypto');

const app = express();
const PORT = 1337;
const DISPLAY_TIME = 20 * 1000;
const server = http.createServer(app);
const wss = new WebSocket.Server({ noServer: true });
const adminWss = new WebSocket.Server({ noServer: true });

const uploadsDir = path.join(__dirname, 'uploads');

if (!fs.existsSync(uploadsDir)) {
    fs.mkdirSync(uploadsDir, { recursive: true });
    console.log(`[LOG] Created uploads directory at ${uploadsDir}`);
}

app.use('/uploads', express.static(uploadsDir));
app.use('/admin_panel', express.static(path.join(__dirname, 'admin_panel')));
app.use(express.json());

app.get('/admin', (req, res) => {
    res.sendFile(path.join(__dirname, 'admin_panel', 'index.html'));
});

let clients = [];
let adminClients = [];
let currentBatch = null;

let queue = [];
let activeTimer = null;
let pauseQueue = false;

wss.on('connection', (ws) => {
    console.log('[WS] New WebSocket connection');
    clients.push(ws);
    
    if (currentBatch) {
        console.log('[WS] Sending current batch to new client');
        ws.send(JSON.stringify({ 
            type: 'display',
            images: currentBatch.filePaths, 
            id: currentBatch.id 
        }));
    }
    
    ws.on('close', () => {
        clients = clients.filter(client => client !== ws);
    });
});

adminWss.on('connection', (ws) => {
    console.log('[ADMIN] New admin WebSocket connection');
    adminClients.push(ws);
    
    sendQueueStateToAdmins();
    
    ws.on('message', handleAdminMessage);
    
    ws.on('close', () => {
        adminClients = adminClients.filter(client => client !== ws);
    });
});

const upgradedSockets = new WeakSet();

server.on('upgrade', (request, socket, head) => {
    try {
        if (upgradedSockets.has(socket)) {
            return;
        }

        const pathname = new URL(request.url, `http://${request.headers.host}`).pathname;
        // console.log(`[WS] Upgrade request for path: ${pathname}`);
        
        if (pathname === '/admin_ws') {
            upgradedSockets.add(socket);
            adminWss.handleUpgrade(request, socket, head, (ws) => {
                adminWss.emit('connection', ws, request);
            });
        } else {
            upgradedSockets.add(socket);
            wss.handleUpgrade(request, socket, head, (ws) => {
                wss.emit('connection', ws, request);
            });
        }
    } catch (error) {
        console.error('Error during WebSocket upgrade:', error);
        socket.destroy();
    }
});

function generateRandomId() {
    return crypto.randomBytes(16).toString('hex');
}

const storage = multer.diskStorage({
    destination: (req, file, cb) => {
        const batchId = req.batchId || generateRandomId();
        req.batchId = batchId;
        
        const newDir = path.join(uploadsDir, batchId);
        fs.mkdir(newDir, { recursive: true }, (err) => {
            if (err) {
                console.error('[-] Error creating directory:', err);
            } else {
                cb(null, newDir);
            }
        });
    },
    filename: (req, file, cb) => {
        cb(null, file.originalname);
    }
});

const upload = multer({ storage });

function broadcast(batch) {
    console.log(`[QUEUE] Broadcasting batch ID ${batch.id} with ${batch.filePaths.length} images.`);
    currentBatch = batch;
    
    clients.forEach(client => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(JSON.stringify({ 
                type: 'display',
                images: batch.filePaths, 
                id: batch.id 
            }));
        }
    });

    setTimeout(() => {
        const dirPath = path.join(uploadsDir, batch.id);
        fs.rm(dirPath, { recursive: true, force: true }, (err) => {
            if (err) console.error(`[ERROR] Failed to delete ${dirPath}:`, err);
            else console.log(`[LOG] Deleted directory: ${dirPath}`);
            
            if (currentBatch && currentBatch.id === batch.id) {
                currentBatch = null;
            }
        });
    }, DISPLAY_TIME);
}

function processQueue() {
    if (pauseQueue) {
        console.log("[QUEUE] Queue is paused. Not processing next batch.");
        return;
    }
    
    if (queue.length > 0) {
        const batch = queue.shift();
        broadcast(batch);
        
        sendQueueStateToAdmins();
        
        activeTimer = setTimeout(() => {
            activeTimer = null;
            if (queue.length > 0 && !pauseQueue) {
                processQueue();
            }
        }, DISPLAY_TIME);
    }
}

function sendQueueStateToAdmins() {
    const queueState = {
        type: 'queueUpdate',
        current: currentBatch,
        queue: queue,
        paused: pauseQueue,
        activeTimer: activeTimer !== null
    };
    
    adminClients.forEach(client => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(JSON.stringify(queueState));
        }
    });
}

function handleAdminMessage(message) {
    try {
        const data = JSON.parse(message);
        console.log(`[ADMIN] Received command: ${data.action}`);
        
        switch (data.action) {
            case 'skip':
                if (activeTimer) {
                    clearTimeout(activeTimer);
                    activeTimer = null;
                    
                    // Start next item if queue isn't paused
                    if (!pauseQueue && queue.length > 0) {
                        processQueue();
                    } else {
                        currentBatch = null;
                        sendQueueStateToAdmins();
                        
                        // Clear display for clients
                        clients.forEach(client => {
                            if (client.readyState === WebSocket.OPEN) {
                                client.send(JSON.stringify({ 
                                    type: 'clear'
                                }));
                            }
                        });
                    }
                }
                break;
                
            case 'pause':
                pauseQueue = true;
                sendQueueStateToAdmins();
                break;
                
            case 'resume':
                pauseQueue = false;
                if (!activeTimer && queue.length > 0) {
                    processQueue();
                }
                sendQueueStateToAdmins();
                break;
                
            case 'remove':
                if (data.id) {
                    queue = queue.filter(batch => batch.id !== data.id);
                    sendQueueStateToAdmins();
                }
                break;
                
            case 'move':
                if (data.id && data.position !== undefined) {
                    const index = queue.findIndex(batch => batch.id === data.id);
                    if (index !== -1) {
                        const batch = queue.splice(index, 1)[0];
                        const newPosition = Math.min(Math.max(data.position, 0), queue.length);
                        queue.splice(newPosition, 0, batch);
                        sendQueueStateToAdmins();
                    }
                }
                break;
                
            case 'getState':
                // This just triggers sending the current state to admins
                sendQueueStateToAdmins();
                break;
        }
    } catch (err) {
        console.error('[ADMIN] Error processing admin message:', err);
    }
}

app.post('/upload', upload.any(), (req, res) => {
    if (!req.files || req.files.length === 0) {
        return res.status(400).send('No files were uploaded.');
    } else if (req.files.length !== 8) {
        return res.status(400).send('Invalid amount of frames sent');
    }

    const batchId = req.batchId;
    console.log(`[POST] Received ${req.files.length} images, queued for ID ${batchId}`);

    const sortedFiles = [...req.files].sort((a, b) => {
        return parseInt(a.fieldname) - parseInt(b.fieldname);
    });

    const uploadedFileNames = sortedFiles.map(file =>
        `/uploads/${path.basename(file.destination)}/${file.filename}`
    );

    const batch = { id: batchId, filePaths: uploadedFileNames, timestamp: Date.now() };

    queue.push(batch);
    console.log(`[QUEUE] Batch ${batch.id} added to queue.`);
    
    sendQueueStateToAdmins();
    
    if (!activeTimer && !pauseQueue) {
        processQueue();
    }

    res.send('Images queued successfully!');
});

app.get('/api/queue', (req, res) => {
    res.json({
        current: currentBatch,
        queue: queue,
        paused: pauseQueue
    });
});

function cleanUploadsDir() {
    console.log("[LOG] Cleaning up uploads dir");
    
    if (!fs.existsSync(uploadsDir)) {
        fs.mkdirSync(uploadsDir, { recursive: true });
        return;
    }
    
    fs.readdir(uploadsDir, (err, files) => {
        if (err) {
            console.error(`[ERROR] Failed to read uploads directory: ${err.message}`);
            return;
        }
        
        if (files.length === 0) {
            return;
        }

        for (const file of files) {
            let currentPath = path.join(uploadsDir, file);
            fs.rm(currentPath, { recursive: true, force: true }, (err) => {
                if (err) {
                    console.error(`[ERROR] Failed to delete ${currentPath}: ${err.message}`);
                }
            });
        }
    });
}

server.listen(PORT, '0.0.0.0', () => {
    console.log(`[LOG] Server started at http://127.0.0.1:${PORT}`);
    console.log(`[LOG] Admin panel available at http://127.0.0.1:${PORT}/admin`);
    cleanUploadsDir();
});