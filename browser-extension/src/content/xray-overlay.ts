//! X-Ray overlay for displaying MockForge state in the browser
//!
//! This content script creates an overlay that displays current scenario, persona,
//! reality level, and active chaos rules from response headers.

interface XRayState {
    workspace?: string;
    scenario?: string;
    persona?: string;
    realityLevel?: string;
    realityRatio?: string;
    chaosRules?: string[];
    requestId?: string;
}

class XRayOverlay {
    private overlay: HTMLElement | null = null;
    private state: XRayState = {};
    private apiBaseUrl: string;
    private pollInterval: number = 2000; // Poll every 2 seconds
    private pollTimer: number | null = null;

    constructor(apiBaseUrl: string = 'http://localhost:3000') {
        this.apiBaseUrl = apiBaseUrl;
        this.init();
    }

    private init() {
        // Create overlay element
        this.createOverlay();
        
        // Listen for response headers from fetch/XHR
        this.interceptFetch();
        this.interceptXHR();
        
        // Start polling for state updates
        this.startPolling();
        
        // Listen for messages from background script
        chrome.runtime.onMessage.addListener((message, _sender, _sendResponse) => {
            if (message.type === 'XRAY_STATE_UPDATE') {
                this.updateState(message.state);
            }
        });
    }

    private createOverlay() {
        // Remove existing overlay if present
        const existing = document.getElementById('mockforge-xray-overlay');
        if (existing) {
            existing.remove();
        }

        // Create overlay container
        this.overlay = document.createElement('div');
        this.overlay.id = 'mockforge-xray-overlay';
        this.overlay.style.cssText = `
            position: fixed;
            top: 10px;
            right: 10px;
            width: 300px;
            background: rgba(0, 0, 0, 0.9);
            color: #fff;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 12px;
            padding: 12px;
            border-radius: 8px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
            z-index: 999999;
            pointer-events: auto;
            border: 1px solid rgba(255, 255, 255, 0.1);
        `;

        // Create header
        const header = document.createElement('div');
        header.style.cssText = `
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 10px;
            padding-bottom: 8px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.2);
        `;
        
        const title = document.createElement('div');
        title.textContent = '🔍 MockForge X-Ray';
        title.style.cssText = 'font-weight: 600; font-size: 14px;';
        
        const toggle = document.createElement('button');
        toggle.textContent = '−';
        toggle.style.cssText = `
            background: transparent;
            border: none;
            color: #fff;
            cursor: pointer;
            font-size: 18px;
            padding: 0;
            width: 20px;
            height: 20px;
        `;
        toggle.addEventListener('click', () => this.toggleCollapse());
        
        header.appendChild(title);
        header.appendChild(toggle);
        this.overlay.appendChild(header);

        // Create content container
        const content = document.createElement('div');
        content.id = 'mockforge-xray-content';
        content.style.cssText = 'display: block;';
        this.overlay.appendChild(content);

        // Append to document
        document.body.appendChild(this.overlay);
        
        // Initial render
        this.render();
    }

    private toggleCollapse() {
        const content = document.getElementById('mockforge-xray-content');
        if (content) {
            const isCollapsed = content.style.display === 'none';
            content.style.display = isCollapsed ? 'block' : 'none';
            const toggle = this.overlay?.querySelector('button');
            if (toggle) {
                toggle.textContent = isCollapsed ? '−' : '+';
            }
        }
    }

    private render() {
        const content = document.getElementById('mockforge-xray-content');
        if (!content) return;

        const items: Array<{ label: string; value: string | undefined; color?: string }> = [
            { label: 'Workspace', value: this.state.workspace, color: '#4A9EFF' },
            { label: 'Scenario', value: this.state.scenario, color: '#9B59B6' },
            { label: 'Persona', value: this.state.persona, color: '#E74C3C' },
            { label: 'Reality Level', value: this.state.realityLevel, color: '#F39C12' },
            { label: 'Reality Ratio', value: this.state.realityRatio, color: '#F39C12' },
        ];

        content.innerHTML = '';

        items.forEach(item => {
            if (!item.value) return;

            const row = document.createElement('div');
            row.style.cssText = `
                margin-bottom: 8px;
                padding: 6px;
                background: rgba(255, 255, 255, 0.05);
                border-radius: 4px;
            `;

            const label = document.createElement('div');
            label.textContent = item.label;
            label.style.cssText = `
                font-size: 10px;
                color: rgba(255, 255, 255, 0.6);
                margin-bottom: 2px;
            `;

            const value = document.createElement('div');
            value.textContent = item.value;
            value.style.cssText = `
                font-size: 12px;
                color: ${item.color || '#fff'};
                font-weight: 500;
            `;

            row.appendChild(label);
            row.appendChild(value);
            content.appendChild(row);
        });

        // Chaos rules
        if (this.state.chaosRules && this.state.chaosRules.length > 0) {
            const chaosRow = document.createElement('div');
            chaosRow.style.cssText = `
                margin-top: 8px;
                padding: 6px;
                background: rgba(231, 76, 60, 0.2);
                border-radius: 4px;
                border-left: 3px solid #E74C3C;
            `;

            const chaosLabel = document.createElement('div');
            chaosLabel.textContent = 'Active Chaos Rules';
            chaosLabel.style.cssText = `
                font-size: 10px;
                color: rgba(255, 255, 255, 0.6);
                margin-bottom: 4px;
            `;

            const chaosList = document.createElement('div');
            this.state.chaosRules.forEach(rule => {
                const ruleItem = document.createElement('div');
                ruleItem.textContent = `• ${rule}`;
                ruleItem.style.cssText = `
                    font-size: 11px;
                    color: #E74C3C;
                    margin-left: 8px;
                `;
                chaosList.appendChild(ruleItem);
            });

            chaosRow.appendChild(chaosLabel);
            chaosRow.appendChild(chaosList);
            content.appendChild(chaosRow);
        }

        // Request ID (small, at bottom)
        if (this.state.requestId) {
            const requestIdRow = document.createElement('div');
            requestIdRow.style.cssText = `
                margin-top: 8px;
                padding-top: 8px;
                border-top: 1px solid rgba(255, 255, 255, 0.1);
                font-size: 9px;
                color: rgba(255, 255, 255, 0.4);
            `;
            requestIdRow.textContent = `Request ID: ${this.state.requestId.substring(0, 8)}...`;
            content.appendChild(requestIdRow);
        }
    }

