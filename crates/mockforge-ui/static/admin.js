// MockForge Admin UI JavaScript

class MockForgeAdmin {
    constructor() {
        this.currentTab = 'dashboard';
        this.init();
    }

    init() {
        this.bindEvents();
        this.loadDashboard();
    }

    bindEvents() {
        // Tab switching
        document.querySelectorAll('.nav-tab').forEach(tab => {
            tab.addEventListener('click', (e) => {
                this.switchTab(e.target.dataset.tab);
            });
        });

        // Refresh button
        document.getElementById('refresh-btn').addEventListener('click', () => {
            this.loadDashboard();
        });

        // Configuration forms
        document.getElementById('latency-form')?.addEventListener('submit', (e) => {
            e.preventDefault();
            this.updateLatency(new FormData(e.target));
        });

        document.getElementById('fault-form')?.addEventListener('submit', (e) => {
            e.preventDefault();
            this.updateFaults(new FormData(e.target));
        });

        document.getElementById('proxy-form')?.addEventListener('submit', (e) => {
            e.preventDefault();
            this.updateProxy(new FormData(e.target));
        });

        document.getElementById('validation-form')?.addEventListener('submit', (e) => {
            e.preventDefault();
            this.updateValidation(new FormData(e.target));
        });

        // Fixtures controls
        document.getElementById('refresh-fixtures-btn')?.addEventListener('click', () => {
            this.loadFixtures();
        });

        // Routes controls
        document.getElementById('refresh-routes-btn')?.addEventListener('click', () => this.loadRoutes());
        document.getElementById('routes-filter')?.addEventListener('input', () => this.renderRoutes());
        document.getElementById('routes-only-overrides')?.addEventListener('change', () => this.renderRoutes());
        document.getElementById('routes-sort')?.addEventListener('change', () => this.renderRoutes());
    }

