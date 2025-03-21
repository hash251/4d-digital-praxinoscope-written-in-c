const { app, BrowserWindow, screen } = require('electron'); const path = require('path');
const PORT = "1337";
const PROTOCOL = "https"
const IP = '98.188.150.207';
const SERVER_URL = `${PROTOCOL}://${IP}:${PORT}`;

let windows = [];

function createWindow(id, bounds) {
    const win = new BrowserWindow({
        width: bounds.width,
        height: bounds.height,
        x: bounds.x,
        y: bounds.y,
        frame: false,
        fullscreen: false,
        webPreferences: {
            nodeIntegration: true,
            contextIsolation: false,
        }
    });

    win.loadFile(path.join(__dirname, '/web/index.html'));

    win.webContents.once('did-finish-load', () => {
        win.webContents.send('initialize', { id, serverIp: SERVER_URL });
    });


    win.on('closed', () => {
        const index = windows.indexOf(win);
        if (index > -1) windows.splice(index, 1);
    });

    windows.push(win);
}

app.whenReady().then(() => {
    const displays = screen.getAllDisplays();
    
    if (displays.length < 2) {
        console.error('The required 2 monitors were not detected, only detected ', displays.length, ' monitor');
        app.quit();
        return;
    }

    createWindow(0, displays[0].bounds);
    createWindow(1, displays[1].bounds);

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
