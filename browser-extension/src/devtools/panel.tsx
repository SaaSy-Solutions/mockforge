/**
 * DevTools Panel
 *
 * React component for the DevTools panel
 */

import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { MockConfig, ConnectionStatus, CapturedRequest, Environment } from '../shared/types';
import { MockPreview } from '../preview/MockPreview';
import XRayPanel from './xray-panel';

type Tab = 'requests' | 'mocks' | 'preview' | 'xray';

function ForgeConnectPanel() {
    const [mocks, setMocks] = useState<MockConfig[]>([]);
    const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({ connected: false });
    const [capturedRequests, setCapturedRequests] = useState<CapturedRequest[]>([]);
    const [selectedRequest, setSelectedRequest] = useState<CapturedRequest | null>(null);
    const [activeTab, setActiveTab] = useState<Tab>('requests');
    const [environments, setEnvironments] = useState<Environment[]>([]);
    const [activeEnvironment, setActiveEnvironment] = useState<Environment | null>(null);
    const [liveReloadEnabled, setLiveReloadEnabled] = useState(true);

    useEffect(() => {
        // Load mocks and environments
        loadMocks();
        loadEnvironments();

        // Listen for messages from background
        chrome.runtime.onMessage.addListener((message) => {
            if (message.type === 'REQUEST_CAPTURED') {
                setCapturedRequests((prev) => [message.payload, ...prev].slice(0, 100)); // Keep last 100
            } else if (message.type === 'MOCK_UPDATED' && liveReloadEnabled) {
                // Live reload: refresh mocks when updated
                loadMocks();
            } else if (message.type === 'MOCK_DELETED' && liveReloadEnabled) {
                // Live reload: refresh mocks when deleted
                loadMocks();
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

    const loadEnvironments = async () => {
        try {
            const response = await chrome.runtime.sendMessage({ type: 'GET_ENVIRONMENTS' });
            if (response.success) {
                setEnvironments(response.data);
                const active = response.data.find((env: Environment) => env.active) || response.data[0] || null;
                setActiveEnvironment(active);
            }
        } catch (error) {
            console.error('Failed to load environments:', error);
        }
    };

    const handleEnvironmentChange = async (environmentId: string) => {
        try {
            const response = await chrome.runtime.sendMessage({
                type: 'SET_ACTIVE_ENVIRONMENT',
                payload: { environmentId },
            });
            if (response.success) {
                await loadEnvironments();
            } else {
                alert(`Failed to switch environment: ${response.error}`);
            }
        } catch (error) {
            alert(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
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

            {/* Environment Selector and Live Reload Toggle */}
            <div style={{ display: 'flex', gap: '20px', marginBottom: '20px', alignItems: 'flex-end' }}>
                {environments.length > 0 && (
                    <div style={{ flex: 1 }}>
                        <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold' }}>
                            Environment:
                        </label>
                        <select
                            value={activeEnvironment?.id || ''}
                            onChange={(e) => handleEnvironmentChange(e.target.value)}
                            style={{
                                padding: '8px',
                                border: '1px solid #ccc',
                                borderRadius: '4px',
                                minWidth: '200px',
                            }}
                        >
                            {environments.map((env) => (
                                <option key={env.id} value={env.id}>
                                    {env.name} {env.active ? '(Active)' : ''}
                                </option>
                            ))}
                        </select>
                    </div>
                )}
                <div>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
                        <input
                            type="checkbox"
                            checked={liveReloadEnabled}
                            onChange={(e) => {
                                setLiveReloadEnabled(e.target.checked);
                                chrome.storage.local.set({ liveReloadEnabled: e.target.checked });
                            }}
                        />
                        <span style={{ fontWeight: 'bold' }}>Live Reload</span>
                    </label>
                    <div style={{ fontSize: '12px', color: '#666', marginTop: '2px' }}>
                        Auto-refresh when mocks change
                    </div>
                </div>
            </div>

            {/* Tab Navigation */}
            <div style={{ display: 'flex', gap: '10px', marginBottom: '20px', borderBottom: '2px solid #ccc' }}>
                <button
                    onClick={() => setActiveTab('requests')}
                    style={{
                        padding: '10px 20px',
                        backgroundColor: activeTab === 'requests' ? '#007bff' : 'transparent',
                        color: activeTab === 'requests' ? 'white' : '#007bff',
                        border: 'none',
                        borderBottom: activeTab === 'requests' ? '2px solid #007bff' : '2px solid transparent',
                        cursor: 'pointer',
                        fontWeight: activeTab === 'requests' ? 'bold' : 'normal',
                    }}
                >
                    Requests
                </button>
                <button
                    onClick={() => setActiveTab('mocks')}
                    style={{
                        padding: '10px 20px',
                        backgroundColor: activeTab === 'mocks' ? '#007bff' : 'transparent',
                        color: activeTab === 'mocks' ? 'white' : '#007bff',
                        border: 'none',
                        borderBottom: activeTab === 'mocks' ? '2px solid #007bff' : '2px solid transparent',
                        cursor: 'pointer',
                        fontWeight: activeTab === 'mocks' ? 'bold' : 'normal',
                    }}
                >
                    Mocks ({mocks.length})
                </button>
                <button
                    onClick={() => setActiveTab('preview')}
                    style={{
                        padding: '10px 20px',
                        backgroundColor: activeTab === 'preview' ? '#007bff' : 'transparent',
                        color: activeTab === 'preview' ? 'white' : '#007bff',
                        border: 'none',
                        borderBottom: activeTab === 'preview' ? '2px solid #007bff' : '2px solid transparent',
                        cursor: 'pointer',
                        fontWeight: activeTab === 'preview' ? 'bold' : 'normal',
                    }}
                >
                    Preview
                </button>
                <button
                    onClick={() => setActiveTab('xray')}
                    style={{
                        padding: '10px 20px',
                        backgroundColor: activeTab === 'xray' ? '#007bff' : 'transparent',
                        color: activeTab === 'xray' ? 'white' : '#007bff',
                        border: 'none',
                        borderBottom: activeTab === 'xray' ? '2px solid #007bff' : '2px solid transparent',
                        cursor: 'pointer',
                        fontWeight: activeTab === 'xray' ? 'bold' : 'normal',
                    }}
                >
                    🔍 X-Ray
                </button>
            </div>

            {/* Tab Content */}
            {activeTab === 'preview' && (
                <MockPreview
                    request={selectedRequest}
                    onSave={async (mock) => {
                        const response = await chrome.runtime.sendMessage({
                            type: 'CREATE_MOCK',
                            payload: mock,
                        });
                        if (response.success) {
                            await loadMocks();
                            alert('Mock created successfully!');
                            setActiveTab('mocks');
                        } else {
                            alert(`Failed to create mock: ${response.error}`);
                        }
                    }}
                    onCancel={() => setActiveTab('requests')}
                />
            )}

            {activeTab === 'xray' && (
                <XRayPanel />
            )}

            {activeTab !== 'preview' && activeTab !== 'xray' && (
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
                        <div style={{ marginTop: '10px', display: 'flex', gap: '10px' }}>
                            <button
                                onClick={() => {
                                    setActiveTab('preview');
                                }}
                                style={{
                                    padding: '8px 16px',
                                    backgroundColor: '#28a745',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                }}
                            >
                                Preview Mock
                            </button>
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
            )}
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
