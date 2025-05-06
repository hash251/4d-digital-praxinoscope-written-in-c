const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld(
    'api', {
        receive: (channel, func) => {
            const validChannels = ['init', 'image-update'];
            if (validChannels.includes(channel)) {
                ipcRenderer.on(channel, (event, ...args) => func(...args));
            }
        },
    }
);

window.addEventListener('error', (event) => {
  event.preventDefault();
  console.error('Renderer Error:', event.error);
});

window.addEventListener('unhandledrejection', (event) => {
  event.preventDefault();
  console.error('Unhandled Promise Rejection:', event.reason);
});