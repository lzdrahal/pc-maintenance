const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
    runTask: (taskName, scriptFile) => ipcRenderer.invoke('run-task', taskName, scriptFile),
    onTaskOutput: (callback) => ipcRenderer.on('task-output', (event, data) => callback(data)),
    onTaskComplete: (callback) => ipcRenderer.on('task-complete', (event, success) => callback(success)),
    reportError: () => ipcRenderer.invoke('report-error'),
    onUpdateMessage: (callback) => ipcRenderer.on('update-message', (event, msg) => callback(msg))
});
