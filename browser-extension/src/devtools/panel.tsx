/**
 * DevTools Panel
 *
 * React component for the DevTools panel
 */

import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { MockConfig, ConnectionStatus, CapturedRequest, Environment, Persona, Scenario } from '../shared/types';
import { MockPreview } from '../preview/MockPreview';
import XRayPanel from './xray-panel';
import { SnapshotDiffPanel } from './SnapshotDiffPanel';

/**
 * Mock Item Component
 * Displays a mock with edit and delete actions
 */
interface MockItemProps {
    mock: MockConfig;
    onEdit: (mock: MockConfig) => void;
    onDelete: (id: string) => void;
    onUpdate: (id: string, mock: MockConfig) => void;
}

function MockItem({ mock, onEdit, onDelete, onUpdate }: MockItemProps) {
    const [isExpanded, setIsExpanded] = useState(false);
    const [isEditing, setIsEditing] = useState(false);
    const [editedResponse, setEditedResponse] = useState<string>('');
    const [jsonError, setJsonError] = useState<string | null>(null);

    useEffect(() => {
        if (mock.response?.body) {
            setEditedResponse(JSON.stringify(mock.response.body, null, 2));
        }
    }, [mock]);

    const handleEditClick = () => {
        setIsEditing(true);
        setIsExpanded(true);
    };

    const handleSaveEdit = () => {
        try {
            const parsed = JSON.parse(editedResponse);
            setJsonError(null);
            const updatedMock: MockConfig = {
                ...mock,
                response: {
                    ...mock.response,
                    body: parsed,
                },
            };
            onUpdate(mock.id!, updatedMock);
            setIsEditing(false);
        } catch (error) {
            setJsonError(error instanceof Error ? error.message : 'Invalid JSON');
        }
    };

    const handleCancelEdit = () => {
        setIsEditing(false);
        setJsonError(null);
        if (mock.response?.body) {
            setEditedResponse(JSON.stringify(mock.response.body, null, 2));
        }
    };

    return (
        <div
            style={{
                padding: '10px',
                margin: '5px 0',
                border: '1px solid #ddd',
                borderRadius: '4px',
                backgroundColor: isEditing ? '#fff9e6' : 'white',
            }}
        >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ flex: 1 }}>
                    <div style={{ fontWeight: 'bold' }}>
                        {mock.name}
                    </div>
                    <div style={{ fontSize: '12px', color: '#666' }}>
                        {mock.method} {mock.path} • Status: {mock.status_code || 200}
                    </div>
                </div>
                <div style={{ display: 'flex', gap: '5px' }}>
                    <button
                        onClick={() => setIsExpanded(!isExpanded)}
                        style={{
                            padding: '4px 8px',
                            backgroundColor: '#6c757d',
                            color: 'white',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer',
                            fontSize: '12px',
                        }}
                    >
                        {isExpanded ? '▼' : '▶'}
                    </button>
                    {!isEditing && (
                        <>
                            <button
                                onClick={handleEditClick}
                                style={{
                                    padding: '4px 8px',
                                    backgroundColor: '#007bff',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px',
                                }}
                            >
                                ✏️ Edit
                            </button>
                            <button
                                onClick={() => onDelete(mock.id!)}
                                style={{
                                    padding: '4px 8px',
                                    backgroundColor: '#dc3545',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px',
                                }}
                            >
                                🗑️ Delete
                            </button>
                        </>
                    )}
                </div>
            </div>

            {isExpanded && (
                <div style={{ marginTop: '10px', paddingTop: '10px', borderTop: '1px solid #eee' }}>
                    {isEditing ? (
                        <div>
                            <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold', fontSize: '12px' }}>
                                Response Body (JSON) - Live Edit
                            </label>
                            <textarea
                                value={editedResponse}
                                onChange={(e) => {
                                    setEditedResponse(e.target.value);
                                    try {
                                        JSON.parse(e.target.value);
                                        setJsonError(null);
                                    } catch (error) {
                                        setJsonError(error instanceof Error ? error.message : 'Invalid JSON');
                                    }
                                }}
                                style={{
                                    width: '100%',
                                    minHeight: '200px',
                                    padding: '8px',
                                    border: `1px solid ${jsonError ? '#dc3545' : '#ccc'}`,
                                    borderRadius: '4px',
                                    fontFamily: 'monospace',
                                    fontSize: '11px',
                                }}
                            />
                            {jsonError && (
                                <div style={{ color: '#dc3545', fontSize: '11px', marginTop: '5px' }}>
                                    JSON Error: {jsonError}
                                </div>
                            )}
                            <div style={{ display: 'flex', gap: '5px', marginTop: '10px' }}>
                                <button
                                    onClick={handleSaveEdit}
                                    disabled={!!jsonError}
                                    style={{
                                        padding: '6px 12px',
                                        backgroundColor: jsonError ? '#ccc' : '#28a745',
                                        color: 'white',
                                        border: 'none',
                                        borderRadius: '4px',
                                        cursor: jsonError ? 'not-allowed' : 'pointer',
                                        fontSize: '12px',
                                    }}
                                >
                                    💾 Save
                                </button>
                                <button
                                    onClick={handleCancelEdit}
                                    style={{
                                        padding: '6px 12px',
                                        backgroundColor: '#6c757d',
                                        color: 'white',
                                        border: 'none',
                                        borderRadius: '4px',
                                        cursor: 'pointer',
                                        fontSize: '12px',
                                    }}
                                >
                                    Cancel
                                </button>
                            </div>
                        </div>
                    ) : (
                        <div>
                            <pre
                                style={{
                                    padding: '8px',
                                    backgroundColor: '#f5f5f5',
                                    borderRadius: '4px',
                                    overflow: 'auto',
                                    fontSize: '11px',
                                    maxHeight: '200px',
                                }}
                            >
                                {JSON.stringify(mock.response?.body || {}, null, 2)}
                            </pre>
                            <button
                                onClick={() => onEdit(mock)}
                                style={{
                                    marginTop: '8px',
                                    padding: '6px 12px',
                                    backgroundColor: '#007bff',
                                    color: 'white',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px',
                                }}
                            >
                                📝 Edit in Preview
                            </button>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}

type Tab = 'requests' | 'mocks' | 'preview' | 'xray' | 'snapshot-diff';

function ForgeConnectPanel() {
    const [mocks, setMocks] = useState<MockConfig[]>([]);
    const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({ connected: false });
    const [capturedRequests, setCapturedRequests] = useState<CapturedRequest[]>([]);
    const [selectedRequest, setSelectedRequest] = useState<CapturedRequest | null>(null);
    const [activeTab, setActiveTab] = useState<Tab>('requests');
    const [environments, setEnvironments] = useState<Environment[]>([]);
    const [activeEnvironment, setActiveEnvironment] = useState<Environment | null>(null);
    const [liveReloadEnabled, setLiveReloadEnabled] = useState(true);
    const [personas, setPersonas] = useState<Persona[]>([]);
    const [activePersona, setActivePersona] = useState<Persona | null>(null);
    const [scenarios, setScenarios] = useState<Scenario[]>([]);
    const [activeScenario, setActiveScenario] = useState<Scenario | null>(null);
    const [workspaceState, setWorkspaceState] = useState<any>(null);

    useEffect(() => {
        // Load mocks, environments, personas, and scenarios
        loadMocks();
        loadEnvironments();
        loadPersonas();
        loadScenarios();
        loadWorkspaceState();

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

    const loadPersonas = async () => {
        try {
            const response = await chrome.runtime.sendMessage({ type: 'GET_PERSONAS' });
            if (response.success) {
                setPersonas(response.data || []);
            }
        } catch (error) {
            console.error('Failed to load personas:', error);
        }
    };

    const loadScenarios = async () => {
        try {
            const response = await chrome.runtime.sendMessage({ type: 'GET_SCENARIOS' });
            if (response.success) {
                setScenarios(response.data || []);
            }
        } catch (error) {
            console.error('Failed to load scenarios:', error);
        }
    };

    const loadWorkspaceState = async () => {
        try {
            const response = await chrome.runtime.sendMessage({ type: 'GET_WORKSPACE_STATE' });
            if (response.success) {
                setWorkspaceState(response.data);
                // Update active persona and scenario from state
                if (response.data.active_persona) {
                    setActivePersona(response.data.active_persona);
                }
                if (response.data.active_scenario) {
                    const scenario = scenarios.find(s => s.id === response.data.active_scenario) || {
                        id: response.data.active_scenario,
                        name: response.data.active_scenario,
                    };
                    setActiveScenario(scenario);
                }
            }
        } catch (error) {
            console.error('Failed to load workspace state:', error);
        }
    };

    const handlePersonaChange = async (personaId: string) => {
        try {
            const persona = personas.find(p => p.id === personaId);
            if (!persona) return;

            const response = await chrome.runtime.sendMessage({
                type: 'SET_ACTIVE_PERSONA',
                payload: { persona },
            });

            if (response.success) {
                setActivePersona(persona);
                await loadWorkspaceState();
                // Show notification
                const notification = document.createElement('div');
                notification.style.cssText = `
                    position: fixed;
                    top: 20px;
                    right: 20px;
                    background: #28a745;
                    color: white;
                    padding: '15px 20px';
                    border-radius: 4px;
                    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
                    z-index: 10000;
                    font-family: system-ui;
                    font-size: 14px;
                `;
                notification.textContent = `✅ Persona "${persona.name || persona.id}" activated`;
                document.body.appendChild(notification);
                setTimeout(() => notification.remove(), 3000);
            } else {
                alert(`Failed to set persona: ${response.error}`);
            }
        } catch (error) {
            alert(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    };

    const handleScenarioChange = async (scenarioId: string) => {
        try {
            const response = await chrome.runtime.sendMessage({
                type: 'SET_ACTIVE_SCENARIO',
                payload: { scenario_id: scenarioId },
            });

            if (response.success) {
                const scenario = scenarios.find(s => s.id === scenarioId) || {
                    id: scenarioId,
                    name: scenarioId,
                };
                setActiveScenario(scenario);
                await loadWorkspaceState();
                // Show notification
                const notification = document.createElement('div');
                notification.style.cssText = `
                    position: fixed;
                    top: 20px;
                    right: 20px;
                    background: #28a745;
                    color: white;
                    padding: 15px 20px;
                    border-radius: 4px;
                    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
                    z-index: 10000;
                    font-family: system-ui;
                    font-size: 14px;
                `;
                notification.textContent = `✅ Scenario "${scenario.name}" activated`;
                document.body.appendChild(notification);
                setTimeout(() => notification.remove(), 3000);
            } else {
                alert(`Failed to set scenario: ${response.error}`);
            }
        } catch (error) {
            alert(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
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

    /**
     * Handle editing a mock's response
     */
    const handleEditMock = (mock: MockConfig) => {
        setSelectedRequest(null);
        setActiveTab('preview');
        // Create a synthetic request from the mock for editing
        const syntheticRequest: CapturedRequest = {
            method: mock.method,
            path: mock.path,
            url: `http://localhost:3000${mock.path}`,
            statusCode: mock.status_code || 200,
            responseBody: mock.response?.body,
            timestamp: Date.now(),
        };
        setSelectedRequest(syntheticRequest);
    };

    /**
     * Handle updating a mock's response
     */
    const handleUpdateMock = async (mockId: string, updatedMock: MockConfig) => {
        try {
            const response = await chrome.runtime.sendMessage({
                type: 'UPDATE_MOCK',
                payload: {
                    id: mockId,
                    mock: updatedMock,
                },
            });

            if (response.success) {
                await loadMocks();
                // Show success notification
                const notification = document.createElement('div');
                notification.style.cssText = `
                    position: fixed;
                    top: 20px;
                    right: 20px;
                    background: #28a745;
                    color: white;
                    padding: 15px 20px;
                    border-radius: 4px;
                    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
                    z-index: 10000;
                    font-family: system-ui;
                    font-size: 14px;
                `;
                notification.textContent = `✅ Mock updated successfully!`;
                document.body.appendChild(notification);
                setTimeout(() => notification.remove(), 3000);
            } else {
                alert(`Failed to update mock: ${response.error}`);
            }
        } catch (error) {
            alert(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    };

    /**
     * Handle "Mock this endpoint" - creates mock and reverse-injects into workspace
     * This integrates with the runtime daemon to automatically generate types,
     * OpenAPI updates, scenarios, and client stubs
     */
    const handleMockThisEndpoint = async (request: CapturedRequest) => {
        try {
            // Create mock with full context for reverse-injection
            const mock: MockConfig = {
                name: `Auto-mocked: ${request.method} ${request.path}`,
                method: request.method,
                path: request.path,
                response: {
                    body: request.responseBody || { message: 'Mock response' },
                },
                enabled: true,
                status_code: request.statusCode || 200,
                // Include additional metadata for runtime daemon
                metadata: {
                    source: 'devtools-extension',
                    captured_at: request.timestamp,
                    original_url: request.url,
                    query_params: request.queryParams,
                    request_headers: request.headers,
                    response_headers: request.responseHeaders,
                },
            };

            // Send to background script with reverse-injection flag
            const response = await chrome.runtime.sendMessage({
                type: 'CREATE_MOCK_WITH_INJECTION',
                payload: {
                    mock,
                    reverse_inject: true, // Trigger runtime daemon auto-generation
                    generate_types: true,
                    generate_client_stubs: true,
                    update_openapi: true,
                    create_scenario: true,
                },
            });

            if (response.success) {
                await loadMocks();
                // Show success notification
                const notification = document.createElement('div');
                notification.style.cssText = `
                    position: fixed;
                    top: 20px;
                    right: 20px;
                    background: #28a745;
                    color: white;
                    padding: 15px 20px;
                    border-radius: 4px;
                    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
                    z-index: 10000;
                    font-family: system-ui;
                    font-size: 14px;
                `;
                notification.textContent = `✅ Mock created! Types, OpenAPI, and client stubs generated.`;
                document.body.appendChild(notification);
                setTimeout(() => notification.remove(), 3000);
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

            {/* Environment, Persona, Scenario Selectors and Live Reload Toggle */}
            <div style={{ marginBottom: '20px' }}>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '15px', marginBottom: '15px' }}>
                    {environments.length > 0 && (
                        <div>
                            <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold', fontSize: '12px' }}>
                                Environment:
                            </label>
                            <select
                                value={activeEnvironment?.id || ''}
                                onChange={(e) => handleEnvironmentChange(e.target.value)}
                                style={{
                                    width: '100%',
                                    padding: '8px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
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
                    {personas.length > 0 && (
                        <div>
                            <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold', fontSize: '12px' }}>
                                Persona:
                            </label>
                            <select
                                value={activePersona?.id || ''}
                                onChange={(e) => handlePersonaChange(e.target.value)}
                                style={{
                                    width: '100%',
                                    padding: '8px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            >
                                <option value="">None</option>
                                {personas.map((persona) => (
                                    <option key={persona.id} value={persona.id}>
                                        {persona.name || persona.id} {activePersona?.id === persona.id ? '✓' : ''}
                                    </option>
                                ))}
                            </select>
                        </div>
                    )}
                    {scenarios.length > 0 && (
                        <div>
                            <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold', fontSize: '12px' }}>
                                Scenario:
                            </label>
                            <select
                                value={activeScenario?.id || ''}
                                onChange={(e) => handleScenarioChange(e.target.value)}
                                style={{
                                    width: '100%',
                                    padding: '8px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            >
                                <option value="">None</option>
                                {scenarios.map((scenario) => (
                                    <option key={scenario.id} value={scenario.id}>
                                        {scenario.name} {activeScenario?.id === scenario.id ? '✓' : ''}
                                    </option>
                                ))}
                            </select>
                        </div>
                    )}
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
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
                    {workspaceState && (
                        <div style={{ fontSize: '12px', color: '#666' }}>
                            {workspaceState.reality_level && (
                                <span>Reality: {workspaceState.reality_level.toFixed(1)}</span>
                            )}
                        </div>
                    )}
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
                <button
                    onClick={() => setActiveTab('snapshot-diff')}
                    style={{
                        padding: '10px 20px',
                        backgroundColor: activeTab === 'snapshot-diff' ? '#007bff' : 'transparent',
                        color: activeTab === 'snapshot-diff' ? 'white' : '#007bff',
                        border: 'none',
                        borderBottom: activeTab === 'snapshot-diff' ? '2px solid #007bff' : '2px solid transparent',
                        cursor: 'pointer',
                        fontWeight: activeTab === 'snapshot-diff' ? 'bold' : 'normal',
                    }}
                >
                    📊 Snapshot Diff
                </button>
            </div>

            {/* Tab Content */}
            {activeTab === 'preview' && (
                <MockPreview
                    request={selectedRequest}
                    existingMock={selectedRequest ? mocks.find(m => 
                        m.method === selectedRequest.method && 
                        m.path === selectedRequest.path
                    ) : undefined}
                    onSave={async (mock) => {
                        // Check if this is an update or create
                        const existing = mocks.find(m => 
                            m.method === mock.method && 
                            m.path === mock.path
                        );

                        if (existing && existing.id) {
                            // Update existing mock
                            await handleUpdateMock(existing.id, mock);
                        } else {
                            // Create new mock
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
                        }
                    }}
                    onCancel={() => setActiveTab('requests')}
                />
            )}

            {activeTab === 'xray' && (
                <XRayPanel />
            )}

            {activeTab === 'snapshot-diff' && (
                <SnapshotDiffPanel onClose={() => setActiveTab('requests')} />
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
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                        <div style={{ flex: 1 }}>
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
                                        <button
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                handleMockThisEndpoint(req);
                                            }}
                                            style={{
                                                padding: '6px 12px',
                                                backgroundColor: '#28a745',
                                                color: 'white',
                                                border: 'none',
                                                borderRadius: '4px',
                                                cursor: 'pointer',
                                                fontSize: '12px',
                                                fontWeight: 'bold',
                                                marginLeft: '10px',
                                            }}
                                            title="Mock this endpoint and reverse-inject into MockForge workspace"
                                        >
                                            🎯 Mock This
                                        </button>
                                    </div>
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
                                <MockItem
                                    key={mock.id}
                                    mock={mock}
                                    onEdit={handleEditMock}
                                    onDelete={deleteMock}
                                    onUpdate={handleUpdateMock}
                                />
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
