/**
 * Mock Preview Component
 *
 * React component for previewing and editing mocks before creating them in MockForge
 */

import React, { useState, useEffect } from 'react';
import { MockConfig, CapturedRequest } from '../shared/types';
import { MockPreviewService } from './MockPreviewService';

interface MockPreviewProps {
    request: CapturedRequest | null;
    existingMock?: MockConfig;
    onSave?: (mock: MockConfig) => void;
    onCancel?: () => void;
}

export function MockPreview({ request, existingMock, onSave, onCancel }: MockPreviewProps) {
    const [mock, setMock] = useState<MockConfig | null>(null);
    const [previewEnabled, setPreviewEnabled] = useState(false);
    const [previewService] = useState(() => new MockPreviewService());
    const [originalResponse, setOriginalResponse] = useState<any>(null);
    const [editedResponse, setEditedResponse] = useState<string>('');
    const [jsonError, setJsonError] = useState<string | null>(null);

    useEffect(() => {
        previewService.initialize();
        checkPreviewMode();
    }, []);

    useEffect(() => {
        if (existingMock) {
            // Initialize from existing mock for editing
            setMock(existingMock);
            setOriginalResponse(existingMock.response?.body);
            setEditedResponse(JSON.stringify(existingMock.response?.body || {}, null, 2));
            setJsonError(null);
        } else if (request) {
            initializeMockFromRequest(request);
        }
    }, [request, existingMock]);

    const checkPreviewMode = async () => {
        try {
            const result = await chrome.storage.local.get(['previewModeEnabled']);
            setPreviewEnabled(result.previewModeEnabled !== false);
        } catch {
            setPreviewEnabled(true);
        }
    };

    const initializeMockFromRequest = (req: CapturedRequest) => {
        const newMock: MockConfig = {
            name: `${req.method} ${req.path}`,
            method: req.method,
            path: req.path,
            response: {
                body: req.responseBody || { message: 'Mock response' },
            },
            enabled: true,
            status_code: req.statusCode || 200,
        };

        setMock(newMock);
        setOriginalResponse(req.responseBody);
        setEditedResponse(JSON.stringify(newMock.response.body, null, 2));
        setJsonError(null);
    };

    const handleResponseEdit = (value: string) => {
        setEditedResponse(value);
        try {
            const parsed = JSON.parse(value);
            setJsonError(null);
            if (mock) {
                setMock({
                    ...mock,
                    response: {
                        ...mock.response,
                        body: parsed,
                    },
                });
            }
        } catch (error) {
            setJsonError(error instanceof Error ? error.message : 'Invalid JSON');
        }
    };

    const handleStatusCodeChange = (statusCode: number) => {
        if (mock) {
            setMock({
                ...mock,
                status_code: statusCode,
            });
        }
    };

    const handlePreviewToggle = async (enabled: boolean) => {
        setPreviewEnabled(enabled);
        await chrome.storage.local.set({ previewModeEnabled: enabled });

        if (enabled && mock) {
            // Register Service Worker for preview
            try {
                await registerPreviewServiceWorker();
                // Save preview mock
                await previewService.savePreviewMock(mock);
            } catch (error) {
                console.error('Failed to enable preview:', error);
            }
        }
    };

    const registerPreviewServiceWorker = async () => {
        try {
            // Get Service Worker script URL
            const swUrl = chrome.runtime.getURL('src/preview/mock-sw.js');

            // Check if already registered
            const registration = await navigator.serviceWorker.getRegistration(swUrl);
            if (registration) {
                return registration;
            }

            // Register new Service Worker
            const reg = await navigator.serviceWorker.register(swUrl, {
                scope: '/',
            });

            // Wait for activation
            await navigator.serviceWorker.ready;
            return reg;
        } catch (error) {
            console.error('Failed to register preview Service Worker:', error);
            throw error;
        }
    };

    const handleSave = async () => {
        if (!mock) return;

        try {
            // Validate JSON
            JSON.parse(editedResponse);

            // Save preview mock if preview is enabled
            if (previewEnabled) {
                await previewService.savePreviewMock(mock);
            }

            if (onSave) {
                onSave(mock);
            }
        } catch (error) {
            alert('Invalid JSON. Please fix errors before saving.');
        }
    };

    if (!request || !mock) {
        return (
            <div style={{ padding: '20px', textAlign: 'center', color: '#666' }}>
                Select a request to preview
            </div>
        );
    }

    return (
        <div style={{ padding: '20px', fontFamily: 'system-ui' }}>
            <div style={{ marginBottom: '20px' }}>
                <h2>Preview Mock</h2>
                <div style={{ marginBottom: '10px' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                        <input
                            type="checkbox"
                            checked={previewEnabled}
                            onChange={(e) => handlePreviewToggle(e.target.checked)}
                        />
                        <span>Enable Preview Mode (Service Worker)</span>
                    </label>
                    <div style={{ fontSize: '12px', color: '#666', marginTop: '5px' }}>
                        When enabled, preview mocks will be served via Service Worker
                    </div>
                </div>
            </div>

            <div style={{ marginBottom: '20px' }}>
                <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold' }}>
                    Mock Name
                </label>
                <input
                    type="text"
                    value={mock.name}
                    onChange={(e) => setMock({ ...mock, name: e.target.value })}
                    style={{
                        width: '100%',
                        padding: '8px',
                        border: '1px solid #ccc',
                        borderRadius: '4px',
                    }}
                />
            </div>

            <div style={{ marginBottom: '20px' }}>
                <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold' }}>
                    Method & Path
                </label>
                <div style={{ display: 'flex', gap: '10px' }}>
                    <input
                        type="text"
                        value={mock.method}
                        onChange={(e) => setMock({ ...mock, method: e.target.value.toUpperCase() })}
                        style={{
                            width: '100px',
                            padding: '8px',
                            border: '1px solid #ccc',
                            borderRadius: '4px',
                        }}
                    />
                    <input
                        type="text"
                        value={mock.path}
                        onChange={(e) => setMock({ ...mock, path: e.target.value })}
                        style={{
                            flex: 1,
                            padding: '8px',
                            border: '1px solid #ccc',
                            borderRadius: '4px',
                        }}
                    />
                </div>
            </div>

            <div style={{ marginBottom: '20px' }}>
                <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold' }}>
                    Status Code
                </label>
                <input
                    type="number"
                    value={mock.status_code || 200}
                    onChange={(e) => handleStatusCodeChange(parseInt(e.target.value, 10))}
                    style={{
                        width: '100px',
                        padding: '8px',
                        border: '1px solid #ccc',
                        borderRadius: '4px',
                    }}
                />
            </div>

            <div style={{ marginBottom: '20px' }}>
                <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold' }}>
                    Response Body (JSON)
                </label>
                <textarea
                    value={editedResponse}
                    onChange={(e) => handleResponseEdit(e.target.value)}
                    style={{
                        width: '100%',
                        minHeight: '300px',
                        padding: '10px',
                        border: `1px solid ${jsonError ? '#dc3545' : '#ccc'}`,
                        borderRadius: '4px',
                        fontFamily: 'monospace',
                        fontSize: '12px',
                    }}
                />
                {jsonError && (
                    <div style={{ color: '#dc3545', fontSize: '12px', marginTop: '5px' }}>
                        JSON Error: {jsonError}
                    </div>
                )}
            </div>

            {originalResponse && (
                <div style={{ marginBottom: '20px' }}>
                    <details>
                        <summary style={{ cursor: 'pointer', fontWeight: 'bold', marginBottom: '10px' }}>
                            Original Response (for comparison)
                        </summary>
                        <pre
                            style={{
                                padding: '10px',
                                backgroundColor: '#f5f5f5',
                                borderRadius: '4px',
                                overflow: 'auto',
                                fontSize: '12px',
                            }}
                        >
                            {JSON.stringify(originalResponse, null, 2)}
                        </pre>
                    </details>
                </div>
            )}

            <div style={{ display: 'flex', gap: '10px' }}>
                <button
                    onClick={handleSave}
                    disabled={!!jsonError}
                    style={{
                        padding: '10px 20px',
                        backgroundColor: jsonError ? '#ccc' : '#007bff',
                        color: 'white',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: jsonError ? 'not-allowed' : 'pointer',
                    }}
                >
                    {existingMock ? 'Update Mock' : previewEnabled ? 'Save Preview Mock' : 'Create Mock in MockForge'}
                </button>
                {onCancel && (
                    <button
                        onClick={onCancel}
                        style={{
                            padding: '10px 20px',
                            backgroundColor: '#6c757d',
                            color: 'white',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer',
                        }}
                    >
                        Cancel
                    </button>
                )}
            </div>
        </div>
    );
}
