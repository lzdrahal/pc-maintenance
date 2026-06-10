const terminalOutput = document.getElementById('terminal-output');
const taskButtons = document.querySelectorAll('.task-btn:not(#run-all-btn)');
const runAllBtn = document.getElementById('run-all-btn');
const statusBadge = document.querySelector('.status-badge');
const reportBtn = document.getElementById('report-btn');

let isRunning = false;

reportBtn.addEventListener('click', () => {
    window.api.reportError();
});

function appendLog(text, type = 'normal') {
    const lines = text.split('\n');
    for (let t of lines) {
        if (!t.trim()) continue;
        const line = document.createElement('div');
        line.className = `log-line log-${type}`;
        line.textContent = t.replace(/\x1b\[.*?m/g, ''); // strip ansi
        terminalOutput.appendChild(line);
    }
    terminalOutput.scrollTop = terminalOutput.scrollHeight;
}

window.api.onTaskOutput((data) => {
    // Determine log type based on content loosely
    let type = 'normal';
    if (data.toLowerCase().includes('error') || data.toLowerCase().includes('failed')) type = 'error';
    if (data.toLowerCase().includes('completed') || data.toLowerCase().includes('finished') || data.toLowerCase().includes('success')) type = 'success';
    appendLog(data, type);
});

window.api.onTaskComplete((success) => {
    isRunning = false;
    
    if (success) {
        statusBadge.textContent = 'Ready';
        statusBadge.style.color = '#10b981';
        statusBadge.style.background = 'rgba(16, 185, 129, 0.15)';
        statusBadge.style.borderColor = 'rgba(16, 185, 129, 0.3)';
    } else {
        statusBadge.textContent = 'Failed';
        statusBadge.style.color = '#ef4444';
        statusBadge.style.background = 'rgba(239, 68, 68, 0.15)';
        statusBadge.style.borderColor = 'rgba(239, 68, 68, 0.3)';
        reportBtn.classList.remove('hidden');
    }
    
    // remove active class from all buttons
    taskButtons.forEach(b => b.classList.remove('active'));
    runAllBtn.classList.remove('active');
});

async function runTask(taskName, scriptFile, btnElement) {
    if (isRunning) return;
    isRunning = true;
    
    statusBadge.textContent = 'Running...';
    statusBadge.style.color = '#f59e0b';
    statusBadge.style.background = 'rgba(245, 158, 11, 0.15)';
    statusBadge.style.borderColor = 'rgba(245, 158, 11, 0.3)';
    reportBtn.classList.add('hidden');

    if (btnElement) {
        taskButtons.forEach(b => b.classList.remove('active'));
        btnElement.classList.add('active');
    }

    appendLog(`\n> Initializing task: ${taskName}...`);
    await window.api.runTask(taskName, scriptFile);
}

taskButtons.forEach(btn => {
    btn.addEventListener('click', () => {
        const script = btn.getAttribute('data-script');
        const name = btn.innerText.trim();
        runTask(name, script, btn);
    });
});

runAllBtn.addEventListener('click', async () => {
    if (isRunning) return;
    runAllBtn.classList.add('active');
    
    const tasks = [
        { name: "Update Windows", script: "Update-Windows.ps1" },
        { name: "Update Software", script: "Update-Software.ps1" },
        { name: "Clean System Junk", script: "Cleanup-System.ps1" },
        { name: "Repair System Files", script: "Repair-System.ps1" },
        { name: "Optimize Network", script: "Optimize-Network.ps1" }
    ];

    isRunning = true;
    statusBadge.textContent = 'Running All Tasks...';
    statusBadge.style.color = '#f59e0b';
    
    for (const task of tasks) {
        appendLog(`\n========================================`, 'normal');
        appendLog(`> Starting ${task.name}...`, 'normal');
        await window.api.runTask(task.name, task.script);
        // Temporarily set isRunning back to true because onTaskComplete sets it to false
        isRunning = true; 
    }
    
    appendLog(`\n> ALL TASKS COMPLETED SUCCESSFULLY.`, 'success');
    window.api.onTaskComplete(true); // reset UI state
});
