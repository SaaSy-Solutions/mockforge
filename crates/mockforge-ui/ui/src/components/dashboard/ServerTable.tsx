import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { cn } from '../../utils/cn';
import { Server, Globe, Zap, Database, RefreshCw, AlertTriangle } from 'lucide-react';
import { ModernCard, ModernBadge, EmptyState, Alert } from '../ui/DesignSystem';
import { useDashboard } from '../../hooks/useApi';
import { useErrorToast } from '../../components/ui/ToastProvider';
import { usePreferencesStore } from '../../stores/usePreferencesStore';
import { SkeletonCard } from '../ui/Skeleton';

interface ServerInstance {
  server_type: string;
  address?: string;
  running: boolean;
  uptime_seconds?: number;
  total_requests: number;
  active_connections: number;
}

function formatUptime(seconds?: number): string {
  if (!seconds) return 'N/A';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

function formatAddress(address?: string): { display: string, ip: string, port: string } {
  if (!address) return { display: 'Not configured', ip: 'N/A', port: 'N/A' };

  try {
    // Handle URLs like http://127.0.0.1:9080
    if (address.startsWith('http')) {
      const url = new URL(address);
      return {
        display: `${url.hostname}:${url.port}`,
        ip: url.hostname,
        port: url.port
      };
    }

    // Handle raw addresses like 127.0.0.1:9080 or [::1]:9080
    const parts = address.split(':');
    if (parts.length === 2) {
      const [ip, port] = parts;
      return {
        display: `${ip}:${port}`,
        ip: ip.replace(/^\[|\]$/g, ''), // Remove IPv6 brackets
        port
      };
    }

    // Handle IPv6 addresses like [::1]:9080
    if (address.startsWith('[') && address.includes(']:')) {
      const match = address.match(/\[([^\]]+)\]:(\d+)/);
      if (match) {
        return {
          display: `${match[1]}:${match[2]}`,
          ip: match[1],
          port: match[2]
        };
      }
    }

    // Fallback
    return { display: address, ip: 'Unknown', port: 'Unknown' };
  } catch {
    return { display: address, ip: 'Unknown', port: 'Unknown' };
  }
}

function getServerStatusInfo(server: { address?: string; running: boolean }) {

  // Server might be running but not detected if address is not configured
  if (!server.address) {
    return {
      status: 'unknown',
      message: 'Address not configured - server may be running',
      badgeVariant: 'warning' as const,
      showWarning: true
    };
  }

  if (server.running) {
    return {
      status: 'running',
      message: 'Server is active and responding',
      badgeVariant: 'success' as const,
      showWarning: false
    };
  }

  // Server address is configured but running is false - this is the issue the user mentioned
  return {
    status: 'stopped',
    message: 'Server configured but not detected as running',
    badgeVariant: 'error' as const,
    showWarning: true
  };
}

