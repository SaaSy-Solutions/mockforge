/**
 * DevTools Panel
 *
 * React component for the DevTools panel
 */

import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { MockConfig, ConnectionStatus, CapturedRequest } from '../shared/types';

function ForgeConnectPanel() {
    const [mocks, setMocks] = useState<MockConfig[]>([]);
    const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({ connected: false });
    const [capturedRequests, setCapturedRequests] = useState<CapturedRequest[]>([]);
    const [selectedRequest, setSelectedRequest] = useState<CapturedRequest | null>(null);

    useEffect(() => {
        // Load mocks
        loadMocks();

        // Listen for messages from background
        chrome.runtime.onMessage.addListener((message) => {
            if (message.type === 'REQUEST_CAPTURED') {
                setCapturedRequests((prev) => [message.payload, ...prev].slice(0, 100)); // Keep last 100
            }
        });

        // Poll connection status
        const interval = setInterval(() => {
            checkConnection();
        }, 5000);

        return () => clearInterval(interval);
    }, []);

    const checkConnection = async () => {
        // Connection status is managed by background script
        // We'll get updates via messages
    };

    const loadMocks = async () => {
        try {
            const response = await chrome.runtime.sendMessage({ type: 'GET_MOCKS' });
            if (response.success) {
                setMocks(response.data);
            }
        } catch (error) {
            console.error('Failed to load mocks:', error);
        }
    };

    const createMock = async (request: CapturedRequest) => {
        try {
            const mock: MockConfig = {
                name: `${request.method} ${request.path}`,
                method: request.method,
                path: request.path,
                response: {
                    body: request.responseBody || { message: 'Mock response' },
                },
                enabled: true,
                status_code: request.statusCode || 200,
            };

            const response = await chrome.runtime.sendMessage({
                type: 'CREATE_MOCK',
                payload: mock,
            });

            if (response.success) {
                await loadMocks();
                alert('Mock created successfully!');
            } else {
                alert(`Failed to create mock: ${response.error}`);
            }
        } catch (error) {
            alert(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    };

    const deleteMock = async (id: string) => {
        if (!confirm('Delete this mock?')) {
            return;
        }

        try {
            const response = await chrome.runtime.sendMessage({
                type: 'DELETE_MOCK',
                payload: { id },
            });

            if (response.success) {
                await loadMocks();
            } else {
                alert(`Failed to delete mock: ${response.error}`);
            }
        } catch (error) {
            alert(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    };

    return (
        <div style={{ padding: '20px', fontFamily: 'system-ui' }}>
            <h1>ForgeConnect</h1>

            <div style={{
                padding: '10px',
                marginBottom: '20px',
                backgroundColor: connectionStatus.connected ? '#d4edda' : '#f8d7da',
                color: connectionStatus.connected ? '#155724' : '#721c24',
                borderRadius: '4px',
            }}>
                {connectionStatus.connected
                    ? `✓ Connected to ${connectionStatus.url}`
                    : '✗ Not connected to MockForge'}
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px' }}>
                <div>
                    <h2>Captured Requests</h2>
                    <div style={{ maxHeight: '400px', overflowY: 'auto', border: '1px solid #ccc', padding: '10px' }}>
                        {capturedRequests.length === 0 ? (
                            <p>No requests captured yet. Make some API calls!</p>
                        ) : (
                            capturedRequests.map((req, idx) => (
                                <div
                                    key={idx}
                                    onClick={() => setSelectedRequest(req)}
                                    style={{
                                        padding: '10px',
                                        margin: '5px 0',
                                        border: '1px solid #ddd',
                                        borderRadius: '4px',
                                        cursor: 'pointer',
                                        backgroundColor: selectedRequest === req ? '#e7f3ff' : 'white',
                                    }}
                                >
                                    <div style={{ fontWeight: 'bold' }}>
                                        {req.method} {req.path}
                                    </div>
                                    <div style={{ fontSize: '12px', color: '#666' }}>
                                        {req.statusCode || 'Error'} • {new Date(req.timestamp).toLocaleTimeString()}
                                    </div>
                                    {req.error && (
                                        <div style={{ fontSize: '12px', color: '#dc3545' }}>
                                            {req.error.message}
                                        </div>
                                    )}
                                </div>
                            ))
                        )}
                    </div>
                    {selectedRequest && (
                        <div style={{ marginTop: '10px' }}>
                            <button
                                onClick={() => createMock(selectedRequest)}
                                style={{
                                    padding: '8px 16px',
                                    backgroundColor: '#007bff',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                }}
                            >
                                Create Mock from Request
                            </button>
                        </div>
                    )}
                </div>

                <div>
                    <h2>Mocks ({mocks.length})</h2>
                    <button
                        onClick={loadMocks}
                        style={{
                            padding: '8px 16px',
                            marginBottom: '10px',
                            backgroundColor: '#6c757d',
                            color: 'white',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer',
                        }}
                    >
                        Refresh
                    </button>
                    <div style={{ maxHeight: '400px', overflowY: 'auto', border: '1px solid #ccc', padding: '10px' }}>
                        {mocks.length === 0 ? (
                            <p>No mocks yet. Create one from a captured request!</p>
                        ) : (
                            mocks.map((mock) => (
                                <div
                                    key={mock.id}
                                    style={{
                                        padding: '10px',
                                        margin: '5px 0',
                                        border: '1px solid #ddd',
                                        borderRadius: '4px',
                                    }}
                                >
                                    <div style={{ fontWeight: 'bold' }}>
                                        {mock.name}
                                    </div>
                                    <div style={{ fontSize: '12px', color: '#666' }}>
                                        {mock.method} {mock.path}
                                    </div>
                                    <button
                                        onClick={() => deleteMock(mock.id!)}
                                        style={{
                                            marginTop: '5px',
                                            padding: '4px 8px',
                                            backgroundColor: '#dc3545',
                                            color: 'white',
                                            border: 'none',
                                            borderRadius: '4px',
                                            cursor: 'pointer',
                                            fontSize: '12px',
                                        }}
                                    >
                                        Delete
                                    </button>
                                </div>
                            ))
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}

// Initialize DevTools panel
chrome.devtools.panels.create(
    'ForgeConnect',
    'icons/icon48.png',
    'panel.html',
    (panel) => {
        // Panel will be initialized when HTML loads
    }
);

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initializePanel);
} else {
    initializePanel();
}

function initializePanel() {
    const container = document.getElementById('root') || document.body;
    if (container) {
        const root = createRoot(container);
        root.render(React.createElement(ForgeConnectPanel));
    }
}
