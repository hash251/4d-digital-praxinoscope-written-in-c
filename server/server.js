const express = require('express');
const multer = require('multer');
const http = require('http');
const WebSocket = require('ws');
const path = require('path');
const fs = require("fs");
const app = express();
const PORT = 1337;
const server = http.createServer(app);
const wss = new WebSocket.Server({ server });

const uploadsDir = path.join(__dirname, 'uploads');

if (!fs.existsSync(uploadsDir)) {
    fs.mkdirSync(uploadsDir, { recursive: true });
    console.log(`[LOG] Created uploads directory at ${uploadsDir}`);
}

app.use('/uploads', express.static(uploadsDir));

let clients = [];
wss.on('connection', (ws) => {
    console.log('[WS] New WebSocket connection');
    clients.push(ws);
    ws.on('close', () => {
        clients = clients.filter(client => client !== ws);
    });
});

const storage = multer.diskStorage({
    destination: (req, file, cb) => {
        cb(null, uploadsDir);
    },
    filename: (req, file, cb) => {
        cb(null, file.originalname);
    }
});

const upload = multer({ storage });

function cleanUploadsDir() {
    console.log("[LOG] Cleaning up uploads dir");
    
    if (!fs.existsSync(uploadsDir)) {
        console.log("[LOG] Uploads directory doesn't exist, creating it");
        fs.mkdirSync(uploadsDir, { recursive: true });
        console.log("[LOG] Uploads is clean\n");
        return;
    }
    
    fs.readdir(uploadsDir, (err, files) => {
        if (err) {
            console.error(`[ERROR] Failed to read uploads directory: ${err.message}`);
            return;
        }
        
        if (files.length === 0) {
            console.log("[LOG] Uploads is already clean\n");
            return;
        }
        
        let completedDeletions = 0;
        for (const file of files) {
            let p = path.join(uploadsDir, file);
            fs.unlink(p, (err) => {
                if (err) {
                    console.error(`[ERROR] Failed to delete file ${p}: ${err.message}`);
                } else {
                    console.log(`[LOG]\t Removed file: ${p}`);
                }
                
                completedDeletions++;
                if (completedDeletions === files.length) {
                    console.log("[LOG] Uploads is clean\n");
                }
            });
        }
    });
}

function getFileExtension(filename) {
    return filename.substring(filename.lastIndexOf('.'));
}

app.post('/upload', upload.any(), (req, res) => {
    if (!req.files || req.files.length === 0) {
        return res.status(400).send('No files were uploaded.');
    }
    
    console.log(`[POST] Received ${req.files.length} images from drawing client`);
    
    const sortedFiles = [...req.files].sort((a, b) => {
        const aNum = parseInt(a.fieldname.split('_')[1]);
        const bNum = parseInt(b.fieldname.split('_')[1]);
        return aNum - bNum;
    });
    
    const uploadedFileNames = sortedFiles.map(file => `/uploads/${file.filename}`);
    
    clients.forEach(client => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(JSON.stringify({ images: uploadedFileNames }));
        }
    });
    
    res.send('Images uploaded successfully!');
});

server.listen(PORT, '0.0.0.0', () => {
    console.log(`[LOG] Server started at http://127.0.0.1:${PORT}`);
    cleanUploadsDir();
});
