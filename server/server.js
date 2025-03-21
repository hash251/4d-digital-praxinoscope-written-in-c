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

app.use('/uploads', express.static(path.join(__dirname, '../uploads')));

let clients = [];

wss.on('connection', (ws) => {
    console.log('[LOG] New WebSocket connection');
    clients.push(ws);

    ws.on('close', () => {
        clients = clients.filter(client => client !== ws);
    });
});

const storage = multer.diskStorage({
    destination: (req, file, cb) => {
        cb(null, 'uploads/');
    },
    filename: (req, file, cb) => {
        cb(null, file.originalname);
    }
});
const upload = multer({ storage });

function cleanUploadsDir() {
    const directory = "uploads"

    console.log("[LOG] Cleaning up uploads dir")
    fs.readdir(directory, (err, files) => {
        if (err) throw err;

        for (const file of files) {
            let p = path.join(directory, file);
            fs.unlink(p, (err) => {
                console.log(`[LOG]\t Removed file: ${p}`);
                if (err) throw err;
            });
        }
    });
    console.log("[LOG] Uploads is clean\n")
}

function getFileExtension(filename) {
    return filename.substring(filename.lastIndexOf('.'));
}

app.post('/upload', upload.array('images', 8), (req, res) => {
    const uploadedFileNames = req.files.map(file => `/uploads/${file.filename}`);
    console.log("[LOG] Received images");

    clients.forEach(client => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(JSON.stringify({ images: uploadedFileNames }));
        }
    });

    res.send('Images uploaded successfully!');
});

server.listen(PORT, () => {
    console.log(`[LOG] Server started at http://127.0.0.1:${PORT}`);

    cleanUploadsDir();
});