    switchTab(tabName) {
        // Update tab buttons
        document.querySelectorAll('.nav-tab').forEach(tab => {
            tab.classList.remove('active');
        });
        document.querySelector(`[data-tab="${tabName}"]`).classList.add('active');

        // Update tab content
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.remove('active');
        });
        document.getElementById(`${tabName}-tab`).classList.add('active');

        this.currentTab = tabName;
        this.loadTabContent(tabName);
    }

    getBasePath() {
        // Ensure trailing slash for concatenation; derive from current location
        let base = window.location.pathname;
        if (!base.endsWith('/')) base = base + '/';
        return base;
    }

    api(path) {
        const base = this.getBasePath();
        return `${base.replace(/\/+$/, '/')}${path.replace(/^\/+/, '')}`;
    }

    async loadDashboard() {
        try {
            const response = await fetch(this.api('__mockforge/dashboard'));
            const data = await response.json();

            if (data.success) {
                this.updateDashboard(data.data);
            }
        } catch (error) {
            console.error('Error loading dashboard:', error);
        }
    }

    updateDashboard(data) {
        // Update system status
        if (data.system) {
            document.getElementById('uptime').textContent = this.formatDuration(data.system.uptime_seconds || 0);
            document.getElementById('memory').textContent = `${data.system.memory_usage_mb || 0} MB`;
            document.getElementById('cpu').textContent = `${(data.system.cpu_usage_percent || 0).toFixed(1)}%`;
        }

        // Update server statuses
        if (data.servers) {
            this.updateServerStatus('http-status', data.servers.find(s => s.server_type === 'HTTP'));
            this.updateServerStatus('ws-status', data.servers.find(s => s.server_type === 'WebSocket'));
            this.updateServerStatus('grpc-status', data.servers.find(s => s.server_type === 'gRPC'));
        }

        // Update recent requests
        if (data.recent_logs) {
            this.updateRecentRequests(data.recent_logs);
        }
    }

    updateServerStatus(elementId, server) {
        const el = document.getElementById(elementId);
        if (!el) return;

        if (server && server.running) {
            el.className = 'server-status status-running';
            el.textContent = `● Running`;
        } else {
            el.className = 'server-status status-stopped';
            el.textContent = '● Stopped';
        }
    }

    updateRecentRequests(logs) {
        const container = document.getElementById('requests-body');
        if (!container) return;

        if (!logs || logs.length === 0) {
            container.innerHTML = '<div class="loading">No recent requests</div>';
            return;
        }

        container.innerHTML = logs.map(log => `
            <div class="table-row">
                <span>${this.formatTime(log.timestamp)}</span>
                <span>${log.method}</span>
                <span>${log.path}</span>
                <span class="status-${log.status_code}">${log.status_code}</span>
                <span>${log.response_time_ms}ms</span>
            </div>
        `).join('');
    }

    async loadTabContent(tabName) {
        switch (tabName) {
            case 'routes':
                this.loadRoutes();
                break;
            case 'fixtures':
                this.loadFixtures();
                break;
            case 'logs':
                this.loadLogs();
                break;
            case 'metrics':
                this.loadMetrics();
                break;
            case 'config':
                this.loadValidation();
                break;
        }
    }

    async loadRoutes() {
        const container = document.getElementById('routes-list');
        if (!container) return;
        container.innerHTML = '<div class="loading">Loading routes...</div>';
        try {
            const [routesRes, valRes] = await Promise.all([
                fetch(this.api('__mockforge/routes')),
                fetch(this.api('__mockforge/validation')),
            ]);
            const routesJson = await routesRes.json();
            const valJson = await valRes.json();
            this.routesCache = (routesJson && routesJson.routes) || [];
            this.overridesCache = (valJson && valJson.overrides) || {};
            this.renderRoutes();
        } catch (e) {
            container.innerHTML = '<div class="loading">Failed to load routes</div>';
        }
    }

    renderRoutes() {
        const container = document.getElementById('routes-list');
        if (!container) return;
        const q = (document.getElementById('routes-filter')?.value || '').toLowerCase();
        const only = document.getElementById('routes-only-overrides')?.checked;
        const sort = document.getElementById('routes-sort')?.value || 'path';
        let routes = (this.routesCache || []).slice();
        if (q) { routes = routes.filter(r => `${r.method} ${r.path}`.toLowerCase().includes(q)); }
        if (only) { routes = routes.filter(r => !!this.overridesCache?.[`${r.method} ${r.path}`]); }
        routes.sort((a,b) => sort === 'method' ? a.method.localeCompare(b.method) || a.path.localeCompare(b.path) : a.path.localeCompare(b.path) || a.method.localeCompare(b.method));
        container.innerHTML = routes.map(r => this.routeRow(r)).join('');
        container.querySelectorAll('.btn-override').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const row = e.target.closest('[data-route]');
                const method = row.dataset.method;
                const path = row.dataset.path;
                const key = `${method} ${path}`;
                const mode = row.querySelector('.route-mode').value;
                this.renderOverrides(this.collectOverridesFromUI() || {});
                const list = document.getElementById('overrides-list');
                const newRow = this.overrideRow(key, mode);
                const existing = list.querySelector(`[data-key="${key}"]`);
                if (existing) existing.replaceWith(newRow); else list.appendChild(newRow);
            });
        });
        // Prefill per-route mode select based on overrides
        container.querySelectorAll('[data-route]').forEach(row => {
            const key = `${row.dataset.method} ${row.dataset.path}`;
            const sel = row.querySelector('.route-mode');
            const v = this.overridesCache?.[key];
            if (v && sel) sel.value = v;
            if (v) row.style.background = '#f0f9ff';
        });
    }

    routeRow(r) {
        return `
            <div data-route data-method="${r.method}" data-path="${r.path}" style="display:flex; justify-content: space-between; padding:.5rem; border-bottom:1px solid #e2e8f0;">
                <div>
                    <span style=\"font-weight:600; margin-right:1rem;\">${r.method}</span>
                    <span>${r.path}</span>
                </div>
                <div style=\"display:flex; gap:.5rem; align-items:center;\">
                    <select class=\"route-mode\">
                        <option value=\"enforce\">enforce</option>
                        <option value=\"warn\">warn</option>
                        <option value=\"off\">off</option>
                    </select>
                    <button type=\"button\" class=\"btn btn-secondary btn-override\">Add Override</button>
                </div>
            </div>
        `;
    }

    async loadFixtures() {
        try {
            const response = await fetch(this.api('__mockforge/fixtures'));
            const data = await response.json();

            if (data.success && data.data) {
                this.displayFixtures(data.data);
            } else {
                this.displayFixtures([]);
            }
        } catch (error) {
            console.error('Error loading fixtures:', error);
            this.displayFixtures([]);
        }
    }

    displayFixtures(fixtures) {
        const container = document.getElementById('fixtures-table');
        if (!container) return;

        if (!fixtures || fixtures.length === 0) {
            container.innerHTML = '<div style="padding: 2rem; text-align: center; color: #64748b;">No fixtures found</div>';
            return;
        }

        const header = `
            <div class="fixture-header">
                <span>Protocol</span>
                <span>Operation</span>
                <span>Saved At</span>
                <span>Path</span>
            </div>
        `;

        const rows = fixtures.map(fixture => `
            <div class="fixture-row">
                <span class="fixture-protocol">${fixture.protocol || 'N/A'}</span>
                <span class="fixture-operation">${fixture.operation_id || 'N/A'}</span>
                <span class="fixture-saved-at">${this.formatFixtureDate(fixture.saved_at)}</span>
                <span class="fixture-path">${fixture.path || 'N/A'}</span>
            </div>
        `).join('');

        container.innerHTML = header + rows;
    }

    formatFixtureDate(dateString) {
        if (!dateString) return 'N/A';
        try {
            const date = new Date(dateString);
            return date.toLocaleString();
        } catch (e) {
            return dateString;
        }
    }

    async loadLogs() {
        try {
            const response = await fetch(this.api('__mockforge/logs'));
            const data = await response.json();

            if (data.success && data.data) {
                this.updateLogs(data.data);
            }
        } catch (error) {
            console.error('Error loading logs:', error);
        }
    }

    updateLogs(logs) {
        const container = document.getElementById('logs-container');
        if (!container) return;

        container.innerHTML = logs.map(log => `
            <div style="padding: 0.5rem; border-bottom: 1px solid #e2e8f0; font-family: monospace; font-size: 0.8rem;">
                <span style="color: #64748b; margin-right: 1rem;">${this.formatTime(log.timestamp)}</span>
                <span style="font-weight: 600; margin-right: 1rem;">${log.method}</span>
                <span style="margin-right: 1rem;">${log.path}</span>
                <span style="color: ${log.status_code >= 400 ? '#ef4444' : '#10b981'}; font-weight: 600;">${log.status_code}</span>
                <span>${log.response_time_ms}ms</span>
            </div>
        `).join('');
    }

    async loadMetrics() {
        try {
            const response = await fetch(this.api('__mockforge/metrics'));
            const data = await response.json();

            if (data.success && data.data) {
                this.updateMetrics(data.data);
            }
        } catch (error) {
            console.error('Error loading metrics:', error);
        }
    }

    updateMetrics(metrics) {
        const container = document.getElementById('metrics-content');
        if (!container) return;

        container.innerHTML = `
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem;">
                <div style="background: #f8fafc; padding: 1rem; border-radius: 0.25rem;">
                    <div style="font-weight: 500; margin-bottom: 0.5rem;">Total Requests</div>
                    <div style="font-size: 1.5rem; font-weight: 600; color: #2563eb;">
                        ${Object.values(metrics.requests_by_endpoint || {}).reduce((a, b) => a + b, 0)}
                    </div>
                </div>
                <div style="background: #f8fafc; padding: 1rem; border-radius: 0.25rem;">
                    <div style="font-weight: 500; margin-bottom: 0.5rem;">Avg Response Time</div>
                    <div style="font-size: 1.5rem; font-weight: 600; color: #2563eb;">
                        ${metrics.response_time_percentiles?.p50 || 0}ms
                    </div>
                </div>
            </div>
        `;
    }

    async updateLatency(formData) {
        await this.updateConfig('latency', {
            base_ms: parseInt(formData.get('base')),
            jitter_ms: parseInt(formData.get('jitter'))
        });
    }

    async updateFaults(formData) {
        await this.updateConfig('faults', {
            enabled: formData.has('enabled'),
            failure_rate: parseFloat(formData.get('rate'))
        });
    }

    async updateProxy(formData) {
        await this.updateConfig('proxy', {
            enabled: formData.has('enabled'),
            upstream_url: formData.get('url')
        });
    }

    async loadValidation() {
        try {
            const response = await fetch(this.api('__mockforge/validation'));
            const data = await response.json();
            const mode = data.mode || 'enforce';
            document.getElementById('validation-mode').value = mode;
            document.getElementById('aggregate-errors').checked = !!data.aggregate_errors;
            document.getElementById('validate-responses').checked = !!data.validate_responses;
            // Show overrides if present
            if (data.overrides) {
                this.renderOverrides(data.overrides);
            }
        } catch (e) {
            console.warn('Failed to load validation settings');
        }
    }

    async updateValidation(formData) {
        try {
            const payload = {
                mode: formData.get('mode'),
                aggregate_errors: formData.has('aggregate_errors'),
                validate_responses: formData.has('validate_responses'),
                overrides: this.collectOverridesFromUI()
            };
            const response = await fetch(this.api('__mockforge/validation'), {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            if (result && result.status === 'ok') {
                alert('Validation settings updated');
            } else {
                alert('Failed to update validation settings');
            }
        } catch (e) {
            alert('Network error updating validation settings');
        }
    }

    renderOverrides(overrides) {
        // Create simple list UI if not present
        if (!document.getElementById('overrides-container')) {
            const form = document.getElementById('validation-form');
            const cont = document.createElement('div');
            cont.id = 'overrides-container';
            cont.className = 'form-group';
            cont.innerHTML = `
                <h4>Per-Route Overrides</h4>
                <div id="overrides-list"></div>
                <div style="display:flex; gap:.5rem; margin-top:.5rem;">
                    <input id="ov-key" placeholder="METHOD /path" style="flex:2;"/>
                    <select id="ov-mode" style="flex:1;">
                        <option value="enforce">enforce</option>
                        <option value="warn">warn</option>
                        <option value="off">off</option>
                    </select>
                    <button id="ov-add" type="button" class="btn btn-secondary">Add/Update</button>
                </div>
            `;
            form.appendChild(cont);
            document.getElementById('ov-add').addEventListener('click', () => {
                const key = document.getElementById('ov-key').value.trim();
                const mode = document.getElementById('ov-mode').value;
                if (!key) return;
                const row = this.overrideRow(key, mode);
                const list = document.getElementById('overrides-list');
                const existing = list.querySelector(`[data-key="${key}"]`);
                if (existing) existing.replaceWith(row); else list.appendChild(row);
            });
        }

        const list = document.getElementById('overrides-list');
        list.innerHTML = '';
        Object.entries(overrides).forEach(([k, v]) => {
            list.appendChild(this.overrideRow(k, v));
        });
    }

    overrideRow(key, mode) {
        const div = document.createElement('div');
        div.dataset.key = key;
        div.style = 'display:flex; gap:.5rem; align-items:center; margin-top:.25rem;';
        div.innerHTML = `
            <code style="flex:2;">${key}</code>
            <select class="ov-mode" style="flex:1;">
                <option value="enforce" ${mode==='enforce'?'selected':''}>enforce</option>
                <option value="warn" ${mode==='warn'?'selected':''}>warn</option>
                <option value="off" ${mode==='off'?'selected':''}>off</option>
            </select>
            <button type="button" class="btn btn-danger ov-del">Remove</button>
        `;
        div.querySelector('.ov-del').addEventListener('click', () => div.remove());
        return div;
    }

    collectOverridesFromUI() {
        const list = document.getElementById('overrides-list');
        if (!list) return null;
        const out = {};
        list.querySelectorAll('[data-key]').forEach(row => {
            const key = row.dataset.key;
            const mode = row.querySelector('.ov-mode').value;
            out[key] = mode;
        });
        return out;
    }

    async updateConfig(endpoint, data) {
        try {
            const response = await fetch(this.api(`__mockforge/config/${endpoint}`), {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ config_type: endpoint, data })
            });

            const result = await response.json();
            if (result.success) {
                alert('Configuration updated successfully');
            } else {
                alert('Error updating configuration: ' + result.error);
            }
        } catch (error) {
            alert('Network error updating configuration');
        }
    }

    formatDuration(seconds) {
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;

        if (hours > 0) return `${hours}h ${minutes}m ${secs}s`;
        if (minutes > 0) return `${minutes}m ${secs}s`;
        return `${secs}s`;
    }

    formatTime(timestamp) {
        return new Date(timestamp).toLocaleTimeString();
    }
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.mockForgeAdmin = new MockForgeAdmin();
});
