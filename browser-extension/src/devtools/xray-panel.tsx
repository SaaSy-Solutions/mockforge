//! X-Ray DevTools Panel
//!
//! Provides a DevTools panel for viewing and controlling MockForge state

import React, { useEffect, useState } from 'react';

interface XRayState {
    workspace_id?: string;
    scenario?: string;
    persona?: {
        id: string;
        traits?: Record<string, any>;
    };
    reality_level?: number;
    reality_level_name?: string;
    reality_ratio?: number;
    chaos_rules?: string[];
    timestamp?: string;
}

interface Entity {
    entity_type: string;
    entity_id: string;
    data: any;
    last_updated: string;
}

const XRayPanel: React.FC = () => {
    const [state, setState] = useState<XRayState | null>(null);
    const [entities, setEntities] = useState<Entity[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [apiUrl, setApiUrl] = useState('http://localhost:3000');
    const [workspace, setWorkspace] = useState('default');
    const [pollInterval, setPollInterval] = useState(2000);

    useEffect(() => {
        // Load settings from storage
        chrome.storage.sync.get(['mockforgeApiUrl', 'mockforgeWorkspace'], (result) => {
            if (result.mockforgeApiUrl) {
                setApiUrl(result.mockforgeApiUrl);
            }
            if (result.mockforgeWorkspace) {
                setWorkspace(result.mockforgeWorkspace);
            }
        });

        // Fetch state
        fetchState();

        // Poll for updates
        const interval = setInterval(() => {
            fetchState();
        }, pollInterval);

        return () => clearInterval(interval);
    }, [apiUrl, workspace, pollInterval]);

    const fetchState = async () => {
        try {
            setLoading(true);
            setError(null);

            // Fetch state summary
            const stateResponse = await fetch(
                `${apiUrl}/api/v1/xray/state/summary?workspace=${workspace}`
            );
            if (!stateResponse.ok) {
                throw new Error(`Failed to fetch state: ${stateResponse.statusText}`);
            }
            const stateData = await stateResponse.json();
            setState(stateData);

            // Fetch entities
            const entitiesResponse = await fetch(
                `${apiUrl}/api/v1/xray/entities?workspace=${workspace}`
            );
            if (entitiesResponse.ok) {
                const entitiesData = await entitiesResponse.json();
                setEntities(entitiesData.entities || []);
            }
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Unknown error');
        } finally {
            setLoading(false);
        }
    };

    const updatePersona = async (personaId: string) => {
        try {
            const response = await fetch(
                `${apiUrl}/api/v1/consistency/state/${workspace}/persona`,
                {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ persona_id: personaId }),
                }
            );
            if (response.ok) {
                fetchState();
            }
        } catch (err) {
            console.error('Failed to update persona:', err);
        }
    };

    const updateScenario = async (scenarioId: string) => {
        try {
            const response = await fetch(
                `${apiUrl}/api/v1/consistency/state/${workspace}/scenario`,
                {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ scenario_id: scenarioId }),
                }
            );
            if (response.ok) {
                fetchState();
            }
        } catch (err) {
            console.error('Failed to update scenario:', err);
        }
    };

    if (loading && !state) {
        return (
            <div style={{ padding: '20px', textAlign: 'center' }}>
                <div>Loading MockForge state...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div style={{ padding: '20px' }}>
                <div style={{ color: '#E74C3C', marginBottom: '10px' }}>Error: {error}</div>
                <button onClick={fetchState}>Retry</button>
            </div>
        );
    }

    return (
        <div style={{ padding: '16px', fontFamily: 'system-ui, sans-serif', fontSize: '13px' }}>
            {/* Settings */}
            <div style={{ marginBottom: '16px', padding: '12px', background: '#f5f5f5', borderRadius: '4px' }}>
                <div style={{ marginBottom: '8px' }}>
                    <label style={{ display: 'block', marginBottom: '4px', fontWeight: '500' }}>
                        API URL:
                    </label>
                    <input
                        type="text"
                        value={apiUrl}
                        onChange={(e) => {
                            setApiUrl(e.target.value);
                            chrome.storage.sync.set({ mockforgeApiUrl: e.target.value });
                        }}
                        style={{ width: '100%', padding: '4px', fontSize: '12px' }}
                    />
                </div>
                <div>
                    <label style={{ display: 'block', marginBottom: '4px', fontWeight: '500' }}>
                        Workspace:
                    </label>
                    <input
                        type="text"
                        value={workspace}
                        onChange={(e) => {
                            setWorkspace(e.target.value);
                            chrome.storage.sync.set({ mockforgeWorkspace: e.target.value });
                        }}
                        style={{ width: '100%', padding: '4px', fontSize: '12px' }}
                    />
                </div>
            </div>

            {/* State Summary */}
            {state && (
                <div style={{ marginBottom: '16px' }}>
                    <h2 style={{ fontSize: '16px', fontWeight: '600', marginBottom: '12px' }}>
                        Current State
                    </h2>
                    <div style={{ display: 'grid', gap: '8px' }}>
                        <StateRow label="Workspace" value={state.workspace_id} />
                        <StateRow
                            label="Scenario"
                            value={state.scenario || 'None'}
                            editable={state.scenario !== undefined}
                            onEdit={(value) => updateScenario(value)}
                        />
                        <StateRow
                            label="Persona"
                            value={state.persona?.id || 'None'}
                            editable={state.persona !== undefined}
                            onEdit={(value) => updatePersona(value)}
                        />
                        <StateRow
                            label="Reality Level"
                            value={state.reality_level_name || state.reality_level?.toString() || 'Unknown'}
                        />
                        <StateRow
                            label="Reality Ratio"
                            value={state.reality_ratio?.toFixed(2) || '0.00'}
                        />
                        {state.chaos_rules && state.chaos_rules.length > 0 && (
                            <div style={{ padding: '8px', background: '#fff3cd', borderRadius: '4px' }}>
                                <div style={{ fontWeight: '500', marginBottom: '4px' }}>Active Chaos Rules:</div>
                                <ul style={{ margin: 0, paddingLeft: '20px' }}>
                                    {state.chaos_rules.map((rule, i) => (
                                        <li key={i}>{rule}</li>
                                    ))}
                                </ul>
                            </div>
                        )}
                    </div>
                </div>
            )}

            {/* Entities */}
            {entities.length > 0 && (
                <div>
                    <h2 style={{ fontSize: '16px', fontWeight: '600', marginBottom: '12px' }}>
                        Entities ({entities.length})
                    </h2>
                    <div style={{ display: 'grid', gap: '8px' }}>
                        {entities.map((entity, i) => (
                            <div
                                key={i}
                                style={{
                                    padding: '8px',
                                    background: '#f9f9f9',
                                    borderRadius: '4px',
                                    border: '1px solid #ddd',
                                }}
                            >
                                <div style={{ fontWeight: '500', marginBottom: '4px' }}>
                                    {entity.entity_type}: {entity.entity_id}
                                </div>
                                <pre
                                    style={{
                                        fontSize: '11px',
                                        overflow: 'auto',
                                        maxHeight: '200px',
                                        margin: 0,
                                    }}
                                >
                                    {JSON.stringify(entity.data, null, 2)}
                                </pre>
                            </div>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
};

interface StateRowProps {
    label: string;
    value: string;
    editable?: boolean;
    onEdit?: (value: string) => void;
}

const StateRow: React.FC<StateRowProps> = ({ label, value, editable, onEdit }) => {
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState(value);

    if (isEditing && editable && onEdit) {
        return (
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                <span style={{ minWidth: '120px', fontWeight: '500' }}>{label}:</span>
                <input
                    type="text"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    onBlur={() => {
                        onEdit(editValue);
                        setIsEditing(false);
                    }}
                    onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                            onEdit(editValue);
                            setIsEditing(false);
                        } else if (e.key === 'Escape') {
                            setEditValue(value);
                            setIsEditing(false);
                        }
                    }}
                    autoFocus
                    style={{ flex: 1, padding: '4px', fontSize: '12px' }}
                />
            </div>
        );
    }

    return (
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <span style={{ minWidth: '120px', fontWeight: '500' }}>{label}:</span>
            <span style={{ flex: 1 }}>{value}</span>
            {editable && (
                <button
                    onClick={() => setIsEditing(true)}
                    style={{
                        padding: '2px 8px',
                        fontSize: '11px',
                        cursor: 'pointer',
                    }}
                >
                    Edit
                </button>
            )}
        </div>
    );
};

export default XRayPanel;
