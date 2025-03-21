const WebSocket = require('ws');
const { ipcRenderer } = require('electron');

let imageId = 0;
let serverIp = '';

ipcRenderer.on('initialize', (_, data) => {
    imageId = data.id;
    serverIp = data.serverIp;

    console.log('Image ID set to:', imageId);
    console.log('Server IP set to:', serverIp);

    connectWebSocket();
});

function connectWebSocket() {
    const ws = new WebSocket(`${serverIp.replace(/^http/, 'ws')}`);

    ws.on('open', () => {
        console.log('Connected to WebSocket server');
    });

    ws.on('message', (data) => {
        const parsedData = JSON.parse(data);
        console.log('Received update:', parsedData);

        const imageElement = document.getElementById('image');
        if (parsedData.images && parsedData.images[imageId]) {
            imageElement.src = `${serverIp}${parsedData.images[imageId]}`;
        }
    });

    ws.on('error', (error) => {
        console.error('WebSocket error:', error);
    });

    ws.on('close', () => {
        console.log('WebSocket connection closed');
    });
}
