L// MockForge Admin Client
// Simple client-side dashboard for monitoring system metrics

let isFetching = false;

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', function() {
    console.log('MockForge Admin Client loaded');
    refreshData();
    // Auto-refresh every 30 seconds
    setInterval(refreshData, 30000);
});

// Format uptime duration
function formatUptime(seconds) {
    if (!seconds) return '0h 0m';

    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);

    if (hours > 0) {
        return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
}

// Format bytes to human readable
function formatBytes(bytes) {
    if (bytes === 0) return '0 MB';
    return Math.round(bytes / 1024 / 1024) + ' MB';
}

// Refresh all dashboard data
async function refreshData() {
    if (isFetching) return;

    isFetching = true;
    console.log('Refreshing dashboard data...');

    try {
        // Fetch dashboard data from API
        const response = await fetch('/__mockforge/dashboard');
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();
        console.log('Dashboard data received:', data);

        if (data.success && data.data) {
            updateDashboard(data.data);
        } else {
            console.error('API returned error:', data.error);
            updateDashboardWithError(data.error || 'Failed to load data');
        }

    } catch (error) {
        console.error('Error fetching dashboard data:', error);
        updateDashboardWithError(error.message);
    } finally {
        isFetching = false;
    }
}

// Update dashboard with fetched data
function updateDashboard(data) {
    if (!data.system) {
        updateDashboardWithError('No system data available');
        return;
    }

    const system = data.system;

    // Update metrics
    document.getElementById('uptime').textContent = formatUptime(system.uptime_seconds);
    document.getElementById('cpu').textContent = `${system.cpu_usage_percent.toFixed(1)}%`;
    document.getElementById('memory').textContent = formatBytes(system.memory_usage_mb * 1024 * 1024);
    document.getElementById('requests').textContent = data.servers ?
        data.servers.reduce((total, server) => total + (server.total_requests || 0), 0).toLocaleString() :
        '0';

    // Update servers list
    updateServersList(data.servers || []);

    // Update logs
    updateRequestLogs(data.recent_logs || []);
}

// Update with error state
function updateDashboardWithError(error) {
    document.getElementById('uptime').textContent = 'Error';
    document.getElementById('cpu').textContent = '0%';
    document.getElementById('memory').textContent = '0 MB';
    document.getElementById('requests').textContent = '0';

    document.getElementById('servers-list').innerHTML = `
        <div class="server-item">
            <span><span class="status-indicator status-offline"></span>Error loading server status</span>
            <span>${error}</span>
        </div>
    `;

    document.getElementById('logs-container').innerHTML = `
        <div style="padding: 20px; background: #f8d7da; color: #721c24; border-radius: 5px; margin: 10px 0;">
            Error loading request logs: ${error}
        </div>
    `;
}

// Update servers list display
function updateServersList(servers) {
    const container = document.getElementById('servers-list');

    if (servers.length === 0) {
        container.innerHTML = '<div class="server-item">No servers configured</div>';
        return;
    }

    const html = servers.map(server => {
        const isRunning = server.running;
        const statusClass = isRunning ? 'status-online' : 'status-offline';
        const statusText = isRunning ? 'Online' : 'Offline';

        return `
            <div class="server-item">
                <span>
                    <span class="status-indicator ${statusClass}"></span>
                    ${server.server_type || 'Unknown'} Server
                </span>
                <span>
                    ${server.address || 'Not configured'} |
                    Status: ${statusText} |
                    Requests: ${server.total_requests?.toLocaleString() || '0'}
                </span>
            </div>
        `;
    }).join('');

    container.innerHTML = html;
}

// Update request logs display
function updateRequestLogs(logs) {
    const container = document.getElementById('logs-container');

    if (logs.length === 0) {
        container.innerHTML = '<div style="padding: 20px; color: #6c757d;">No request logs available</div>';
        return;
    }

    // Show only the most recent 10 logs
    const recentLogs = logs.slice(0, 10);

    const html = recentLogs.map(log => {
        const timeStr = log.timestamp ? new Date(log.timestamp).toLocaleTimeString() : 'Unknown';
        const methodColor = getMethodColor(log.method);
        const statusColor = getStatusColor(log.status_code);

        return `
            <div style="padding: 10px; border-bottom: 1px solid #e9ecef; font-family: monospace; font-size: 12px;">
                <div style="margin-bottom: 5px;">
                    <span style="background: ${methodColor}; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;">
                        ${log.method || 'GET'}
                    </span>
                    <span style="margin-left: 10px; color: #007bff;">${log.path || '/'}</span>
                    <span style="float: right; background: ${statusColor}; color: white; padding: 2px 4px; border-radius: 3px;">
                        ${log.status_code || '200'}
                    </span>
                </div>
                <div style="color: #6c757d;">
                    <span>${timeStr}</span>
                    <span style="margin-left: 20px;">${log.response_time_ms || 0}ms</span>
                    <span style="margin-left: 20px;">${formatBytes(log.response_size_bytes || 0)}</span>
                </div>
            </div>
        `;
    }).join('');

    container.innerHTML = html;
}

// Get color for HTTP method
function getMethodColor(method) {
    const colors = {
        'GET': '#28a745',
        'POST': '#007bff',
        'PUT': '#ffc107',
        'DELETE': '#dc3545',
        'PATCH': '#6f42c1',
        'OPTIONS': '#17a2b8',
        'HEAD': '#6c757d'
    };
    return colors[method] || '#6c757d';
}

// Get color for HTTP status code
function getStatusColor(statusCode) {
    if (statusCode >= 200 && statusCode < 300) return '#28a745';
    if (statusCode >= 300 && statusCode < 400) return '#17a2b8';
    if (statusCode >= 400 && statusCode < 500) return '#ffc107';
    if (statusCode >= 500) return '#dc3545';
    return '#6c757d';
}

// Global error handler
window.addEventListener('unhandledrejection', function(event) {
    console.error('Unhandled error:', event.reason);
});

// Make functions globally available
window.refreshData = refreshData;
