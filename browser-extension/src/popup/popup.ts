/**
 * Popup Script
 */

document.getElementById('openDevTools')?.addEventListener('click', () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs[0]?.id) {
            chrome.tabs.sendMessage(tabs[0].id, { type: 'OPEN_DEVTOOLS' });
        }
    });
});

document.getElementById('refresh')?.addEventListener('click', async () => {
    const statusEl = document.getElementById('status');
    if (statusEl) {
        statusEl.textContent = 'Checking connection...';
        statusEl.className = 'status disconnected';
    }

    try {
        const response = await chrome.runtime.sendMessage({ type: 'GET_MOCKS' });
        if (response.success) {
            if (statusEl) {
                statusEl.textContent = '✓ Connected';
                statusEl.className = 'status connected';
            }
        } else {
            if (statusEl) {
                statusEl.textContent = '✗ Not connected';
                statusEl.className = 'status disconnected';
            }
        }
    } catch (error) {
        if (statusEl) {
            statusEl.textContent = '✗ Connection error';
            statusEl.className = 'status disconnected';
        }
    }
});

// Check connection on load
document.getElementById('refresh')?.click();
