const { app, BrowserWindow, screen } = require('electron');
const path = require('path');
const WebSocket = require('ws');
const dotenv = require('dotenv');
dotenv.config();

const fs = require('fs');

let baseIndex = 0;
try {
  const idFilePath = path.join(require('os').homedir(), '.config', 'id');
  const fileContents = fs.readFileSync(idFilePath, 'utf8');
  baseIndex = parseInt(fileContents.trim(), 10) * 2;
  if (isNaN(baseIndex)) {
    console.warn('[!] Could not parse integer from ~/.config/id, defaulting to 0');
    baseIndex = 0;
  }
  console.log(`[+] Base index: ${baseIndex}`);
} catch (err) {
  console.error('[-] Failed to read ~/.config/id:', err);
}

const PORT = process.env.PORT || "1337";
const PROTOCOL = process.env.PROTOCOL || "ws";
const IP = process.env.IP || '127.0.0.1';
const SERVER_URL = `${PROTOCOL}://${IP}:${PORT}`;

console.log(`[CONFIG] PORT: ${PORT}`);
console.log(`[CONFIG] PROTOCOL: ${PROTOCOL}`);
console.log(`[CONFIG] IP: ${IP}`);
console.log(`[CONFIG] SERVER_URL: ${SERVER_URL}`);

let windows = [];
let wsConnection = null;
let lastUpdateTime = null;

app.commandLine.appendSwitch("disable-http-cache");