export function ServerTable() {
  const { preferences, updateUI } = usePreferencesStore();
  const density = preferences.ui.serverTableDensity;
  const showErrorToast = useErrorToast();
  const [isRefreshing, setIsRefreshing] = useState(false);

  const { data: dashboard, isLoading, error, refetch } = useDashboard();

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await refetch();
    } catch {
      showErrorToast(
        'Failed to Refresh Server Data',
        'Unable to refresh server status information.'
      );
    } finally {
      setIsRefreshing(false);
    }
  };

  // Show error toast when there's an error
  React.useEffect(() => {
    if (error) {
      showErrorToast(
        'Failed to Load Server Data',
        'Unable to fetch server status information. Please check your connection.'
      );
    }
  }, [error, showErrorToast]);

  if (isLoading) {
    return (
      <ModernCard
        title="Server Instances"
        subtitle="Running MockForge services"
        icon={<Server className="h-6 w-6" />}
      >
        <div className="space-y-4">
          {[...Array(3)].map((_, i) => (
            <SkeletonCard key={i} className="h-24" />
          ))}
        </div>
      </ModernCard>
    );
  }

  if (error) {
    return (
      <ModernCard
        title="Server Instances"
        subtitle="Running MockForge services"
        icon={<Server className="h-6 w-6" />}
      >
        <Alert
          type="error"
          title="Failed to load server data"
          message={error instanceof Error ? error.message : 'Unable to fetch server information. Please check your connection.'}
        />
      </ModernCard>
    );
  }

  const servers = dashboard?.servers || [];

  if (servers.length === 0) {
    return (
      <ModernCard
        title="Server Instances"
        subtitle="Running MockForge services"
        icon={<Server className="h-6 w-6" />}
      >
        <EmptyState
          icon={<Server className="h-12 w-12" />}
          title="No servers running"
          description="No MockForge server instances are currently active. Start a server to see status information here."
        />
      </ModernCard>
    );
  }

  const renderComfortableView = () => (
    <div className="space-y-4">
      {servers.map((server: ServerInstance) => {
        const addrInfo = formatAddress(server.address);
        const statusInfo = getServerStatusInfo(server);
        return (
          <div
            key={server.server_type}
            className="flex items-center justify-between p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
          >
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-xl bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400">
                {server.server_type === 'HTTP' && <Globe className="h-5 w-5" />}
                {server.server_type === 'WebSocket' && <Zap className="h-5 w-5" />}
                {server.server_type === 'gRPC' && <Database className="h-5 w-5" />}
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <h3 className="font-semibold text-gray-900 dark:text-gray-100">
                    {server.server_type}
                  </h3>
                  <ModernBadge
                    variant={statusInfo.badgeVariant}
                    size="sm"
                  >
                    {statusInfo.status === 'running' ? 'Running' :
                     statusInfo.status === 'stopped' ? 'Stopped' : 'Unknown'}
                  </ModernBadge>
                  {statusInfo.showWarning && (
                    <AlertTriangle className="h-4 w-4 text-amber-500" />
                  )}
                </div>
                <div className="text-sm text-gray-600 dark:text-gray-400 mt-1 space-y-1">
                  <div className="flex items-center gap-4">
                    <span className="inline-flex items-center gap-1">
                      <span className="text-xs font-medium text-gray-500 dark:text-gray-400">IP:</span>
                      <span className="font-mono text-xs bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded">
                        {addrInfo.ip}
                      </span>
                    </span>
                    <span className="inline-flex items-center gap-1">
                      <span className="text-xs font-medium text-gray-500 dark:text-gray-400">Port:</span>
                      <span className="font-mono text-xs bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded">
                        {addrInfo.port}
                      </span>
                    </span>
                  </div>
                  {statusInfo.showWarning && (
                    <div className="text-xs text-amber-600 dark:text-amber-400 flex items-center gap-1">
                      <AlertTriangle className="h-3 w-3" />
                      {statusInfo.message}
                    </div>
                  )}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-6 text-sm">
              <div className="text-center">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  {formatUptime(server.uptime_seconds)}
                </div>
                <div className="text-gray-500 dark:text-gray-400">Uptime</div>
              </div>

              <div className="text-center">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  {server.total_requests.toLocaleString()}
                </div>
                <div className="text-gray-500 dark:text-gray-400">Requests</div>
              </div>

              <div className="text-center">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  {server.active_connections}
                </div>
                <div className="text-gray-500 dark:text-gray-400">Active</div>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );

  const renderCompactView = () => (
    <div className="space-y-2">
      {servers.map((server: ServerInstance) => {
        const addrInfo = formatAddress(server.address);
        const statusInfo = getServerStatusInfo(server);
        return (
          <div
            key={server.server_type}
            className="flex items-center justify-between p-3 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
          >
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400">
                {server.server_type === 'HTTP' && <Globe className="h-4 w-4" />}
                {server.server_type === 'WebSocket' && <Zap className="h-4 w-4" />}
                {server.server_type === 'gRPC' && <Database className="h-4 w-4" />}
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <h3 className="font-medium text-gray-900 dark:text-gray-100 text-sm">
                    {server.server_type}
                  </h3>
                  <ModernBadge
                    variant={statusInfo.badgeVariant}
                    size="sm"
                  >
                    {statusInfo.status === 'running' ? 'Running' :
                     statusInfo.status === 'stopped' ? 'Stopped' : 'Unknown'}
                  </ModernBadge>
                  {statusInfo.showWarning && (
                    <AlertTriangle className="h-3 w-3 text-amber-500" />
                  )}
                </div>
                <div className="text-xs text-gray-600 dark:text-gray-400 mt-1">
                  <div className="flex items-center gap-2">
                    <span className="font-mono bg-gray-100 dark:bg-gray-800 px-1.5 py-0.5 rounded text-xs">
                      {addrInfo.display}
                    </span>
                    {statusInfo.showWarning && (
                      <AlertTriangle className="h-3 w-3 text-amber-500" />
                    )}
                  </div>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-3 text-xs">
              <div className="text-center">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  {formatUptime(server.uptime_seconds)}
                </div>
                <div className="text-gray-500 dark:text-gray-400">Uptime</div>
              </div>

              <div className="text-center">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  {server.total_requests.toLocaleString()}
                </div>
                <div className="text-gray-500 dark:text-gray-400">Req</div>
              </div>

              <div className="text-center">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  {server.active_connections}
                </div>
                <div className="text-gray-500 dark:text-gray-400">Active</div>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );

  return (
    <ModernCard
      title="Server Instances"
      subtitle={`${servers.length} service${servers.length === 1 ? '' : 's'} running`}
      icon={<Server className="h-6 w-6" />}
      action={
        <div className="flex items-center gap-4">
          <button
            onClick={handleRefresh}
            disabled={isRefreshing || isLoading}
            className={cn(
              'flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md transition-colors',
              'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100',
              'hover:bg-gray-100 dark:hover:bg-gray-800',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            title="Refresh server status"
          >
            <RefreshCw className={cn('h-3.5 w-3.5', isRefreshing && 'animate-spin')} />
            Refresh
          </button>

          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500 dark:text-gray-400">View</span>
            <div className="inline-flex rounded-lg border border-gray-200 dark:border-gray-700 p-0.5 bg-gray-50 dark:bg-gray-800">
              <button
                className={cn(
                  'text-xs h-7 px-3 rounded-md transition-colors',
                  density === 'comfortable'
                    ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100'
                )}
                onClick={() => updateUI({ serverTableDensity: 'comfortable' })}
              >
                Comfortable
              </button>
              <button
                className={cn(
                  'text-xs h-7 px-3 rounded-md transition-colors',
                  density === 'compact'
                    ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100'
                )}
                onClick={() => updateUI({ serverTableDensity: 'compact' })}
              >
                Compact
              </button>
            </div>
          </div>
        </div>
      }
    >
      {density === 'comfortable' ? renderComfortableView() : renderCompactView()}
    </ModernCard>
  );
}

