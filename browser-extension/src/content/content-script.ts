/**
 * Content Script
 *
 * Injects SDK into page context and forwards requests to background
 */

import { CapturedRequest } from '../shared/types';

// Inject SDK into page context
const script = document.createElement('script');
script.src = chrome.runtime.getURL('src/injector/injector.js');
script.onload = () => {
    script.remove();
};
(document.head || document.documentElement).appendChild(script);

// Listen for messages from injected script
window.addEventListener('message', (event) => {
    // Only accept messages from our extension
    if (event.source !== window) {
        return;
    }

    if (event.data && event.data.type === 'FORGECONNECT_REQUEST') {
        const request: CapturedRequest = event.data.payload;

        // Forward to background script
        chrome.runtime.sendMessage({
            type: 'REQUEST_CAPTURED',
            payload: request,
        }).catch(() => {
            // Background might not be ready
        });
    }
});

// Listen for connection status updates
chrome.runtime.onMessage.addListener((message) => {
    if (message.type === 'CONNECTION_CHANGE') {
        // Forward to page context
        window.postMessage({
            type: 'FORGECONNECT_CONNECTION',
            payload: message.payload,
        }, '*');
    }
});