function createWindow(id, bounds) {
  console.log(`[WINDOW] Creating window ${id} with bounds:`, bounds);
  
  const win = new BrowserWindow({
    width: bounds.width,
    height: bounds.height,
    x: bounds.x,
    y: bounds.y,
    frame: false,
    fullscreen: true,
    webPreferences: {
      preload: path.join(__dirname, 'web/preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
    webSecurity: true,
  });

  win.loadFile(path.join(__dirname, '/web/index.html'));
  win.webContents.once('did-finish-load', () => {
    console.log(`[WINDOW] Window ${id} finished loading`);
    win.webContents.send('init', { id });
  });

  win.on('closed', () => {
    console.log(`[WINDOW] Window ${id} closed`);
    const index = windows.indexOf(win);
    if (index > -1) windows.splice(index, 1);
  });

  windows.push({ win, id });
  console.log(`[WINDOW] Window ${id} created and added to array. Total windows: ${windows.length}`);
}

function connectWebSocket() {
  const ws_url = SERVER_URL;
  
  console.log(`[WS] Connecting to WebSocket at: ${ws_url}`);
  console.log(`[WS] Connection timestamp: ${new Date().toISOString()}`);
  
  const connectionTimeout = setTimeout(() => {
    console.error(`[WS] Connection timeout after 10 seconds for ${ws_url}`);
    if (wsConnection && wsConnection.readyState === WebSocket.CONNECTING) {
      wsConnection.terminate();
    }
  }, 10000);
  
  try {
    wsConnection = new WebSocket(ws_url);
    console.log(`[WS] WebSocket object created, readyState: ${wsConnection.readyState}`);
    
    wsConnection.on('open', () => {
      clearTimeout(connectionTimeout);
      console.log(`[WS] Successfully connected to WebSocket server at ${ws_url}`);
      console.log(`[WS] Connection established at: ${new Date().toISOString()}`);
      console.log(`[WS] WebSocket readyState: ${wsConnection.readyState}`);
    });

    wsConnection.on('message', (data) => {
      try {
        const parsed = JSON.parse(data);
        const currentTime = Date.now();
    
        console.log(`[WS] 📨 Received update from server at ${new Date().toISOString()}`);
        console.log(`[WS] Message size: ${data.length} bytes`);
        console.log(`[WS] Parsed data keys: ${Object.keys(parsed)}`);
    
        if (lastUpdateTime !== null) {
          const timeDifference = (currentTime - lastUpdateTime) / 1000;
          console.log(`[TIME] Time since last update: ${timeDifference.toFixed(2)} seconds`);
        }
    
        lastUpdateTime = currentTime;
    
        console.log(`[WS] Processing update for ${windows.length} windows`);
        windows.forEach(({ win }, idx) => {
          const imageIndex = baseIndex + idx;
          console.log(`[WS] Window ${idx}: Looking for image at index ${imageIndex}`);
          
          if (parsed.images && parsed.images[imageIndex]) {
            const imageUrl = `${SERVER_URL.replace(/^ws/, 'http')}${parsed.images[imageIndex]}`;
            console.log(`[WS] Sending image update to window ${idx}: ${imageUrl}`);
            win.webContents.send('image-update', imageUrl);
          } else {
            console.log(`[WS] No image found at index ${imageIndex} for window ${idx}`);
          }
        });
      } catch (error) {
        console.error('[ERROR] Error parsing WebSocket message:', error);
        console.error('[WS] Raw message data:', data.toString());
      }
    });

    wsConnection.on('error', (error) => {
      clearTimeout(connectionTimeout);
      console.error(`[ERROR] WebSocket error:`, error);
      console.error(`[WS] Error code: ${error.code}`);
      console.error(`[WS] Error message: ${error.message}`);
      
      if (error.code === 'ECONNREFUSED') {
        console.error(`[WS] Connection refused - server may not be running at ${ws_url}`);
      } else if (error.code === 'ENOTFOUND') {
        console.error(`[WS] Host not found - check if ${IP} is correct`);
      } else if (error.code === 'ETIMEDOUT') {
        console.error(`[WS] Connection timed out - check network connectivity`);
      }
      
      console.log(`[WS] Retrying connection in 5 seconds...`);
      setTimeout(connectWebSocket, 5000);
    });

    wsConnection.on('close', (code, reason) => {
      clearTimeout(connectionTimeout);
      console.log(`[WS] 🔌 WebSocket connection closed`);
      console.log(`[WS] Close code: ${code}`);
      console.log(`[WS] Close reason: ${reason || 'No reason provided'}`);
      console.log(`[WS] Close timestamp: ${new Date().toISOString()}`);
      
      lastUpdateTime = null;
      
      setTimeout(connectWebSocket, 5000);
    });

    wsConnection.on('ping', () => {
      console.log('[WS] Received ping from server');
    });

    wsConnection.on('pong', () => {
      console.log('[WS] Received pong from server');
    });

  } catch (error) {
    clearTimeout(connectionTimeout);
    console.error('[WS] Error creating WebSocket:', error);
    
    setTimeout(connectWebSocket, 5000);
  }
}

app.whenReady().then(() => {
  console.log('[APP] Electron app is ready');
  
  const displays = screen.getAllDisplays();
  console.log(`[DISPLAY] Detected ${displays.length} display(s)`);
  
  displays.forEach((display, idx) => {
    console.log(`[DISPLAY] Display ${idx}:`, {
      id: display.id,
      bounds: display.bounds,
      workArea: display.workArea,
      scaleFactor: display.scaleFactor
    });
  });

  if (displays.length < 2) {
    console.error(`[ERROR] The required 2 monitors were not detected, only detected ${displays.length} monitor(s)`);
    app.quit();
    return;
  }

  displays.sort((a, b) => a.bounds.x - b.bounds.x);
  console.log('[DISPLAY] Displays sorted by x position');

  const primaryDisplay = displays[0];
  const offsetDisplay = displays[1];
  
  console.log('[DISPLAY] Primary display:', primaryDisplay.bounds);
  console.log('[DISPLAY] Offset display:', offsetDisplay.bounds);

  createWindow(0, offsetDisplay.bounds);
  createWindow(1, primaryDisplay.bounds);

  console.log('[WS] Starting WebSocket connection process...');
  connectWebSocket();

  app.on('activate', () => {
    console.log('[APP] App activated');
    if (BrowserWindow.getAllWindows().length === 0) {
      console.log('[APP] No windows found, creating new ones');
      createWindow(0, offsetDisplay.bounds);
      createWindow(1, primaryDisplay.bounds);
    }
  });
});

app.on('window-all-closed', () => {
  console.log('[APP] All windows closed');
  if (process.platform !== 'darwin') {
    console.log('[APP] Quitting app (not on macOS)');
    app.quit();
  }
});

app.on('before-quit', () => {
  console.log('[APP] App is about to quit');
  if (wsConnection) {
    console.log('[WS] Closing WebSocket connection before quit');
    wsConnection.close();
  }
});

process.on('uncaughtException', (error) => {
  console.error('[PROCESS] Main Process Uncaught Exception:', error);
  console.error('[PROCESS] Stack trace:', error.stack);
});

process.on('unhandledRejection', (reason, promise) => {
  console.error('[PROCESS] Main Process Unhandled Rejection:', reason);
  console.error('[PROCESS] Promise:', promise);
});