    private updateState(newState: Partial<XRayState>) {
        this.state = { ...this.state, ...newState };
        this.render();
    }

    private extractStateFromHeaders(headers: Headers): Partial<XRayState> {
        const state: Partial<XRayState> = {};
        
        const workspace = headers.get('X-MockForge-Workspace');
        if (workspace) state.workspace = workspace;

        const scenario = headers.get('X-MockForge-Scenario');
        if (scenario) state.scenario = scenario;

        const persona = headers.get('X-MockForge-Persona');
        if (persona) state.persona = persona;

        const realityLevel = headers.get('X-MockForge-Reality-Level');
        if (realityLevel) state.realityLevel = realityLevel;

        const realityRatio = headers.get('X-MockForge-Reality-Ratio');
        if (realityRatio) state.realityRatio = realityRatio;

        const chaosRules = headers.get('X-MockForge-Chaos-Rules');
        if (chaosRules) {
            state.chaosRules = chaosRules.split(',').map(r => r.trim());
        }

        const requestId = headers.get('X-MockForge-Request-ID');
        if (requestId) state.requestId = requestId;

        return state;
    }

    private interceptFetch() {
        const originalFetch = window.fetch;
        window.fetch = async (...args) => {
            const response = await originalFetch(...args);
            
            // Extract state from response headers
            const state = this.extractStateFromHeaders(response.headers);
            if (Object.keys(state).length > 0) {
                this.updateState(state);
            }
            
            return response;
        };
    }

    private interceptXHR() {
        const originalOpen = XMLHttpRequest.prototype.open;
        const originalSend = XMLHttpRequest.prototype.send;

        XMLHttpRequest.prototype.open = function(...args) {
            this._mockforgeXRayUrl = args[1] as string;
            return originalOpen.apply(this, args as any);
        };

        XMLHttpRequest.prototype.send = function(...args) {
            this.addEventListener('load', function() {
                const headers = new Headers();
                const headerString = this.getAllResponseHeaders();
                if (headerString) {
                    headerString.split('\r\n').forEach(line => {
                        const parts = line.split(': ');
                        if (parts.length === 2) {
                            headers.set(parts[0], parts[1]);
                        }
                    });
                }

                // Extract state from response headers
                const xray = (window as any).__mockforgeXRay;
                if (xray) {
                    const state = xray.extractStateFromHeaders(headers);
                    if (Object.keys(state).length > 0) {
                        xray.updateState(state);
                    }
                }
            });

            return originalSend.apply(this, args as any);
        };
    }

    private async fetchStateFromAPI() {
        try {
            // Try to get state from API
            const response = await fetch(`${this.apiBaseUrl}/api/v1/xray/state/summary?workspace=default`);
            if (response.ok) {
                const data = await response.json();
                this.updateState({
                    workspace: data.workspace_id,
                    scenario: data.scenario,
                    persona: data.persona?.id,
                    realityLevel: data.reality_level_name || data.reality_level?.toString(),
                    realityRatio: data.reality_ratio?.toString(),
                    chaosRules: data.chaos_rules || [],
                });
            }
        } catch (error) {
            // Silently fail - API might not be available
            console.debug('MockForge X-Ray: Could not fetch state from API', error);
        }
    }

    private startPolling() {
        // Initial fetch
        this.fetchStateFromAPI();
        
        // Poll periodically
        this.pollTimer = window.setInterval(() => {
            this.fetchStateFromAPI();
        }, this.pollInterval);
    }

    public destroy() {
        if (this.pollTimer !== null) {
            clearInterval(this.pollTimer);
        }
        if (this.overlay) {
            this.overlay.remove();
        }
    }
}

// Initialize X-Ray overlay when content script loads
if (typeof window !== 'undefined') {
    // Get API base URL from storage or use default
    chrome.storage.sync.get(['mockforgeApiUrl'], (result) => {
        const apiUrl = result.mockforgeApiUrl || 'http://localhost:3000';
        const xray = new XRayOverlay(apiUrl);
        (window as any).__mockforgeXRay = xray;
    });
}

