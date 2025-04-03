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
const wss = new WebSocket.Server({ server });

const uploadsDir = path.join(__dirname, 'uploads');

if (!fs.existsSync(uploadsDir)) {
    fs.mkdirSync(uploadsDir, { recursive: true });
    console.log(`[LOG] Created uploads directory at ${uploadsDir}`);
}

app.use('/uploads', express.static(uploadsDir));

let clients = [];
let currentBatch = null;

wss.on('connection', (ws) => {
    console.log('[WS] New WebSocket connection');
    clients.push(ws);
    
    if (currentBatch) {
        console.log('[WS] Sending current batch to new client');
        ws.send(JSON.stringify({ 
            images: currentBatch.filePaths, 
            id: currentBatch.id 
        }));
    }
    
    ws.on('close', () => {
        clients = clients.filter(client => client !== ws);
    });
});

let queue = [];
let activeTimer = null;

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
            client.send(JSON.stringify({ images: batch.filePaths, id: batch.id }));
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
    if (queue.length > 0) {
        const batch = queue.shift();
        broadcast(batch);
        
        activeTimer = setTimeout(() => {
            activeTimer = null;
            if (queue.length > 0) {
                processQueue();
            }
        }, DISPLAY_TIME);
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

    const batch = { id: batchId, filePaths: uploadedFileNames };

    if (activeTimer) {
        queue.push(batch);
        console.log(`[QUEUE] Batch ${batch.id} added to queue.`);
    } else {
        queue.push(batch);
        processQueue();
    }

    res.send('Images queued successfully!');
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
                } else {
                }
            });
        }
    });
}

server.listen(PORT, '0.0.0.0', () => {
    console.log(`[LOG] Server started at http://127.0.0.1:${PORT}`);
    cleanUploadsDir();
});