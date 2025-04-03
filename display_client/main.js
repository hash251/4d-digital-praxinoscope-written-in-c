const { app, BrowserWindow, screen } = require('electron');
const path = require('path');
const WebSocket = require('ws');
const dotenv = require('dotenv');
dotenv.config();

const PORT = process.env.PORT || "1337";
const PROTOCOL = process.env.PROTOCOL || "ws";
const IP = process.env.IP || '127.0.0.1';
const SERVER_URL = `${PROTOCOL}://${IP}:${PORT}`;

let windows = [];
let wsConnection = null;
let lastUpdateTime = null;

app.commandLine.appendSwitch("disable-http-cache");

function createWindow(id, bounds) {
  const win = new BrowserWindow({
    width: bounds.width,
    height: bounds.height,
    x: bounds.x,
    y: bounds.y,
    frame: false,
    fullscreen: false,
    webPreferences: {
      preload: path.join(__dirname, 'web/preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
    webSecurity: true,
  });

  win.loadFile(path.join(__dirname, '/web/index.html'));
  win.webContents.once('did-finish-load', () => {
    win.webContents.send('init', { id });
  });

  win.on('closed', () => {
    const index = windows.indexOf(win);
    if (index > -1) windows.splice(index, 1);
  });

  windows.push({ win, id });
}

function connectWebSocket() {
  const ws_url = SERVER_URL;
  console.log('[+] Connecting to WebSocket at:', ws_url);
  
  wsConnection = new WebSocket(ws_url);
  
  wsConnection.on('open', () => {
    console.log('[+] Connected to WebSocket server');
  });

  wsConnection.on('message', (data) => {
    try {
      const parsed = JSON.parse(data);
      const currentTime = Date.now();
      
      console.log('[+] Received update from server');
      
      if (lastUpdateTime !== null) {
        const timeDifference = (currentTime - lastUpdateTime) / 1000; // Convert to seconds
        console.log(`[TIME] Time since last update: ${timeDifference.toFixed(2)} seconds`);
      }
      
      lastUpdateTime = currentTime;
      
      let i = 0; // let i = window_offset; <- to have the correct absolute frame index
      windows.forEach(({ win, id }) => {
        if (parsed.images && parsed.images[i]) {
          win.webContents.send('image-update', `${SERVER_URL.replace(/^ws/, 'http')}${parsed.images[i]}`);
        }
        i += 1;
      });
    } catch (error) {
      console.error('[-] Error parsing WebSocket message:', error);
    }
  });

  wsConnection.on('error', (error) => {
    console.error('[-] WebSocket error:', error);
    setTimeout(connectWebSocket, 5000);
  });

  wsConnection.on('close', () => {
    console.log('[-] WebSocket connection closed, attempting to reconnect...');
    lastUpdateTime = null;
    setTimeout(connectWebSocket, 5000);
  });
}

app.whenReady().then(() => {
  const displays = screen.getAllDisplays();
  console.log(displays[0], displays[1]);
  
  if (displays.length < 2) {
    console.error('The required 2 monitors were not detected, only detected', displays.length, 'monitor(s)');
    app.quit();
    return;
  }
  
  createWindow(0, displays[0].bounds);
  createWindow(1, displays[1].bounds);
  connectWebSocket();
  
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow(0, displays[0].bounds);
      createWindow(1, displays[1].bounds);
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('before-quit', () => {
  if (wsConnection) {
    wsConnection.close();
  }
});