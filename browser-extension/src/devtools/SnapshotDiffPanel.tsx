/**
 * Snapshot Diff Panel Component
 *
 * Provides side-by-side visualization for comparing mock behavior between
 * different environments, personas, scenarios, or "realities" (Reality 0.1 vs Reality 0.9)
 */

import React, { useState, useEffect } from 'react';
import { MockConfig } from '../shared/types';

interface SnapshotDiff {
    left: Snapshot;
    right: Snapshot;
    differences: Difference[];
    summary: DiffSummary;
}

interface Snapshot {
    id: string;
    timestamp: number;
    environment_id?: string;
    persona_id?: string;
    scenario_id?: string;
    reality_level?: number;
    mocks: MockSnapshotItem[];
    metadata: Record<string, any>;
}

interface MockSnapshotItem {
    id: string;
    method: string;
    path: string;
    status_code: number;
    response_body: any;
    response_headers?: Record<string, string>;
    config: any;
}

interface Difference {
    diff_type: string;
    mock_id?: string;
    path: string;
    method: string;
    description: string;
    left_value?: any;
    right_value?: any;
    field_path?: string;
}

interface DiffSummary {
    left_total: number;
    right_total: number;
    differences_count: number;
    only_in_left: number;
    only_in_right: number;
    mocks_with_differences: number;
}

interface SnapshotDiffPanelProps {
    onClose?: () => void;
}

export function SnapshotDiffPanel({ onClose }: SnapshotDiffPanelProps) {
    const [leftSnapshot, setLeftSnapshot] = useState<Snapshot | null>(null);
    const [rightSnapshot, setRightSnapshot] = useState<Snapshot | null>(null);
    const [diff, setDiff] = useState<SnapshotDiff | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Comparison options
    const [leftEnvironment, setLeftEnvironment] = useState<string>('');
    const [rightEnvironment, setRightEnvironment] = useState<string>('');
    const [leftPersona, setLeftPersona] = useState<string>('');
    const [rightPersona, setRightPersona] = useState<string>('');
    const [leftScenario, setLeftScenario] = useState<string>('');
    const [rightScenario, setRightScenario] = useState<string>('');
    const [leftReality, setLeftReality] = useState<number>(0.0);
    const [rightReality, setRightReality] = useState<number>(1.0);

    const handleCompare = async () => {
        setLoading(true);
        setError(null);

        try {
            const response = await chrome.runtime.sendMessage({
                type: 'COMPARE_SNAPSHOTS',
                payload: {
                    left_environment_id: leftEnvironment || undefined,
                    right_environment_id: rightEnvironment || undefined,
                    left_persona_id: leftPersona || undefined,
                    right_persona_id: rightPersona || undefined,
                    left_scenario_id: leftScenario || undefined,
                    right_scenario_id: rightScenario || undefined,
                    left_reality_level: leftReality || undefined,
                    right_reality_level: rightReality || undefined,
                },
            });

            if (response.success) {
                setDiff(response.data);
                setLeftSnapshot(response.data.left);
                setRightSnapshot(response.data.right);
            } else {
                setError(response.error || 'Failed to compare snapshots');
            }
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Unknown error');
        } finally {
            setLoading(false);
        }
    };

    const getDiffTypeColor = (diffType: string) => {
        switch (diffType) {
            case 'missing_in_right':
            case 'missing_in_left':
                return '#ff6b6b';
            case 'status_code_mismatch':
                return '#ffa500';
            case 'body_mismatch':
                return '#4ecdc4';
            case 'headers_mismatch':
                return '#95e1d3';
            default:
                return '#6c757d';
        }
    };

    const getDiffTypeLabel = (diffType: string) => {
        switch (diffType) {
            case 'missing_in_right':
                return 'Missing in Right';
            case 'missing_in_left':
                return 'Missing in Left';
            case 'status_code_mismatch':
                return 'Status Code Mismatch';
            case 'body_mismatch':
                return 'Body Mismatch';
            case 'headers_mismatch':
                return 'Headers Mismatch';
            default:
                return diffType;
        }
    };

    return (
        <div style={{ padding: '20px', fontFamily: 'system-ui' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
                <h2>Snapshot Diff</h2>
                {onClose && (
                    <button
                        onClick={onClose}
                        style={{
                            padding: '8px 16px',
                            backgroundColor: '#6c757d',
                            color: 'white',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer',
                        }}
                    >
                        Close
                    </button>
                )}
            </div>

            {/* Comparison Configuration */}
            <div style={{
                padding: '15px',
                backgroundColor: '#f8f9fa',
                borderRadius: '4px',
                marginBottom: '20px',
            }}>
                <h3 style={{ marginTop: 0, marginBottom: '15px' }}>Comparison Settings</h3>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '15px' }}>
                    {/* Left Side */}
                    <div>
                        <h4 style={{ marginTop: 0 }}>Left (Baseline)</h4>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Environment:
                            </label>
                            <input
                                type="text"
                                value={leftEnvironment}
                                onChange={(e) => setLeftEnvironment(e.target.value)}
                                placeholder="Environment ID"
                                style={{
                                    width: '100%',
                                    padding: '6px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Persona:
                            </label>
                            <input
                                type="text"
                                value={leftPersona}
                                onChange={(e) => setLeftPersona(e.target.value)}
                                placeholder="Persona ID"
                                style={{
                                    width: '100%',
                                    padding: '6px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Scenario:
                            </label>
                            <input
                                type="text"
                                value={leftScenario}
                                onChange={(e) => setLeftScenario(e.target.value)}
                                placeholder="Scenario ID"
                                style={{
                                    width: '100%',
                                    padding: '6px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Reality Level: {leftReality.toFixed(1)}
                            </label>
                            <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.1"
                                value={leftReality}
                                onChange={(e) => setLeftReality(parseFloat(e.target.value))}
                                style={{ width: '100%' }}
                            />
                        </div>
                    </div>

                    {/* Right Side */}
                    <div>
                        <h4 style={{ marginTop: 0 }}>Right (Comparison)</h4>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Environment:
                            </label>
                            <input
                                type="text"
                                value={rightEnvironment}
                                onChange={(e) => setRightEnvironment(e.target.value)}
                                placeholder="Environment ID"
                                style={{
                                    width: '100%',
                                    padding: '6px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Persona:
                            </label>
                            <input
                                type="text"
                                value={rightPersona}
                                onChange={(e) => setRightPersona(e.target.value)}
                                placeholder="Persona ID"
                                style={{
                                    width: '100%',
                                    padding: '6px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Scenario:
                            </label>
                            <input
                                type="text"
                                value={rightScenario}
                                onChange={(e) => setRightScenario(e.target.value)}
                                placeholder="Scenario ID"
                                style={{
                                    width: '100%',
                                    padding: '6px',
                                    border: '1px solid #ccc',
                                    borderRadius: '4px',
                                }}
                            />
                        </div>
                        <div style={{ marginBottom: '10px' }}>
                            <label style={{ display: 'block', marginBottom: '5px', fontSize: '12px', fontWeight: 'bold' }}>
                                Reality Level: {rightReality.toFixed(1)}
                            </label>
                            <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.1"
                                value={rightReality}
                                onChange={(e) => setRightReality(parseFloat(e.target.value))}
                                style={{ width: '100%' }}
                            />
                        </div>
                    </div>
                </div>

                <button
                    onClick={handleCompare}
                    disabled={loading}
                    style={{
                        marginTop: '15px',
                        padding: '10px 20px',
                        backgroundColor: loading ? '#ccc' : '#007bff',
                        color: 'white',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: loading ? 'not-allowed' : 'pointer',
                        fontWeight: 'bold',
                    }}
                >
                    {loading ? 'Comparing...' : 'Compare Snapshots'}
                </button>
            </div>

            {error && (
                <div style={{
                    padding: '10px',
                    backgroundColor: '#f8d7da',
                    color: '#721c24',
                    borderRadius: '4px',
                    marginBottom: '20px',
                }}>
                    Error: {error}
                </div>
            )}

            {diff && (
                <>
                    {/* Summary */}
                    <div style={{
                        padding: '15px',
                        backgroundColor: '#e7f3ff',
                        borderRadius: '4px',
                        marginBottom: '20px',
                    }}>
                        <h3 style={{ marginTop: 0 }}>Summary</h3>
                        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '10px' }}>
                            <div>
                                <strong>Left Total:</strong> {diff.summary.left_total}
                            </div>
                            <div>
                                <strong>Right Total:</strong> {diff.summary.right_total}
                            </div>
                            <div>
                                <strong>Differences:</strong> {diff.summary.differences_count}
                            </div>
                            <div>
                                <strong>Only in Left:</strong> {diff.summary.only_in_left}
                            </div>
                            <div>
                                <strong>Only in Right:</strong> {diff.summary.only_in_right}
                            </div>
                            <div>
                                <strong>Mocks with Differences:</strong> {diff.summary.mocks_with_differences}
                            </div>
                        </div>
                    </div>

                    {/* Side-by-Side Comparison */}
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px', marginBottom: '20px' }}>
                        {/* Left Snapshot */}
                        <div>
                            <h3>Left Snapshot</h3>
                            <div style={{
                                maxHeight: '500px',
                                overflowY: 'auto',
                                border: '1px solid #ccc',
                                borderRadius: '4px',
                                padding: '10px',
                            }}>
                                {leftSnapshot?.mocks.map((mock) => (
                                    <div
                                        key={mock.id}
                                        style={{
                                            padding: '10px',
                                            margin: '5px 0',
                                            border: '1px solid #ddd',
                                            borderRadius: '4px',
                                            backgroundColor: diff.differences.some(d => d.mock_id === mock.id) ? '#fff3cd' : 'white',
                                        }}
                                    >
                                        <div style={{ fontWeight: 'bold' }}>
                                            {mock.method} {mock.path}
                                        </div>
                                        <div style={{ fontSize: '12px', color: '#666' }}>
                                            Status: {mock.status_code}
                                        </div>
                                        <details style={{ marginTop: '5px' }}>
                                            <summary style={{ cursor: 'pointer', fontSize: '12px' }}>Response Body</summary>
                                            <pre style={{
                                                fontSize: '11px',
                                                overflow: 'auto',
                                                maxHeight: '200px',
                                                marginTop: '5px',
                                            }}>
                                                {JSON.stringify(mock.response_body, null, 2)}
                                            </pre>
                                        </details>
                                    </div>
                                ))}
                            </div>
                        </div>

                        {/* Right Snapshot */}
                        <div>
                            <h3>Right Snapshot</h3>
                            <div style={{
                                maxHeight: '500px',
                                overflowY: 'auto',
                                border: '1px solid #ccc',
                                borderRadius: '4px',
                                padding: '10px',
                            }}>
                                {rightSnapshot?.mocks.map((mock) => (
                                    <div
                                        key={mock.id}
                                        style={{
                                            padding: '10px',
                                            margin: '5px 0',
                                            border: '1px solid #ddd',
                                            borderRadius: '4px',
                                            backgroundColor: diff.differences.some(d => d.mock_id === mock.id) ? '#fff3cd' : 'white',
                                        }}
                                    >
                                        <div style={{ fontWeight: 'bold' }}>
                                            {mock.method} {mock.path}
                                        </div>
                                        <div style={{ fontSize: '12px', color: '#666' }}>
                                            Status: {mock.status_code}
                                        </div>
                                        <details style={{ marginTop: '5px' }}>
                                            <summary style={{ cursor: 'pointer', fontSize: '12px' }}>Response Body</summary>
                                            <pre style={{
                                                fontSize: '11px',
                                                overflow: 'auto',
                                                maxHeight: '200px',
                                                marginTop: '5px',
                                            }}>
                                                {JSON.stringify(mock.response_body, null, 2)}
                                            </pre>
                                        </details>
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>

                    {/* Differences List */}
                    {diff.differences.length > 0 && (
                        <div>
                            <h3>Differences ({diff.differences.length})</h3>
                            <div style={{
                                maxHeight: '400px',
                                overflowY: 'auto',
                                border: '1px solid #ccc',
                                borderRadius: '4px',
                                padding: '10px',
                            }}>
                                {diff.differences.map((difference, idx) => (
                                    <div
                                        key={idx}
                                        style={{
                                            padding: '10px',
                                            margin: '5px 0',
                                            border: '1px solid #ddd',
                                            borderRadius: '4px',
                                            borderLeft: `4px solid ${getDiffTypeColor(difference.diff_type)}`,
                                        }}
                                    >
                                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                            <div>
                                                <div style={{ fontWeight: 'bold' }}>
                                                    {getDiffTypeLabel(difference.diff_type)}
                                                </div>
                                                <div style={{ fontSize: '12px', color: '#666' }}>
                                                    {difference.method} {difference.path}
                                                </div>
                                                <div style={{ fontSize: '12px', marginTop: '5px' }}>
                                                    {difference.description}
                                                </div>
                                            </div>
                                        </div>
                                        {difference.field_path && (
                                            <div style={{ fontSize: '11px', color: '#666', marginTop: '5px' }}>
                                                Field: {difference.field_path}
                                            </div>
                                        )}
                                        {(difference.left_value || difference.right_value) && (
                                            <details style={{ marginTop: '10px' }}>
                                                <summary style={{ cursor: 'pointer', fontSize: '12px' }}>Values</summary>
                                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px', marginTop: '5px' }}>
                                                    {difference.left_value && (
                                                        <div>
                                                            <strong>Left:</strong>
                                                            <pre style={{
                                                                fontSize: '10px',
                                                                overflow: 'auto',
                                                                maxHeight: '150px',
                                                                backgroundColor: '#f5f5f5',
                                                                padding: '5px',
                                                                borderRadius: '4px',
                                                            }}>
                                                                {JSON.stringify(difference.left_value, null, 2)}
                                                            </pre>
                                                        </div>
                                                    )}
                                                    {difference.right_value && (
                                                        <div>
                                                            <strong>Right:</strong>
                                                            <pre style={{
                                                                fontSize: '10px',
                                                                overflow: 'auto',
                                                                maxHeight: '150px',
                                                                backgroundColor: '#f5f5f5',
                                                                padding: '5px',
                                                                borderRadius: '4px',
                                                            }}>
                                                                {JSON.stringify(difference.right_value, null, 2)}
                                                            </pre>
                                                        </div>
                                                    )}
                                                </div>
                                            </details>
                                        )}
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}
                </>
            )}
        </div>
    );
}

